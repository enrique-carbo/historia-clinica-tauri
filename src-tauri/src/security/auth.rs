use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::rngs::OsRng;

/// Toma una contraseña en texto plano y devuelve un hash Argon2id seguro
pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("Error al procesar el hash: {}", e))?
        .to_string();

    Ok(password_hash)
}

/// Verifica si la contraseña provista coincide con el hash guardado en SQLite
pub fn verify_password(password: &str, hash_to_check: &str) -> Result<bool, String> {
    let parsed_hash = PasswordHash::new(hash_to_check)
        .map_err(|e| format!("Error al parsear el hash guardado: {}", e))?;

    let argon2 = Argon2::default();

    // Compara de forma segura (evitando ataques de sincronización / timing attacks)
    Ok(argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}
