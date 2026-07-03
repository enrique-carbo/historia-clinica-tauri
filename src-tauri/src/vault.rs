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
const VAULT_KDF_SALT: &str = "sal-fija-para-derivar-llave-local-hc-v1";

#[derive(Serialize, Deserialize)]
struct VaultPayload {
    magic: String,
    private_key: String, // Llave privada Ed25519 en Base64
}

/// Inicializa o desbloquea el Vault de un usuario específico.
/// Devuelve: (Llave de cifrado, Llave privada de firma, Llave pública opcional)
pub fn unlock_user_vault(
    app_dir: PathBuf,
    user_id: &str,
    password: &str,
) -> Result<([u8; 32], [u8; 32], Option<String>), String> {
    let vault_path = app_dir.join(format!("vault_{}.bin", user_id));
    let master_key = derive_key_from_password(password)?;

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

                    // Devolvemos None en la llave pública porque ya está guardada en la DB
                    Ok((master_key, priv_key_array, None))
                } else {
                    Err("Contraseña incorrecta para este usuario.".to_string())
                }
            }
            _ => Err("Contraseña incorrecta para este usuario.".to_string()),
        }
    } else {
        // --- CASO B: Primer inicio de sesión. Creamos el vault. ---

        // 1. Generamos el par de llaves aquí (nacen juntas)
        let (signing_key, verifying_key) = crate::crypto::generate_keypair();
        let priv_key_bytes = signing_key.to_bytes();
        let pub_key_hex = hex::encode(verifying_key.to_bytes());

        // 2. Guardamos la llave privada en el Vault (.bin)
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

        // 3. Devolvemos la llave maestra, la llave privada (para RAM) y la pública (para DB)
        Ok((master_key, priv_key_bytes, Some(pub_key_hex)))
    }
}

fn derive_key_from_password(password: &str) -> Result<[u8; 32], String> {
    let salt =
        SaltString::from_b64(VAULT_KDF_SALT).map_err(|e| format!("Error en sal interna: {}", e))?;
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
