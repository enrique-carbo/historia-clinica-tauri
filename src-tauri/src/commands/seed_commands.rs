use crate::security::seed;
use zeroize::Zeroizing;

// Solo comandos sin path de persistencia: la frase se compara contra la
// semilla en memoria. `hash_seed` y `derive_key_from_seed` (command) se
// eliminaron (2026-10-17) — exponer el hash o la clave derivada al frontend
// creaba un bypass dormido del KDF Argon2id (ataque barato por SHA-256 si
// alguien lo persistiera); el proof-of-possession contra el master wrap ya
// lo hace `seed_recovery`.

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
pub fn verify_seed(phrase: Zeroizing<String>) -> Result<bool, String> {
    let is_valid = seed::verify_seed_phrase(&phrase)?;
    if is_valid {
        println!("🌱 [Seed] Frase verificada correctamente");
    } else {
        println!("⚠️ [Seed] Frase incorrecta");
    }
    Ok(is_valid)
}


