use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use argon2::{password_hash::SaltString, Argon2};
use rand::rngs::OsRng;
use rand::RngCore;
use std::fs;
use std::path::PathBuf;

// El texto mágico que nos dirá si la contraseña es correcta
const VAULT_MAGIC_BYTES: &[u8] = b"HISTORIA_CLINICA_VAULT_OK";

// Una sal fija para derivar la llave.
// En un vault local offline, esto es seguro porque la seguridad recae en la complejidad de la contraseña del médico.
const VAULT_KDF_SALT: &str = "sal-fija-para-derivar-llave-local-hc-v1";

/// Inicializa o desbloquea el Vault de un usuario específico.
/// Devuelve la llave maestra de 32 bytes si tiene éxito.
pub fn unlock_user_vault(
    app_dir: PathBuf,
    user_id: &str,
    password: &str,
) -> Result<[u8; 32], String> {
    // 1. Definir la ruta exacta del vault de este médico
    let vault_path = app_dir.join(format!("vault_{}.bin", user_id));

    // 2. Derivar los 32 bytes de la contraseña usando Argon2id
    let master_key = derive_key_from_password(password)?;

    // 3. Verificar o Crear el vault
    if vault_path.exists() {
        // --- CASO A: El usuario ya existe. Verificamos la contraseña. ---
        let vault_data =
            fs::read(&vault_path).map_err(|e| format!("Error leyendo vault: {}", e))?;

        // Intentamos descifrar los datos con la llave derivada
        match decrypt_vault_data(&vault_data, &master_key) {
            Ok(magic) if magic == VAULT_MAGIC_BYTES => {
                println!("🔓 [Vault] Desbloqueo exitoso para: {}", user_id);
                Ok(master_key)
            }
            _ => Err("Contraseña incorrecta para este usuario.".to_string()),
        }
    } else {
        // --- CASO B: Primer inicio de sesión. Creamos el vault. ---
        let encrypted_data = encrypt_vault_data(VAULT_MAGIC_BYTES, &master_key)?;

        fs::write(&vault_path, encrypted_data)
            .map_err(|e| format!("Error al crear el vault en disco: {}", e))?;

        println!("🔐 [Vault] Nuevo vault creado para: {}", user_id);
        Ok(master_key)
    }
}

/// Deriva una clave de 32 bytes usando Argon2id a partir de la contraseña
fn derive_key_from_password(password: &str) -> Result<[u8; 32], String> {
    let salt =
        SaltString::from_b64(VAULT_KDF_SALT).map_err(|e| format!("Error en sal interna: {}", e))?;

    let argon2 = Argon2::default();

    // hash_password_into es el método de Argon2 para sacar bytes crudos (KDF)
    // en lugar de un string formateado (que usamos para el login de users)
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

/// Cifra los bytes mágicos para guardarlos en disco
fn encrypt_vault_data(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));

    // Generamos un Nonce seguro para este archivo
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, data)
        .map_err(|e| format!("Error cifrando vault: {:?}", e))?;

    // Formato del archivo .bin: [12 bytes de Nonce] + [Ciphertext]
    let mut final_data = nonce_bytes.to_vec();
    final_data.extend(ciphertext);

    Ok(final_data)
}

/// Descifra el archivo .bin y devuelve los bytes internos
fn decrypt_vault_data(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    if data.len() < 12 {
        return Err("Archivo de vault corrupto (muy corto)".to_string());
    }

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));

    // Extraer el Nonce (primeros 12 bytes) y el Ciphertext (el resto)
    let nonce = Nonce::from_slice(&data[..12]);
    let ciphertext = &data[12..];

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "Error al descifrar vault (¿Contraseña incorrecta?)".to_string())
}
