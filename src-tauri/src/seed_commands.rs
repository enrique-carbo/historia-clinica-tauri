use crate::seed;

/// Genera una nueva mnemonic de 6 palabras.
/// Solo se puede llamar si no hay seed verificada.
#[tauri::command]
pub fn generate_seed() -> Result<String, String> {
    let phrase = seed::generate_mnemonic()?;
    println!("🌱 [Seed] Nueva mnemonic generada");
    Ok(phrase)
}

/// Verifica si una frase semilla coincide con la generada.
#[tauri::command]
pub fn verify_seed(phrase: String) -> Result<bool, String> {
    let is_valid = seed::verify_seed_phrase(&phrase)?;
    if is_valid {
        println!("🌱 [Seed] Frase verificada correctamente");
    } else {
        println!("⚠️ [Seed] Frase incorrecta");
    }
    Ok(is_valid)
}

/// Deriva una clave de cifrado a partir de una frase semilla.
/// Esta clave puede usarse como capa adicional de protección.
#[tauri::command]
pub fn derive_key_from_seed(phrase: String) -> Result<String, String> {
    let key = seed::derive_key_from_seed(&phrase)?;
    Ok(hex::encode(key))
}

/// Calcula el hash de la frase semilla para persistencia.
#[tauri::command]
pub fn hash_seed(phrase: String) -> Result<String, String> {
    Ok(seed::hash_seed_phrase(&phrase))
}


