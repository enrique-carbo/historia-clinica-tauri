use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use rand::RngCore;
use sha2::{Digest, Sha256};

// Estructura para empaquetar el resultado del cifrado listo para guardar en SQLite
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct EncryptedData {
    pub ciphertext: String, // Datos médicos en formato Hexadecimal
    pub nonce: String,      // El IV/Nonce usado (necesario para desencriptar)
}

/// Cifra un texto plano utilizando una clave de 32 bytes (256 bits) y AES-GCM-256
pub fn encrypt_text(plain_text: &str, key_bytes: &[u8; 32]) -> Result<EncryptedData, String> {
    let key = Key::<Aes256Gcm>::from_slice(key_bytes);
    let cipher = Aes256Gcm::new(key);

    // Generar un Nonce (Vector de Inicialización) aleatorio de 12 bytes único
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Cifrar los datos
    let ciphertext_bytes = cipher
        .encrypt(nonce, plain_text.as_bytes())
        .map_err(|e| format!("Error en el proceso de cifrado: {:?}", e))?;

    // Convertir a Hexadecimal para guardarlo de forma segura como Texto en SQLite/PocketBase
    Ok(EncryptedData {
        ciphertext: hex::encode(ciphertext_bytes),
        nonce: hex::encode(nonce_bytes),
    })
}

/// Descifra un texto cifrado en Hex utilizando su respectivo nonce y la clave de 32 bytes
pub fn decrypt_text(encrypted: &EncryptedData, key_bytes: &[u8; 32]) -> Result<String, String> {
    let key = Key::<Aes256Gcm>::from_slice(key_bytes);
    let cipher = Aes256Gcm::new(key);

    // Reconstruir el Nonce y los bytes cifrados desde el formato Hex
    let nonce_bytes = hex::decode(&encrypted.nonce)
        .map_err(|_| "Nonce inválido (No se pudo decodificar Hex)".to_string())?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext_bytes = hex::decode(&encrypted.ciphertext)
        .map_err(|_| "Ciphertext inválido (No se pudo decodificar Hex)".to_string())?;

    // Desencriptar
    let decrypted_bytes = cipher
        .decrypt(nonce, ciphertext_bytes.as_ref())
        .map_err(|e| format!("Error de descifrado (¿Clave incorrecta?): {:?}", e))?;

    // Convertir los bytes descifrados de vuelta a un String UTF-8 legible
    String::from_utf8(decrypted_bytes)
        .map_err(|_| "Los datos descifrados no contienen un UTF-8 válido".to_string())
}

// Clave fija para el Índice Ciego (En producción vendrá derivada de la frase semilla)
const BLIND_INDEX_SALT: &[u8] = b"una-sal-secreta-para-indices-ciegos-123";

/// Genera un Hash determinista e irreversible para buscar duplicados sin revelar el dato real
pub fn generate_blind_index(identity_doc: &str) -> String {
    let mut hasher = Sha256::new();
    // Combinamos la sal con el dato normalizado (sin espacios ni guiones para evitar fallas)
    let normalized = identity_doc.trim().replace("-", "").to_lowercase();

    hasher.update(BLIND_INDEX_SALT);
    hasher.update(normalized.as_bytes());

    let result = hasher.finalize();
    hex::encode(result)
}

// =======================================================================
// 3. FIRMA DIGITAL Y NO-REPUDIO (Ed25519)
// =======================================================================

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

/// Genera un par de llaves asimétricas (Privada y Pública) Ed25519.
pub fn generate_keypair() -> (SigningKey, VerifyingKey) {
    let mut csprng = rand::rngs::OsRng;
    // Generamos 32 bytes aleatorios seguros
    let mut secret_bytes = [0u8; 32];
    csprng.fill_bytes(&mut secret_bytes);

    // Creamos la llave privada a partir de los bytes
    let signing_key = SigningKey::from_bytes(&secret_bytes);
    let verifying_key = VerifyingKey::from(&signing_key);

    (signing_key, verifying_key)
}

/// Calcula un Hash SHA-256 de un texto plano (ej. la nota SOAP concatenada).
pub fn hash_document(document_text: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(document_text.as_bytes());
    let result = hasher.finalize();
    let mut hash_array = [0u8; 32];
    hash_array.copy_from_slice(&result);
    hash_array
}

/// Firma un Hash utilizando la Llave Privada del médico.
pub fn sign_hash(hash: &[u8; 32], private_key_bytes: &[u8; 32]) -> Result<String, String> {
    let signing_key = SigningKey::from_bytes(private_key_bytes);
    let signature: Signature = signing_key.sign(hash);
    Ok(hex::encode(signature.to_bytes()))
}

/// Verifica una firma digital usando la Llave Pública del médico.
pub fn verify_signature(
    hash: &[u8; 32],
    signature_hex: &str,
    public_key_hex: &str,
) -> Result<bool, String> {
    let public_key_bytes = hex::decode(public_key_hex)
        .map_err(|_| "Llave pública inválida (Hex error)".to_string())?;

    let public_key_array: [u8; 32] = public_key_bytes.as_slice().try_into().map_err(|_| {
        format!(
            "La llave pública no tiene 32 bytes (tiene {})",
            public_key_bytes.len()
        )
    })?;

    let verifying_key = VerifyingKey::from_bytes(&public_key_array)
        .map_err(|e| format!("Error al parsear llave pública: {}", e))?;

    let signature_bytes =
        hex::decode(signature_hex).map_err(|_| "Firma inválida (Hex error)".to_string())?;

    let signature_array: [u8; 64] = signature_bytes.as_slice().try_into().map_err(|_| {
        format!(
            "La firma no tiene 64 bytes (tiene {})",
            signature_bytes.len()
        )
    })?;

    let signature = Signature::from_bytes(&signature_array);

    // Aquí está la magia: en lugar de .is_ok(), usamos match para capturar el error exacto
    match verifying_key.verify(hash, &signature) {
        Ok(_) => Ok(true),
        Err(e) => Err(format!("Firma matemáticamente inválida: {}", e)), // <-- Este error nos dirá la verdad
    }
}
