use crate::security::{auth, seed};
use crate::security::data_key;
use crate::types::{CryptoState, DataKey, DbState, SessionState, SessionUser};
use crate::security::vault;
use tauri::{Manager, State};
use uuid::Uuid;
use zeroize::Zeroizing;

#[derive(serde::Deserialize)]
pub struct RegisterInput {
    pub username: String,
    // Zeroizing<String>: la contraseña se pone a cero al dropearse en vez de
    // quedar como bytes residuales en el heap (feature `serde` de zeroize).
    pub password_plain: Zeroizing<String>,
}

#[derive(serde::Deserialize)]
pub struct LoginInput {
    pub username: String,
    pub password_plain: Zeroizing<String>,
}

#[derive(serde::Serialize)]
pub struct AuthResponse {
    pub user_id: String,
    pub username: String,
    pub role: String,
}

/// Alta pública SOLO en bootstrap: crea el primer usuario de la instalación
/// y siempre con rol `administrador`. A partir de ahí, las altas las hace el
/// administrador desde su panel (`admin_create_user`).
#[tauri::command]
pub fn register_user(form: RegisterInput, db_state: State<'_, DbState>) -> Result<String, String> {
    let hashed = auth::hash_password(&form.password_plain)?;
    let user_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    super::with_conn(&db_state, |conn| {
        let existing: i64 = conn
            .query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;

        if existing > 0 {
            return Err(
                "El registro está cerrado. Un administrador debe darte de alta desde el panel."
                    .to_string(),
            );
        }

        // 1. Insertamos el PRIMER usuario como administrador
        conn.execute(
            "INSERT INTO users (id, username, password_hash, role, created_at) VALUES (?1, ?2, ?3, 'administrador', ?4);",
            rusqlite::params![&user_id, &form.username, &hashed, &now],
        ).map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                "Ya existe un usuario con ese nombre.".to_string()
            } else {
                format!("Error al registrar usuario: {}", e)
            }
        })?;

        // 2. Insertamos un perfil profesional VACÍO (sin llaves todavía)
        conn.execute(
            "INSERT INTO professional_profiles (user_id, full_name_ciphertext, full_name_nonce, license_number_ciphertext, license_number_nonce, specialty_ciphertext, specialty_nonce, public_key, updated_at)
             VALUES (?1, '', '', '', '', '', '', '', ?2);",
            rusqlite::params![&user_id, &now],
        ).map_err(|e| format!("Error al crear el perfil: {}", e))?;

        println!("🌱 [Auth] Primer usuario (administrador) creado: {}", form.username);
        Ok(user_id)
    })
}

#[tauri::command]
pub fn login_user(
    form: LoginInput,
    db_state: State<'_, DbState>,
    session_state: State<'_, SessionState>,
) -> Result<AuthResponse, String> {
    super::with_conn(&db_state, |conn| {
        let mut stmt = conn
            .prepare("SELECT id, password_hash, role, is_active FROM users WHERE username = ?1;")
            .map_err(|e| e.to_string())?;

        let mut rows = stmt.query([&form.username]).map_err(|e| e.to_string())?;

        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let user_id: String = row.get(0).map_err(|e| e.to_string())?;
            let hash_guardado: String = row.get(1).map_err(|e| e.to_string())?;
            let role: String = row.get(2).map_err(|e| e.to_string())?;
            let is_active: i64 = row.get(3).map_err(|e| e.to_string())?;

            if is_active == 0 {
                return Err("Usuario desactivado. Contactá al administrador.".to_string());
            }

            let is_valid = auth::verify_password(&form.password_plain, &hash_guardado)?;

            if is_valid {
                let now = chrono::Utc::now().to_rfc3339();
                conn.execute(
                    "UPDATE users SET last_login_at = ?1 WHERE id = ?2;",
                    rusqlite::params![&now, &user_id],
                )
                .map_err(|e| e.to_string())?;

                println!("🔓 [Auth] Login exitoso para el usuario: {}", form.username);

                // Registrar la sesión activa (los commands de administración
                // verifican el rol desde este estado, no desde el frontend).
                *session_state.0.lock().map_err(|_| "Lock poisoned")? = Some(SessionUser {
                    user_id: user_id.clone(),
                    username: form.username.clone(),
                    role: role.clone(),
                });

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
    password: Zeroizing<String>,
    seed_phrase: Option<Zeroizing<String>>,
    app_handle: tauri::AppHandle,
    db_state: State<'_, crate::types::DbState>,
    crypto_state: State<'_, CryptoState>,
    signing_state: State<'_, crate::types::SigningState>,
    data_key_state: State<'_, DataKey>,
) -> Result<bool, String> {
    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Error al obtener ruta del sistema: {}", e))?;

    // El vault devuelve: llave maestra, llave privada de firma y pública opcional
    // (la pública solo viene en `Some` cuando el vault se crea en ESTA llamada).
    let (master_key, private_signing_key, public_key_opt) =
        vault::unlock_user_vault(app_dir.clone(), &user_id, &password)?;

    // 1. Sincronizar la llave pública en DB ANTES de resolve_data_key.
    //    Si el primer intento falla con SEED_REQUIRED, el vault ya quedó creado
    //    y el reintento (con seed) no recibiría `Some(...)` → la pública quedaría
    //    vacía y TODAS las firmas de ese usuario verificarían como false.
    let public_key_hex = match public_key_opt {
        Some(pk) => pk,
        None => crate::security::crypto::public_key_from_private(&private_signing_key),
    };
    super::with_conn(&db_state, |conn| {
        conn.execute(
            "UPDATE professional_profiles SET public_key = ?1, updated_at = ?2 WHERE user_id = ?3;",
            rusqlite::params![
                &public_key_hex,
                &chrono::Utc::now().to_rfc3339(),
                &user_id
            ],
        )
        .map_err(|e| format!("Error al guardar llave pública: {}", e))?;

        // Histórico de claves de firma: si esta es una clave nueva (reset de
        // contraseña, rotación), cerrar la vigente y abrir la nueva. Así las
        // entries firmadas antes siguen verificándose contra su clave de época.
        let current: Option<String> = conn
            .query_row(
                "SELECT public_key FROM signing_keys WHERE user_id = ?1 AND valid_to IS NULL",
                rusqlite::params![&user_id],
                |r| r.get(0),
            )
            .ok();

        let now = chrono::Utc::now().to_rfc3339();
        match current {
            None => {
                conn.execute(
                    "INSERT INTO signing_keys (user_id, public_key, valid_from) VALUES (?1, ?2, ?3);",
                    rusqlite::params![&user_id, &public_key_hex, &now],
                )
                .map_err(|e| format!("Error registrando clave de firma: {}", e))?;
            }
            Some(pk) if pk != public_key_hex => {
                conn.execute(
                    "UPDATE signing_keys SET valid_to = ?1 WHERE user_id = ?2 AND valid_to IS NULL;",
                    rusqlite::params![&now, &user_id],
                )
                .map_err(|e| format!("Error cerrando clave de firma: {}", e))?;
                conn.execute(
                    "INSERT INTO signing_keys (user_id, public_key, valid_from) VALUES (?1, ?2, ?3);",
                    rusqlite::params![&user_id, &public_key_hex, &now],
                )
                .map_err(|e| format!("Error registrando clave de firma: {}", e))?;
                println!("🔄 [Auth] Rotación de llave de firma registrada para: {}", user_id);
            }
            _ => {}
        }

        Ok(())
    })?;
    println!("🔑 [Core] Llave pública sincronizada en DB para: {}", user_id);

    // 2. Resolver la data_key (cifrado de datos médicos) con master_key del usuario
    //    - Fast path: wrap personal .data_key.{user_id}
    //    - Bootstrap: seed + .data_key.master (nuevos usuarios / recovery)
    //    - Legacy: migración de .data_key hex en claro
    //    Puede fallar con SEED_REQUIRED → no inyectamos nada en RAM todavía.
    let resolved_data_key = data_key::resolve_data_key(
        &app_dir,
        &user_id,
        &master_key,
        seed_phrase.as_deref().map(|s| s.as_str()),
    )?;

    // 3. Inyectar llaves en RAM solo si TODO tuvo éxito
    let mut key_guard = crypto_state.0.lock().unwrap();
    *key_guard = Some(master_key);

    let mut sign_guard = signing_state.0.lock().unwrap();
    *sign_guard = Some(private_signing_key);

    let mut dk_guard = data_key_state.0.lock().unwrap();
    *dk_guard = Some(resolved_data_key);

    println!("🔐 [Core] Bóveda desbloqueada. Llaves inyectadas en RAM.");
    Ok(true)
}

/// Cambia la contraseña del usuario.
///
/// Orden de persistencia (falla más probable primero → fs antes que sqlite;
/// verificación completa antes de la primera escritura):
/// 1. Sesión desbloqueada (data_key en RAM) — sin esto, no se puede re-envolver.
/// 2. Vault: verifica contraseña actual y re-cifra (vault.rs).
/// 3. Re-envuelve `.data_key.{user_id}` con la nueva master (si falla, el wrap
///    huérfano se recupera con seed — ver resolve_data_key).
/// 4. Actualiza `users.password_hash` en DB.
/// 5. Actualiza la master en RAM (la llave de firma no cambia: mismo keypair).
///
/// No toca `.data_key.master` (deriva de la seed, independiente de la contraseña).
#[tauri::command]
pub fn change_password(
    user_id: String,
    current_password: Zeroizing<String>,
    new_password: Zeroizing<String>,
    app_handle: tauri::AppHandle,
    db_state: State<'_, DbState>,
    crypto_state: State<'_, CryptoState>,
    data_key_state: State<'_, DataKey>,
) -> Result<(), String> {
    if new_password.len() < 8 {
        return Err("La nueva contraseña debe tener al menos 8 caracteres".to_string());
    }
    if new_password == current_password {
        return Err("La nueva contraseña debe ser distinta a la actual".to_string());
    }

    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Error al obtener ruta del sistema: {}", e))?;

    // 1. Sesión desbloqueada ANTES de tocar disco: sin data_key en RAM no se
    //    puede re-envolver el wrap personal.
    let data_key = {
        let guard = data_key_state.0.lock().map_err(|_| "Lock poisoned")?;
        guard
            .as_ref()
            .cloned()
            .ok_or("La bóveda está cerrada. Abrila antes de cambiar la contraseña.".to_string())?
    };

    // 2. Verifica contraseña actual + re-cifra vault → nueva master
    let new_master = vault::change_vault_password(
        app_dir.clone(),
        &user_id,
        &current_password,
        &new_password,
    )?;

    // 3. Re-envolver data_key con la nueva master (cura el fast path)
    data_key::save_user_wrap(&app_dir, &user_id, &data_key, &new_master)?;

    // 4. Actualizar hash de login en DB
    let hashed = auth::hash_password(&new_password)?;
    super::with_conn(&db_state, |conn| {
        conn.execute(
            "UPDATE users SET password_hash = ?1 WHERE id = ?2;",
            rusqlite::params![&hashed, &user_id],
        )
        .map_err(|e| format!("Error actualizando contraseña en DB: {}", e))
    })?;

    // 5. Actualizar master en RAM (sesión sigue abierta)
    let mut key_guard = crypto_state.0.lock().map_err(|_| "Lock poisoned")?;
    *key_guard = Some(new_master);

    println!("🔑 [Auth] Contraseña cambiada para: {}", user_id);
    Ok(())
}

#[tauri::command]
pub fn lock_vault(
    crypto_state: State<'_, CryptoState>,
    signing_state: State<'_, crate::types::SigningState>,
    data_key_state: State<'_, DataKey>,
    session_state: State<'_, SessionState>,
) -> Result<(), String> {
    // Limpiamos la llave de cifrado
    let mut crypto_guard = crypto_state.0.lock().unwrap();
    *crypto_guard = None;

    // Limpiamos la llave de firma
    let mut sign_guard = signing_state.0.lock().unwrap();
    *sign_guard = None;

    // Limpiamos la data_key (datos médicos)
    let mut dk_guard = data_key_state.0.lock().unwrap();
    *dk_guard = None;

    // Limpiamos la sesión activa
    let mut session_guard = session_state.0.lock().map_err(|_| "Lock poisoned")?;
    *session_guard = None;

    println!("🔒 [Core] Bóveda bloqueada. Llaves borradas de la RAM.");
    Ok(())
}

/// Envuelve la data_key actual en memoria con la seed-derived key
/// y persiste `.data_key.master` para bootstrap/recovery.
/// Se llama desde SeedPhraseSetup después de verificar la frase.
#[tauri::command]
pub fn setup_seed_master_wrap(
    phrase: Zeroizing<String>,
    app_handle: tauri::AppHandle,
    data_key_state: State<'_, DataKey>,
) -> Result<(), String> {
    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Error al obtener ruta del sistema: {}", e))?;

    let dk_guard = data_key_state.0.lock().map_err(|_| "Lock poisoned")?;
    let data_key = dk_guard
        .as_ref()
        .cloned()
        .ok_or("Data key no disponible. Abrí la bóveda primero.".to_string())?;

    data_key::save_master_wrap(&app_dir, &data_key, &phrase)?;
    println!("🌱 [Seed] Master wrap creado (.data_key.master)");
    Ok(())
}

#[tauri::command]
pub fn test_crypto_flow(
    text: String,
    crypto_state: State<'_, CryptoState>,
) -> Result<String, String> {
    let master_key = super::get_key(&crypto_state)?;
    let encrypted = crate::security::crypto::encrypt_text(&text, &master_key)?;
    let decrypted = crate::security::crypto::decrypt_text(&encrypted, &master_key)?;
    Ok(format!(
        "Cifrado Hex: {}. Descifrado: {}",
        encrypted.ciphertext, decrypted
    ))
}

#[tauri::command]
pub fn is_vault_unlocked(crypto_state: State<'_, CryptoState>) -> bool {
    crypto_state.0.lock().map(|g| g.is_some()).unwrap_or(false)
}

// =======================================================================
// RECOVERY BREAK-GLASS CON FRASE SEMILLA
// =======================================================================

/// Resultado de un recovery exitoso: credenciales para abrir la sesión
/// completa (vault, firma y data_key) sin pasar por login/contraseña previa.
pub struct RecoveryOutcome {
    pub user_id: String,
    pub username: String,
    pub role: String,
    pub master_key: Zeroizing<[u8; 32]>,
    pub private_signing_key: Zeroizing<[u8; 32]>,
    pub data_key: Zeroizing<[u8; 32]>,
}

/// Restablece la contraseña del administrador (sin conocer la previa)
/// demostrando posesión de la frase semilla.
///
/// Orden:
/// 1. Verifica la frase contra `.data_key.master` (proof-of-possession:
///    el mismo unwrap que usa el recovery de datos — frase incorrecta no
///    distingue entre "palabras inválidas" y "frase equivocada").
/// 2. Toma el administrador activo más antiguo de la instalación.
/// 3. Regenera el vault (keypair de firma nuevo) con la contraseña nueva.
/// 4. Actualiza `password_hash` + `last_login_at`.
/// 5. Rota `signing_keys` y la pública del perfil (las firmas históricas
///    quedan verificables contra la clave de época).
/// 6. Re-envuelve el wrap personal con la master nueva (fast path).
/// 7. Registra la acción en `audit_log`.
///
/// La data_key NO cambia: el restore de seguridad es exactamente la misma
/// que ya estaba envuelta en el master wrap.
pub fn seed_recovery_core(
    app_dir: &std::path::Path,
    conn: &rusqlite::Connection,
    seed_phrase: &str,
    new_password: &str,
) -> Result<RecoveryOutcome, String> {
    if new_password.len() < 8 {
        return Err("La nueva contraseña debe tener al menos 8 caracteres".to_string());
    }
    let phrase = seed_phrase.trim();
    if phrase.is_empty() {
        return Err("Ingresá la frase semilla.".to_string());
    }

    // 1. Proof-of-possession de la frase contra el master wrap
    let master_path = app_dir.join(".data_key.master");
    if !master_path.exists() {
        return Err(
            "Esta instalación no tiene una frase semilla configurada. Recuperá con un respaldo o con una contraseña válida."
                .to_string(),
        );
    }
    let blob =
        std::fs::read(&master_path).map_err(|e| format!("No se pudo leer el master wrap: {}", e))?;
    let data_key = {
        let seed_key =
            seed::derive_key_from_seed(phrase).map_err(|_| "Frase semilla incorrecta.".to_string())?;
        data_key::unwrap_key(&blob, &seed_key)
            .map_err(|_| "Frase semilla incorrecta.".to_string())?
    };

    // 2. Administrador activo más antiguo (el de bootstrap)
    let (admin_id, admin_username): (String, String) = conn
        .query_row(
            "SELECT id, username FROM users
             WHERE role = 'administrador' AND is_active = 1
             ORDER BY created_at ASC LIMIT 1;",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| {
            "No hay usuarios administradores activos en esta instalación.".to_string()
        })?;

    // 3. Vault regenerado (keypair nuevo) con la contraseña nueva
    let (new_master, new_priv, new_pub) =
        vault::reset_user_vault(app_dir.to_path_buf(), &admin_id, new_password)?;

    // 4. password_hash + last_login_at
    let hashed = auth::hash_password(new_password)?;
    conn.execute(
        "UPDATE users SET password_hash = ?1, last_login_at = ?2 WHERE id = ?3;",
        rusqlite::params![&hashed, &chrono::Utc::now().to_rfc3339(), &admin_id],
    )
    .map_err(|e| format!("Error actualizando contraseña: {}", e))?;

    // 5. Rotación en signing_keys + pública del perfil
    let current: Option<String> = conn
        .query_row(
            "SELECT public_key FROM signing_keys WHERE user_id = ?1 AND valid_to IS NULL",
            rusqlite::params![&admin_id],
            |r| r.get(0),
        )
        .ok();
    let now = chrono::Utc::now().to_rfc3339();
    match current {
        None => {
            conn.execute(
                "INSERT INTO signing_keys (user_id, public_key, valid_from) VALUES (?1, ?2, ?3);",
                rusqlite::params![&admin_id, &new_pub, &now],
            )
            .map_err(|e| format!("Error registrando clave de firma: {}", e))?;
        }
        Some(pk) if pk != new_pub => {
            conn.execute(
                "UPDATE signing_keys SET valid_to = ?1 WHERE user_id = ?2 AND valid_to IS NULL;",
                rusqlite::params![&now, &admin_id],
            )
            .map_err(|e| format!("Error cerrando clave de firma: {}", e))?;
            conn.execute(
                "INSERT INTO signing_keys (user_id, public_key, valid_from) VALUES (?1, ?2, ?3);",
                rusqlite::params![&admin_id, &new_pub, &now],
            )
            .map_err(|e| format!("Error registrando clave de firma: {}", e))?;
        }
        _ => {}
    }
    conn.execute(
        "UPDATE professional_profiles SET public_key = ?1, updated_at = ?2 WHERE user_id = ?3;",
        rusqlite::params![&new_pub, &now, &admin_id],
    )
    .map_err(|e| format!("Error guardando llave pública: {}", e))?;

    // 6. Fast path: wrap personal con la master nueva (ya no hace falta seed)
    data_key::save_user_wrap(app_dir, &admin_id, &data_key, &new_master)?;

    // 7. Auditoría (actor y objetivo: el propio administrador)
    super::record_audit(
        conn,
        &admin_id,
        "seed_recovery",
        Some(&admin_id),
        "Contraseña restablecida con frase semilla (break-glass)",
    )?;

    println!(
        "🚨 [Auth] Recovery con semilla ejecutado para el administrador: {}",
        admin_username
    );

    Ok(RecoveryOutcome {
        user_id: admin_id,
        username: admin_username,
        role: "administrador".to_string(),
        master_key: new_master,
        private_signing_key: new_priv,
        data_key,
    })
}

/// Break-glass: recupera el acceso demostrando la frase semilla y deja la
/// sesión completamente abierta (vault, firma y data_key en RAM).
#[tauri::command]
pub fn seed_recovery(
    seed_phrase: Zeroizing<String>,
    new_password: Zeroizing<String>,
    app_handle: tauri::AppHandle,
    db_state: State<'_, DbState>,
    session_state: State<'_, SessionState>,
    crypto_state: State<'_, CryptoState>,
    signing_state: State<'_, crate::types::SigningState>,
    data_key_state: State<'_, DataKey>,
) -> Result<AuthResponse, String> {
    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Error al obtener ruta del sistema: {}", e))?;

    let outcome = super::with_conn(&db_state, |conn| {
        seed_recovery_core(&app_dir, conn, &seed_phrase, &new_password)
    })?;

    *session_state
        .0
        .lock()
        .map_err(|_| "Lock poisoned".to_string())? = Some(SessionUser {
        user_id: outcome.user_id.clone(),
        username: outcome.username.clone(),
        role: outcome.role.clone(),
    });
    *crypto_state
        .0
        .lock()
        .map_err(|_| "Lock poisoned".to_string())? = Some(outcome.master_key);
    *signing_state
        .0
        .lock()
        .map_err(|_| "Lock poisoned".to_string())? = Some(outcome.private_signing_key);
    *data_key_state
        .0
        .lock()
        .map_err(|_| "Lock poisoned".to_string())? = Some(outcome.data_key);

    Ok(AuthResponse {
        user_id: outcome.user_id,
        username: outcome.username,
        role: outcome.role,
    })
}

// =======================================================================
// TESTS
// =======================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{crypto, data_key, vault};
    use tempfile::TempDir;

    const PHRASE: &str = "abaco abrir acero acida actas acuna";
    const OLD_PASSWORD: &str = "ContraseñaVieja123!";
    const NEW_PASSWORD: &str = "ContraseñaNueva456!";

    /// `unwrap_err` exige `Debug` en el Ok; RecoveryOutcome contiene material
    /// de clave que no queremos derivar/imprimir, así que lo extraemos a mano.
    fn expect_err<T>(result: Result<T, String>) -> String {
        match result {
            Ok(_) => panic!("se esperaba un error"),
            Err(e) => e,
        }
    }

    /// Instalación de prueba: admin con vault propio, data_key y master wrap.
    fn setup_installation() -> (TempDir, rusqlite::Connection, Zeroizing<[u8; 32]>, String) {
        let dir = TempDir::new().unwrap();
        let conn = crate::db::database::init_db(dir.path().to_path_buf()).unwrap();

        let admin_id = "admin-1".to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO users (id, username, password_hash, role, created_at)
             VALUES (?1, 'admin', ?2, 'administrador', ?3)",
            rusqlite::params![
                &admin_id,
                auth::hash_password(OLD_PASSWORD).unwrap(),
                &now
            ],
        )
        .unwrap();

        // Vault con keypair viejo (como si hubiera hecho login antes)
        let (_, old_priv, _) =
            vault::unlock_user_vault(dir.path().to_path_buf(), &admin_id, OLD_PASSWORD).unwrap();

        // Llave vieja registrada en signing_keys
        conn.execute(
            "INSERT INTO signing_keys (user_id, public_key, valid_from)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![
                &admin_id,
                &crypto::public_key_from_private(&old_priv),
                &now
            ],
        )
        .unwrap();

        // data_key + master wrap con la frase
        let data_key_val = {
            let mut dk = Zeroizing::new([0u8; 32]);
            rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut *dk);
            dk
        };
        data_key::save_master_wrap(dir.path(), &data_key_val, PHRASE).unwrap();
        // Wrap personal como si el admin ya se hubiera logueado alguna vez
        let (old_master, _, _) =
            vault::unlock_user_vault(dir.path().to_path_buf(), &admin_id, OLD_PASSWORD).unwrap();
        data_key::save_user_wrap(dir.path(), &admin_id, &data_key_val, &old_master).unwrap();

        (dir, conn, data_key_val, admin_id)
    }

    #[test]
    fn test_seed_recovery_happy_path() {
        let (dir, conn, original_data_key, admin_id) = setup_installation();
        let old_hash: String = conn
            .query_row(
                "SELECT password_hash FROM users WHERE id = ?1",
                [&admin_id],
                |r| r.get(0),
            )
            .unwrap();
        let old_pub: String = conn
            .query_row(
                "SELECT public_key FROM signing_keys WHERE user_id = ?1 AND valid_to IS NULL",
                [&admin_id],
                |r| r.get(0),
            )
            .unwrap();

        let outcome =
            seed_recovery_core(dir.path(), &conn, PHRASE, NEW_PASSWORD).unwrap();

        // Sesión devuelta
        assert_eq!(outcome.role, "administrador");
        assert_eq!(outcome.username, "admin");

        // La contraseña vieja ya no abre el vault; la nueva sí, con keypair NUEVO
        assert!(
            vault::unlock_user_vault(dir.path().to_path_buf(), &admin_id, OLD_PASSWORD).is_err(),
            "la contraseña vieja debe morir"
        );
        let (_, priv_after, _) =
            vault::unlock_user_vault(dir.path().to_path_buf(), &admin_id, NEW_PASSWORD).unwrap();
        assert_eq!(priv_after, outcome.private_signing_key);

        // password_hash verificable con la contraseña nueva y muerto con la vieja
        let new_hash: String = conn
            .query_row(
                "SELECT password_hash FROM users WHERE id = ?1",
                [&admin_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_ne!(new_hash, old_hash);
        assert!(auth::verify_password(NEW_PASSWORD, &new_hash).unwrap());
        assert!(!auth::verify_password(OLD_PASSWORD, &new_hash).unwrap());

        // La data_key NO cambió: el master wrap desenvuelve exactamente la
        // misma clave que estaba envuelta antes del recovery
        let seed_key = seed::derive_key_from_seed(PHRASE).unwrap();
        let master_wrap_blob = std::fs::read(dir.path().join(".data_key.master")).unwrap();
        let master_wrap_key = data_key::unwrap_key(&master_wrap_blob, &seed_key).unwrap();
        assert_eq!(master_wrap_key, original_data_key);

        // signing_keys: la vieja quedó cerrada y hay una vigente nueva
        let closed: Option<String> = conn
            .query_row(
                "SELECT public_key FROM signing_keys WHERE user_id = ?1 AND valid_to IS NOT NULL",
                [&admin_id],
                |r| r.get(0),
            )
            .ok();
        assert_eq!(closed.as_deref(), Some(old_pub.as_str()));
        let current: String = conn
            .query_row(
                "SELECT public_key FROM signing_keys WHERE user_id = ?1 AND valid_to IS NULL",
                [&admin_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_ne!(current, old_pub);

        // Auditoría registrada
        let (action, detail): (String, String) = conn
            .query_row(
                "SELECT action, detail FROM audit_log ORDER BY id DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(action, "seed_recovery");
        assert!(detail.contains("semilla"));
    }

    #[test]
    fn test_seed_recovery_wrong_phrase_fails_without_side_effects() {
        let (dir, conn, _, admin_id) = setup_installation();
        let old_hash: String = conn
            .query_row(
                "SELECT password_hash FROM users WHERE id = ?1",
                [&admin_id],
                |r| r.get(0),
            )
            .unwrap();

        let err = expect_err(seed_recovery_core(
            dir.path(),
            &conn,
            "palabras que no son la frase correcta",
            NEW_PASSWORD,
        ));
        assert!(err.contains("Frase semilla incorrecta"), "error: {}", err);

        // Nada cambió: hash y vault intactos
        let hash_now: String = conn
            .query_row(
                "SELECT password_hash FROM users WHERE id = ?1",
                [&admin_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hash_now, old_hash);
        assert!(
            vault::unlock_user_vault(dir.path().to_path_buf(), &admin_id, OLD_PASSWORD).is_ok(),
            "el vault original debe seguir intacto"
        );
        let audit_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM audit_log", [], |r| r.get(0))
            .unwrap();
        assert_eq!(audit_count, 0, "no debe auditar un intento fallido");
    }

    #[test]
    fn test_seed_recovery_weak_password_rejected() {
        let (dir, conn, _, _) = setup_installation();
        let err = expect_err(seed_recovery_core(dir.path(), &conn, PHRASE, "corta"));
        assert!(err.contains("8 caracteres"), "error: {}", err);
    }

    #[test]
    fn test_seed_recovery_no_active_admin_rejected() {
        let (dir, conn, _, admin_id) = setup_installation();
        conn.execute(
            "UPDATE users SET is_active = 0 WHERE id = ?1",
            [&admin_id],
        )
        .unwrap();

        let err = expect_err(seed_recovery_core(dir.path(), &conn, PHRASE, NEW_PASSWORD));
        assert!(err.contains("administradores"), "error: {}", err);
    }

    #[test]
    fn test_seed_recovery_without_master_wrap_rejected() {
        let (dir, conn, _, _) = setup_installation();
        std::fs::remove_file(dir.path().join(".data_key.master")).unwrap();

        let err = expect_err(seed_recovery_core(dir.path(), &conn, PHRASE, NEW_PASSWORD));
        assert!(err.contains("no tiene una frase semilla"), "error: {}", err);
    }

    #[test]
    fn test_lock_vault_clears_all_keys() {
        use std::sync::Mutex;
        use tauri::Manager;

        let app = tauri::test::mock_app();
        app.manage(CryptoState(Mutex::new(Some(Zeroizing::new([9u8; 32])))));
        app.manage(crate::types::SigningState(Mutex::new(Some(
            Zeroizing::new([8u8; 32]),
        ))));
        app.manage(DataKey(Mutex::new(Some(Zeroizing::new([7u8; 32])))));
        app.manage(SessionState(Mutex::new(Some(SessionUser {
            user_id: "u1".into(),
            username: "admin".into(),
            role: "administrador".into(),
        }))));

        // Bóveda abierta → las extracciones funcionan
        assert!(crate::commands::get_key(&app.state::<CryptoState>()).is_ok());
        assert!(crate::commands::get_data_key(&app.state::<DataKey>()).is_ok());
        assert!(is_vault_unlocked(app.state::<CryptoState>()));

        lock_vault(
            app.state::<CryptoState>(),
            app.state::<crate::types::SigningState>(),
            app.state::<DataKey>(),
            app.state::<SessionState>(),
        )
        .unwrap();

        // Tras lock_vault ninguna llave es extraíble y la sesión está limpia
        let err = crate::commands::get_key(&app.state::<CryptoState>()).unwrap_err();
        assert!(err.contains("no está en memoria"), "err: {}", err);
        let err = crate::commands::get_data_key(&app.state::<DataKey>()).unwrap_err();
        assert!(err.contains("no inicializada"), "err: {}", err);
        assert!(!is_vault_unlocked(app.state::<CryptoState>()));
        assert!(app.state::<SessionState>().0.lock().unwrap().is_none());
        assert!(app
            .state::<crate::types::SigningState>()
            .0
            .lock()
            .unwrap()
            .is_none());
    }
}
