use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use rand::rngs::OsRng;
use rand::RngCore;
use std::path::{Path, PathBuf};

use crate::seed;

/// Error distintivo que el frontend reconoce para pedir la seed al usuario.
pub const SEED_REQUIRED: &str = "SEED_REQUIRED";

/// Formato del wrap: nonce (12 bytes) || ciphertext (32 bytes data_key + 16 tag)
const WRAP_NONCE_SIZE: usize = 12;
const WRAP_OVERHEAD: usize = WRAP_NONCE_SIZE + 16 + 32; // nonce + GCM tag + key

// =======================================================================
// RUTAS
// =======================================================================

fn user_wrap_path(app_dir: &Path, user_id: &str) -> PathBuf {
    app_dir.join(format!(".data_key.{}", user_id))
}

fn master_wrap_path(app_dir: &Path) -> PathBuf {
    app_dir.join(".data_key.master")
}

fn legacy_path(app_dir: &Path) -> PathBuf {
    app_dir.join(".data_key")
}

// =======================================================================
// WRAP / UNWRAP (AES-256-GCM)
// =======================================================================

/// Envuelve (cifra) una data_key con una wrapping key (master_key o seed-derived).
/// Formato: nonce (12) || ciphertext
pub fn wrap_key(data_key: &[u8; 32], wrapping_key: &[u8; 32]) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(wrapping_key));
    let mut nonce_bytes = [0u8; WRAP_NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, data_key.as_ref())
        .map_err(|e| format!("Error envolviendo data_key: {:?}", e))?;

    let mut blob = nonce_bytes.to_vec();
    blob.extend(ciphertext);
    Ok(blob)
}

/// Desenvuelve una data_key.
pub fn unwrap_key(blob: &[u8], wrapping_key: &[u8; 32]) -> Result<[u8; 32], String> {
    if blob.len() < WRAP_OVERHEAD {
        return Err("Wrap de data_key corrupto (muy corto)".to_string());
    }
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(wrapping_key));
    let nonce = Nonce::from_slice(&blob[..WRAP_NONCE_SIZE]);
    let ciphertext = &blob[WRAP_NONCE_SIZE..];

    let plain = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "Data_key corrupta o wrapping key incorrecta".to_string())?;

    let arr: [u8; 32] = plain
        .as_slice()
        .try_into()
        .map_err(|_| "Data_key descifrada inválida".to_string())?;
    Ok(arr)
}

// =======================================================================
// PERSISTENCIA DE WRAPS
// =======================================================================

/// Guarda el wrap personal del usuario (fast path con password).
pub fn save_user_wrap(
    app_dir: &Path,
    user_id: &str,
    data_key: &[u8; 32],
    master_key: &[u8; 32],
) -> Result<(), String> {
    let blob = wrap_key(data_key, master_key)?;
    std::fs::write(user_wrap_path(app_dir, user_id), blob)
        .map_err(|e| format!("Error guardando wrap de usuario: {}", e))
}

/// Guarda el wrap master (bootstrap/recovery con seed, escrita en papel).
pub fn save_master_wrap(
    app_dir: &Path,
    data_key: &[u8; 32],
    seed_phrase: &str,
) -> Result<(), String> {
    let seed_key = seed::derive_key_from_seed(seed_phrase)?;
    let blob = wrap_key(data_key, &seed_key)?;
    std::fs::write(master_wrap_path(app_dir), blob)
        .map_err(|e| format!("Error guardando wrap master: {}", e))
}

/// Migra el legacy `.data_key` en hex plano a un wrap seguro.
/// Devuelve la data_key y borra el archivo legacy.
fn migrate_legacy(app_dir: &Path) -> Result<Option<[u8; 32]>, String> {
    let path = legacy_path(app_dir);
    if !path.exists() {
        return Ok(None);
    }

    let hex_str = std::fs::read_to_string(&path)
        .map_err(|e| format!("Error leyendo data_key legacy: {}", e))?;
    let bytes =
        hex::decode(hex_str.trim()).map_err(|_| "Data_key legacy corrupta".to_string())?;

    let mut key = [0u8; 32];
    if bytes.len() != 32 {
        let _ = std::fs::remove_file(&path);
        return Err("Data_key legacy longitud inválida".to_string());
    }
    key.copy_from_slice(&bytes);

    // Borrar el archivo en claro
    std::fs::remove_file(&path)
        .map_err(|e| format!("Error eliminando data_key legacy: {}", e))?;

    println!("🔄 [DataKey] Legacy .data_key migrada y eliminada del disco");
    Ok(Some(key))
}

// =======================================================================
// RESOLUCIÓN PRINCIPAL
// =======================================================================

/// Resuelve la data_key para el usuario actual, en orden de prioridad:
///
/// 1. **Wrap personal** (`.data_key.{user_id}`) → fast path con password.
/// 2. **Seed + master wrap** → bootstrap de nuevo usuario / recovery.
///    Requiere `seed_phrase` (de papel, no de disco).
/// 3. **Legacy hex** → migración automática.
/// 4. **Nada** → primera instalación: genera data_key nueva.
///
/// Si existe master wrap pero no hay personal ni seed → `Err(SEED_REQUIRED)`.
pub fn resolve_data_key(
    app_dir: &Path,
    user_id: &str,
    master_key: &[u8; 32],
    seed_phrase: Option<&str>,
) -> Result<[u8; 32], String> {
    let user_path = user_wrap_path(app_dir, user_id);

    // 1. Fast path: wrap personal del usuario
    if user_path.exists() {
        let blob = std::fs::read(&user_path)
            .map_err(|e| format!("Error leyendo wrap personal: {}", e))?;
        let data_key = unwrap_key(&blob, master_key)?;
        return Ok(data_key);
    }

    // 2. Bootstrap con seed + master wrap
    let master_path = master_wrap_path(app_dir);
    if master_path.exists() {
        match seed_phrase {
            Some(phrase) => {
                let blob = std::fs::read(&master_path)
                    .map_err(|e| format!("Error leyendo master wrap: {}", e))?;
                let seed_key = seed::derive_key_from_seed(phrase)?;
                let data_key = unwrap_key(&blob, &seed_key)?;
                // Crear wrap personal para próximos logins (sin pedir seed)
                save_user_wrap(app_dir, user_id, &data_key, master_key)?;
                println!("🌱 [DataKey] Bootstrapped desde master wrap para: {}", user_id);
                return Ok(data_key);
            }
            None => {
                return Err(SEED_REQUIRED.to_string());
            }
        }
    }

    // 3. Migración legacy (hex en claro)
    if let Some(data_key) = migrate_legacy(app_dir)? {
        save_user_wrap(app_dir, user_id, &data_key, master_key)?;
        return Ok(data_key);
    }

    // 4. Primera instalación: generar data_key nueva
    let mut data_key = [0u8; 32];
    OsRng.fill_bytes(&mut data_key);
    save_user_wrap(app_dir, user_id, &data_key, master_key)?;
    println!("🔑 [DataKey] Nueva data_key generada y envuelta para: {}", user_id);
    Ok(data_key)
}

// =======================================================================
// TESTS
// =======================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn random_key() -> [u8; 32] {
        let mut k = [0u8; 32];
        OsRng.fill_bytes(&mut k);
        k
    }

    #[test]
    fn test_wrap_unwrap_roundtrip() {
        let data = random_key();
        let wk = random_key();
        let blob = wrap_key(&data, &wk).unwrap();
        let recovered = unwrap_key(&blob, &wk).unwrap();
        assert_eq!(data, recovered);
    }

    #[test]
    fn test_unwrap_wrong_key_fails() {
        let data = random_key();
        let correct_wk = random_key();
        let wrong_wk = random_key();
        let blob = wrap_key(&data, &correct_wk).unwrap();
        assert!(unwrap_key(&blob, &wrong_wk).is_err());
    }

    #[test]
    fn test_unwrap_tampered_fails() {
        let data = random_key();
        let wk = random_key();
        let mut blob = wrap_key(&data, &wk).unwrap();
        blob[WRAP_NONCE_SIZE] ^= 0xFF;
        assert!(unwrap_key(&blob, &wk).is_err());
    }

    #[test]
    fn test_resolve_creates_new_when_empty() {
        let dir = TempDir::new().unwrap();
        let mk = random_key();
        let dk1 = resolve_data_key(dir.path(), "user_a", &mk, None).unwrap();
        // Segunda llamada usa el wrap personal
        let dk2 = resolve_data_key(dir.path(), "user_a", &mk, None).unwrap();
        assert_eq!(dk1, dk2);
        assert!(dir.path().join(".data_key.user_a").exists());
    }

    #[test]
    fn test_resolve_different_users_different_master_keys() {
        let dir = TempDir::new().unwrap();
        let mk_a = random_key();
        let mk_b = random_key();

        let dk_a = resolve_data_key(dir.path(), "user_a", &mk_a, None).unwrap();
        // user_b no tiene wrap personal ni master wrap → genera NUEVA key
        let dk_b = resolve_data_key(dir.path(), "user_b", &mk_b, None).unwrap();

        // Sin master wrap compartido, cada usuario tiene su propia data_key
        // (esto es intencional: sin seed no hay bootstrap multi-usuario)
        assert_ne!(dk_a, dk_b);
    }

    #[test]
    fn test_master_wrap_bootstrap_for_new_user() {
        let dir = TempDir::new().unwrap();
        let mk_a = random_key();
        let mk_b = random_key();
        let phrase = "abaco abrir acero acida actas acuna";

        // user_a crea data_key y comparte via master wrap
        let dk_a = resolve_data_key(dir.path(), "user_a", &mk_a, None).unwrap();
        save_master_wrap(dir.path(), &dk_a, phrase).unwrap();

        // user_b bootstraps con seed (sin wrap personal previo)
        let dk_b = resolve_data_key(dir.path(), "user_b", &mk_b, Some(phrase)).unwrap();
        assert_eq!(dk_a, dk_b);
        // Ahora user_b tiene wrap personal → próxima vez no necesita seed
        let dk_b2 = resolve_data_key(dir.path(), "user_b", &mk_b, None).unwrap();
        assert_eq!(dk_b, dk_b2);
    }

    #[test]
    fn test_seed_required_when_master_exists_without_seed() {
        let dir = TempDir::new().unwrap();
        let mk_a = random_key();
        let mk_b = random_key();
        let phrase = "abaco abrir acero acida actas acuna";

        let dk = resolve_data_key(dir.path(), "user_a", &mk_a, None).unwrap();
        save_master_wrap(dir.path(), &dk, phrase).unwrap();

        // user_b sin personal wrap y sin seed → SEED_REQUIRED
        let err = resolve_data_key(dir.path(), "user_b", &mk_b, None).unwrap_err();
        assert_eq!(err, SEED_REQUIRED);
    }

    #[test]
    fn test_wrong_seed_fails_bootstrap() {
        let dir = TempDir::new().unwrap();
        let mk_a = random_key();
        let mk_b = random_key();

        let dk = resolve_data_key(dir.path(), "user_a", &mk_a, None).unwrap();
        save_master_wrap(dir.path(), &dk, "abaco abrir acero acida actas acuna").unwrap();

        let result = resolve_data_key(
            dir.path(),
            "user_b",
            &mk_b,
            Some("palabra incorrecta otra cosa mas"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_migrate_legacy_hex() {
        let dir = TempDir::new().unwrap();
        let mk = random_key();

        // Crear legacy plaintext
        let legacy_key = random_key();
        std::fs::write(dir.path().join(".data_key"), hex::encode(legacy_key)).unwrap();

        let resolved = resolve_data_key(dir.path(), "user_a", &mk, None).unwrap();
        assert_eq!(resolved, legacy_key);
        // Legacy borrado
        assert!(!dir.path().join(".data_key").exists());
        // Wrap personal creado
        assert!(dir.path().join(".data_key.user_a").exists());
    }

    #[test]
    fn test_save_master_wrap_and_read_back() {
        let dir = TempDir::new().unwrap();
        let dk = random_key();
        let phrase = "abaco abrir acero acida actas acuna";

        save_master_wrap(dir.path(), &dk, phrase).unwrap();
        assert!(dir.path().join(".data_key.master").exists());

        let seed_key = seed::derive_key_from_seed(phrase).unwrap();
        let blob = std::fs::read(dir.path().join(".data_key.master")).unwrap();
        let recovered = unwrap_key(&blob, &seed_key).unwrap();
        assert_eq!(dk, recovered);
    }
}
