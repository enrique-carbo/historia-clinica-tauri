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
