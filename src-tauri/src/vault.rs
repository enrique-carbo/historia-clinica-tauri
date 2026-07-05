use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use argon2::{password_hash::SaltString, Argon2};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const VAULT_MAGIC_BYTES: &str = "HISTORIA_CLINICA_VAULT_OK";

#[derive(Serialize, Deserialize)]
struct VaultPayload {
    magic: String,
    private_key: String,
}

/// Lee o genera la sal del KDF para un usuario.
/// La sal se guarda en `vault_{user_id}.salt` (no es secreta, solo única por usuario).
fn get_or_create_kdf_salt(app_dir: &PathBuf, user_id: &str) -> Result<SaltString, String> {
    let salt_path = app_dir.join(format!("vault_{}.salt", user_id));

    if salt_path.exists() {
        let salt_str = fs::read_to_string(&salt_path)
            .map_err(|e| format!("Error leyendo sal del vault: {}", e))?;
        SaltString::from_b64(&salt_str.trim()).map_err(|e| format!("Sal corrupta: {}", e))
    } else {
        let salt = SaltString::generate(&mut OsRng);
        fs::write(&salt_path, salt.as_str())
            .map_err(|e| format!("Error guardando sal del vault: {}", e))?;
        println!("🔧 [Vault] Nueva sal generada para usuario: {}", user_id);
        Ok(salt)
    }
}

/// Inicializa o desbloquea el Vault de un usuario específico.
/// Devuelve: (Llave de cifrado, Llave privada de firma, Llave pública opcional)
pub fn unlock_user_vault(
    app_dir: PathBuf,
    user_id: &str,
    password: &str,
) -> Result<([u8; 32], [u8; 32], Option<String>), String> {
    let vault_path = app_dir.join(format!("vault_{}.bin", user_id));

    let salt = get_or_create_kdf_salt(&app_dir, user_id)?;
    let master_key = derive_key_from_password(password, &salt)?;

    if vault_path.exists() {
        // --- CASO A: El usuario ya existe ---
        let vault_data =
            fs::read(&vault_path).map_err(|e| format!("Error leyendo vault: {}", e))?;

        match decrypt_vault_data(&vault_data, &master_key) {
            Ok(json_bytes) => {
                let payload: VaultPayload = serde_json::from_slice(&json_bytes)
                    .map_err(|_| "Vault corrupto (JSON inválido)".to_string())?;

                if payload.magic == VAULT_MAGIC_BYTES {
                    println!("🔓 [Vault] Desbloqueo exitoso para: {}", user_id);

                    let priv_key_bytes = BASE64
                        .decode(payload.private_key)
                        .map_err(|_| "Llave privada corrupta".to_string())?;

                    let mut priv_key_array = [0u8; 32];
                    priv_key_array.copy_from_slice(&priv_key_bytes);

                    Ok((master_key, priv_key_array, None))
                } else {
                    Err("Contraseña incorrecta para este usuario.".to_string())
                }
            }
            _ => Err("Contraseña incorrecta para este usuario.".to_string()),
        }
    } else {
        // --- CASO B: Primer inicio de sesión. Creamos el vault. ---

        let (signing_key, verifying_key) = crate::crypto::generate_keypair();
        let priv_key_bytes = signing_key.to_bytes();
        let pub_key_hex = hex::encode(verifying_key.to_bytes());

        let payload = VaultPayload {
            magic: VAULT_MAGIC_BYTES.to_string(),
            private_key: BASE64.encode(priv_key_bytes),
        };
        let json_bytes =
            serde_json::to_vec(&payload).map_err(|e| format!("Error serializando vault: {}", e))?;

        let encrypted_data = encrypt_vault_data(&json_bytes, &master_key)?;
        fs::write(&vault_path, encrypted_data)
            .map_err(|e| format!("Error al crear el vault en disco: {}", e))?;

        println!(
            "🔐 [Vault] Nuevo vault creado con llave de firma para: {}",
            user_id
        );

        Ok((master_key, priv_key_bytes, Some(pub_key_hex)))
    }
}

fn derive_key_from_password(password: &str, salt: &SaltString) -> Result<[u8; 32], String> {
    let argon2 = Argon2::default();
    let mut key_bytes = [0u8; 32];
    argon2
        .hash_password_into(
            password.as_bytes(),
            salt.as_str().as_bytes(),
            &mut key_bytes,
        )
        .map_err(|e| format!("Error derivando clave maestra: {}", e))?;
    Ok(key_bytes)
}

fn encrypt_vault_data(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, data)
        .map_err(|e| format!("Error cifrando vault: {:?}", e))?;
    let mut final_data = nonce_bytes.to_vec();
    final_data.extend(ciphertext);
    Ok(final_data)
}

fn decrypt_vault_data(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    if data.len() < 12 {
        return Err("Archivo de vault corrupto (muy corto)".to_string());
    }
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(&data[..12]);
    let ciphertext = &data[12..];
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "Error al descifrar vault".to_string())
}

// =======================================================================
// TESTS
// =======================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn test_password() -> &'static str {
        "MiContraseñaSegura123!"
    }

    fn test_user_id() -> &'static str {
        "test_user_123"
    }

    #[test]
    fn test_vault_create_and_unlock() {
        let temp_dir = TempDir::new().unwrap();
        let user_id = test_user_id();

        // Primer desbloqueo: crea el vault
        let (master_key1, priv_key1, pub_key1) =
            unlock_user_vault(temp_dir.path().to_path_buf(), user_id, test_password()).unwrap();

        assert_eq!(master_key1.len(), 32);
        assert_eq!(priv_key1.len(), 32);
        assert!(pub_key1.is_some());

        // Verificar que se crearon los archivos
        assert!(temp_dir
            .path()
            .join(format!("vault_{}.bin", user_id))
            .exists());
        assert!(temp_dir
            .path()
            .join(format!("vault_{}.salt", user_id))
            .exists());

        // Segundo desbloqueo: abre el vault existente
        let (master_key2, priv_key2, pub_key2) =
            unlock_user_vault(temp_dir.path().to_path_buf(), user_id, test_password()).unwrap();

        // Misma contraseña = misma master key (misma sal)
        assert_eq!(master_key1, master_key2);
        // Misma llave privada
        assert_eq!(priv_key1, priv_key2);
        // No devuelve pub_key en desbloqueo (ya está en DB)
        assert!(pub_key2.is_none());
    }

    #[test]
    fn test_vault_wrong_password_fails() {
        let temp_dir = TempDir::new().unwrap();
        let user_id = test_user_id();

        // Crear vault
        unlock_user_vault(temp_dir.path().to_path_buf(), user_id, test_password()).unwrap();

        // Intentar desbloquear con contraseña incorrecta
        let result = unlock_user_vault(
            temp_dir.path().to_path_buf(),
            user_id,
            "ContraseñaIncorrecta",
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_vault_different_users_different_keys() {
        let temp_dir = TempDir::new().unwrap();
        let password = test_password();

        // Usuario A
        let (key_a, _, _) =
            unlock_user_vault(temp_dir.path().to_path_buf(), "user_a", password).unwrap();

        // Usuario B (misma contraseña, diferente sal)
        let (key_b, _, _) =
            unlock_user_vault(temp_dir.path().to_path_buf(), "user_b", password).unwrap();

        // Misma contraseña, diferentes usuarios = diferentes master keys
        assert_ne!(key_a, key_b);
    }

    #[test]
    fn test_sal_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let user_id = test_user_id();

        // Crear vault (genera sal)
        unlock_user_vault(temp_dir.path().to_path_buf(), user_id, test_password()).unwrap();

        // Leer sal del archivo
        let salt_path = temp_dir.path().join(format!("vault_{}.salt", user_id));
        let salt1 = fs::read_to_string(&salt_path).unwrap();

        // Desbloquear de nuevo (usa sal existente)
        unlock_user_vault(temp_dir.path().to_path_buf(), user_id, test_password()).unwrap();

        let salt2 = fs::read_to_string(&salt_path).unwrap();

        // La sal no cambia entre desbloqueos
        assert_eq!(salt1, salt2);
    }
}
