use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::{Digest, Sha256};

pub const AES_KEY_SIZE: usize = 32;
pub const AES_NONCE_SIZE: usize = 12;
pub const ED25519_SECRET_SIZE: usize = 32;
pub const ED25519_SIGNATURE_SIZE: usize = 64;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct EncryptedData {
    pub ciphertext: String,
    pub nonce: String,
}

pub fn encrypt_text(
    plain_text: &str,
    key_bytes: &[u8; AES_KEY_SIZE],
) -> Result<EncryptedData, String> {
    let key = Key::<Aes256Gcm>::from_slice(key_bytes);
    let cipher = Aes256Gcm::new(key);

    let mut nonce_bytes = [0u8; AES_NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext_bytes = cipher
        .encrypt(nonce, plain_text.as_bytes())
        .map_err(|_| "Error en el proceso de cifrado".to_string())?;

    Ok(EncryptedData {
        ciphertext: hex::encode(ciphertext_bytes),
        nonce: hex::encode(nonce_bytes),
    })
}

pub fn decrypt_text(
    encrypted: &EncryptedData,
    key_bytes: &[u8; AES_KEY_SIZE],
) -> Result<String, String> {
    let key = Key::<Aes256Gcm>::from_slice(key_bytes);
    let cipher = Aes256Gcm::new(key);

    let nonce_bytes = hex::decode(&encrypted.nonce).map_err(|_| "Nonce inválido".to_string())?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext_bytes =
        hex::decode(&encrypted.ciphertext).map_err(|_| "Ciphertext inválido".to_string())?;

    let decrypted_bytes = cipher
        .decrypt(nonce, ciphertext_bytes.as_ref())
        .map_err(|_| "Error de integridad: datos alterados o clave incorrecta".to_string())?;

    String::from_utf8(decrypted_bytes)
        .map_err(|_| "Datos descifrados no son UTF-8 válido".to_string())
}

/// Genera la sal del blind index a partir de la master key (primeros 16 bytes)
/// Esto garantiza que cada usuario tenga una sal única derivada de su contraseña
pub fn get_blind_index_salt(master_key: &[u8; AES_KEY_SIZE]) -> Vec<u8> {
    master_key[..16].to_vec()
}

/// Genera hash determinista para búsqueda sin revelar dato real
pub fn generate_blind_index(identity_doc: &str, salt: &[u8]) -> String {
    let normalized = identity_doc
        .trim()
        .replace("-", "")
        .replace(".", "")
        .replace(" ", "")
        .to_lowercase();

    let mut hasher = Sha256::new();
    hasher.update(salt);
    hasher.update(normalized.as_bytes());

    hex::encode(hasher.finalize())
}

// =======================================================================
// FIRMA DIGITAL Y NO-REPUDIO (Ed25519)
// =======================================================================

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

pub fn generate_keypair() -> (SigningKey, VerifyingKey) {
    let mut secret_bytes = [0u8; ED25519_SECRET_SIZE];
    OsRng.fill_bytes(&mut secret_bytes);

    let signing_key = SigningKey::from_bytes(&secret_bytes);
    let verifying_key = VerifyingKey::from(&signing_key);

    (signing_key, verifying_key)
}

pub fn hash_document(document_text: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(document_text.as_bytes());
    let result = hasher.finalize();
    let mut hash_array = [0u8; 32];
    hash_array.copy_from_slice(&result);
    hash_array
}

pub fn sign_hash(
    hash: &[u8; 32],
    private_key_bytes: &[u8; ED25519_SECRET_SIZE],
) -> Result<String, String> {
    let signing_key = SigningKey::from_bytes(private_key_bytes);
    let signature: Signature = signing_key.sign(hash);
    Ok(hex::encode(signature.to_bytes()))
}

pub fn verify_signature(
    hash: &[u8; 32],
    signature_hex: &str,
    public_key_hex: &str,
) -> Result<bool, String> {
    let public_key_bytes =
        hex::decode(public_key_hex).map_err(|_| "Llave pública inválida".to_string())?;

    let public_key_array: [u8; ED25519_SECRET_SIZE] = public_key_bytes
        .as_slice()
        .try_into()
        .map_err(|_| format!("Llave pública debe tener {} bytes", ED25519_SECRET_SIZE))?;

    let verifying_key = VerifyingKey::from_bytes(&public_key_array)
        .map_err(|e| format!("Error al parsear llave pública: {}", e))?;

    let signature_bytes = hex::decode(signature_hex).map_err(|_| "Firma inválida".to_string())?;

    let signature_array: [u8; ED25519_SIGNATURE_SIZE] = signature_bytes
        .as_slice()
        .try_into()
        .map_err(|_| format!("Firma debe tener {} bytes", ED25519_SIGNATURE_SIZE))?;

    let signature = Signature::from_bytes(&signature_array);

    match verifying_key.verify(hash, &signature) {
        Ok(_) => Ok(true),
        Err(e) => Err(format!("Firma matemáticamente inválida: {}", e)),
    }
}

// =======================================================================
// TESTS
// =======================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn random_key() -> [u8; AES_KEY_SIZE] {
        let mut key = [0u8; AES_KEY_SIZE];
        OsRng.fill_bytes(&mut key);
        key
    }

    fn test_salt() -> Vec<u8> {
        vec![0u8; 32]
    }

    fn test_master_key() -> [u8; AES_KEY_SIZE] {
        let mut key = [0u8; AES_KEY_SIZE];
        for i in 0..AES_KEY_SIZE {
            key[i] = i as u8;
        }
        key
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = random_key();
        let plaintext = "Nota SOAP: Paciente presenta dolor torácico...";
        let encrypted = encrypt_text(plaintext, &key).unwrap();
        let decrypted = decrypt_text(&encrypted, &key).unwrap();
        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_encrypt_unique_nonces() {
        let key = random_key();
        let plaintext = "Texto idéntico";
        let enc1 = encrypt_text(plaintext, &key).unwrap();
        let enc2 = encrypt_text(plaintext, &key).unwrap();
        assert_ne!(enc1.nonce, enc2.nonce);
        assert_ne!(enc1.ciphertext, enc2.ciphertext);
    }

    #[test]
    fn test_decrypt_wrong_key_fails() {
        let key1 = random_key();
        let key2 = random_key();
        let encrypted = encrypt_text("Dato sensible", &key1).unwrap();
        assert!(decrypt_text(&encrypted, &key2).is_err());
    }

    #[test]
    fn test_decrypt_tampered_ciphertext_fails() {
        let key = random_key();
        let mut encrypted = encrypt_text("Dato original", &key).unwrap();
        let mut bytes = hex::decode(&encrypted.ciphertext).unwrap();
        bytes[0] ^= 0xFF;
        encrypted.ciphertext = hex::encode(bytes);
        assert!(decrypt_text(&encrypted, &key).is_err());
    }

    #[test]
    fn test_decrypt_tampered_nonce_fails() {
        let key = random_key();
        let mut encrypted = encrypt_text("Dato original", &key).unwrap();
        let mut bytes = hex::decode(&encrypted.nonce).unwrap();
        bytes[0] ^= 0xFF;
        encrypted.nonce = hex::encode(bytes);
        assert!(decrypt_text(&encrypted, &key).is_err());
    }

    #[test]
    fn test_get_blind_index_salt() {
        let master_key = test_master_key();
        let salt = get_blind_index_salt(&master_key);
        assert_eq!(salt.len(), 16);
        assert_eq!(
            salt,
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
        );
    }

    #[test]
    fn test_blind_index_deterministic() {
        let salt = test_salt();
        let dni = "12345678";
        let hash1 = generate_blind_index(dni, &salt);
        let hash2 = generate_blind_index(dni, &salt);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_blind_index_normalization() {
        let salt = test_salt();
        let hash1 = generate_blind_index("12345678", &salt);
        let hash2 = generate_blind_index("  12.345.678  ", &salt);
        let hash3 = generate_blind_index("12-345-678", &salt);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1, hash3);
    }

    #[test]
    fn test_blind_index_different_inputs() {
        let salt = test_salt();
        let hash1 = generate_blind_index("12345678", &salt);
        let hash2 = generate_blind_index("87654321", &salt);
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_blind_index_different_salts() {
        let salt1 = vec![0u8; 32];
        let salt2 = vec![1u8; 32];
        let hash1 = generate_blind_index("12345678", &salt1);
        let hash2 = generate_blind_index("12345678", &salt2);
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_sign_and_verify() {
        let (signing_key, verifying_key) = generate_keypair();
        let document = "Nota SOAP firmada el 2026-07-05";
        let hash = hash_document(document);
        let signature = sign_hash(&hash, signing_key.as_bytes()).unwrap();
        let result =
            verify_signature(&hash, &signature, &hex::encode(verifying_key.as_bytes())).unwrap();
        assert!(result);
    }

    #[test]
    fn test_verify_wrong_public_key_fails() {
        let (signing_key, _) = generate_keypair();
        let (_, wrong_key) = generate_keypair();
        let hash = hash_document("Nota SOAP");
        let signature = sign_hash(&hash, signing_key.as_bytes()).unwrap();
        let result = verify_signature(&hash, &signature, &hex::encode(wrong_key.as_bytes()));
        assert!(result.is_err() || !result.unwrap());
    }

    #[test]
    fn test_verify_tampered_document_fails() {
        let (signing_key, verifying_key) = generate_keypair();
        let hash = hash_document("Nota SOAP original");
        let signature = sign_hash(&hash, signing_key.as_bytes()).unwrap();
        let tampered_hash = hash_document("Nota SOAP ALTERADA");
        let result = verify_signature(
            &tampered_hash,
            &signature,
            &hex::encode(verifying_key.as_bytes()),
        );
        let is_invalid = match result {
            Ok(valid) => !valid,
            Err(_) => true,
        };
        assert!(is_invalid);
    }

    #[test]
    fn test_verify_tampered_signature_fails() {
        let (signing_key, verifying_key) = generate_keypair();
        let hash = hash_document("Nota SOAP");
        let mut signature = sign_hash(&hash, signing_key.as_bytes()).unwrap();
        let mut bytes = hex::decode(&signature).unwrap();
        bytes[0] ^= 0xFF;
        signature = hex::encode(bytes);
        let result = verify_signature(&hash, &signature, &hex::encode(verifying_key.as_bytes()));
        assert!(result.is_err() || !result.unwrap());
    }
}
