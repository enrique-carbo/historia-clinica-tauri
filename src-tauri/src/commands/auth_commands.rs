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
        conn.execute(
            "INSERT INTO users (id, username, password_hash, role, created_at) VALUES (?1, ?2, ?3, ?4, ?5);",
            [&user_id, &form.username, &hashed, &form.role, &now],
        ).map_err(|e| {
            if e.to_string().contains("UNIQUE constraint failed") {
                "El nombre de usuario ya existe en este sistema local.".to_string()
            } else {
                format!("Error al registrar usuario: {}", e)
            }
        })?;

        println!(
            "👤 [SQLite] Nuevo usuario registrado -> [{}]: {}",
            form.role, form.username
        );
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
    crypto_state: State<'_, CryptoState>,
) -> Result<bool, String> {
    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Error al obtener ruta del sistema: {}", e))?;

    let master_key = vault::unlock_user_vault(app_dir, &user_id, &password)?;

    let mut key_guard = crypto_state.0.lock().unwrap();
    *key_guard = Some(master_key);

    println!(
        "🔐 [Core] Llave maestra inyectada en memoria para el usuario: {}",
        user_id
    );
    Ok(true)
}

#[tauri::command]
pub fn lock_vault(state: State<'_, CryptoState>) -> Result<(), String> {
    // Como CryptoState usa Mutex<Option<T>>, simplemente le ponemos None.
    // Esto hace que la clave maestra se borre de la RAM inmediatamente.
    let mut crypto_state = state.0.lock().unwrap();
    *crypto_state = None;
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
