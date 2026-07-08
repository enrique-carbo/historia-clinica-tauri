use crate::auth;
use crate::lib_types::{CryptoState, DbState};
use crate::vault;
use tauri::{Manager, State};
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct RegisterInput {
    pub username: String,
    pub password_plain: String,
    pub role: String,
}

#[derive(serde::Deserialize)]
pub struct LoginInput {
    pub username: String,
    pub password_plain: String,
}

#[derive(serde::Serialize)]
pub struct AuthResponse {
    pub user_id: String,
    pub username: String,
    pub role: String,
}

#[tauri::command]
pub fn register_user(form: RegisterInput, db_state: State<'_, DbState>) -> Result<String, String> {
    let hashed = auth::hash_password(&form.password_plain)?;
    let user_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    super::with_conn(&db_state, |conn| {
        // 1. Insertamos el usuario
        conn.execute(
            "INSERT INTO users (id, username, password_hash, role, created_at) VALUES (?1, ?2, ?3, ?4, ?5);",
            rusqlite::params![&user_id, &form.username, &hashed, &form.role, &now],
        ).map_err(|e| format!("Error al registrar usuario: {}", e))?;

        // 2. Insertamos un perfil profesional VACÍO (sin llaves todavía)
        conn.execute(
            "INSERT INTO professional_profiles (user_id, full_name_ciphertext, full_name_nonce, license_number_ciphertext, license_number_nonce, specialty_ciphertext, specialty_nonce, public_key, updated_at)
             VALUES (?1, '', '', '', '', '', '', '', ?2);",
            rusqlite::params![&user_id, &now],
        ).map_err(|e| format!("Error al crear el perfil: {}", e))?;

        Ok(user_id)
    })
}

#[tauri::command]
pub fn login_user(form: LoginInput, db_state: State<'_, DbState>) -> Result<AuthResponse, String> {
    super::with_conn(&db_state, |conn| {
        let mut stmt = conn
            .prepare("SELECT id, password_hash, role FROM users WHERE username = ?1;")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt.query([&form.username]).map_err(|e| e.to_string())?;

        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let user_id: String = row.get(0).map_err(|e| e.to_string())?;
            let hash_guardado: String = row.get(1).map_err(|e| e.to_string())?;
            let role: String = row.get(2).map_err(|e| e.to_string())?;

            let is_valid = auth::verify_password(&form.password_plain, &hash_guardado)?;

            if is_valid {
                println!("🔓 [Auth] Login exitoso para el usuario: {}", form.username);
                Ok(AuthResponse {
                    user_id,
                    username: form.username,
                    role,
                })
            } else {
                Err("Contraseña incorrecta".to_string())
            }
        } else {
            Err("El usuario no existe".to_string())
        }
    })
}

#[tauri::command]
pub fn unlock_vault(
    user_id: String,
    password: String,
    app_handle: tauri::AppHandle,
    db_state: State<'_, crate::lib_types::DbState>,
    crypto_state: State<'_, CryptoState>,
    signing_state: State<'_, crate::lib_types::SigningState>,
) -> Result<bool, String> {
    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Error al obtener ruta del sistema: {}", e))?;

    // El vault ahora devuelve 3 cosas: llave_cifrado, llave_firma, y llave_publica_opcional
    let (master_key, private_signing_key, public_key_opt) =
        vault::unlock_user_vault(app_dir, &user_id, &password)?;

    // 1. Inyectamos la llave simétrica (AES) en CryptoState
    let mut key_guard = crypto_state.0.lock().unwrap();
    *key_guard = Some(master_key);

    // 2. Inyectamos la llave asimétrica (Ed25519) en SigningState
    let mut sign_guard = signing_state.0.lock().unwrap();
    *sign_guard = Some(private_signing_key);

    // 3. NUEVO: Si es primer login, guardamos la llave pública en la DB
    if let Some(pub_key) = public_key_opt {
        super::with_conn(&db_state, |conn| {
            conn.execute(
                "UPDATE professional_profiles SET public_key = ?1 WHERE user_id = ?2;",
                rusqlite::params![&pub_key, &user_id],
            )
            .map_err(|e| format!("Error al guardar llave pública: {}", e)) // <--- AÑADIR ESTO
        })?;
        println!("🔑 [Core] Llave pública guardada en DB para: {}", user_id);
    }

    println!("🔐 [Core] Bóveda desbloqueada. Llaves inyectadas en RAM.");
    Ok(true)
}

#[tauri::command]
pub fn lock_vault(
    crypto_state: State<'_, CryptoState>,
    signing_state: State<'_, crate::lib_types::SigningState>, // <--- NUEVO PARÁMETRO
) -> Result<(), String> {
    // Limpiamos la llave de cifrado
    let mut crypto_guard = crypto_state.0.lock().unwrap();
    *crypto_guard = None;

    // Limpiamos la llave de firma
    let mut sign_guard = signing_state.0.lock().unwrap();
    *sign_guard = None;

    println!("🔒 [Core] Bóveda bloqueada. Llaves borradas de la RAM.");
    Ok(())
}

#[tauri::command]
pub fn test_crypto_flow(
    text: String,
    crypto_state: State<'_, CryptoState>,
) -> Result<String, String> {
    let master_key = super::get_key(&crypto_state)?;
    let encrypted = crate::crypto::encrypt_text(&text, &master_key)?;
    let decrypted = crate::crypto::decrypt_text(&encrypted, &master_key)?;
    Ok(format!(
        "Cifrado Hex: {}. Descifrado: {}",
        encrypted.ciphertext, decrypted
    ))
}

#[tauri::command]
pub fn is_vault_unlocked(crypto_state: State<'_, CryptoState>) -> bool {
    crypto_state.0.lock().map(|g| g.is_some()).unwrap_or(false)
}
