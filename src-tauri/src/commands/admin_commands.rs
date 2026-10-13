use super::with_conn;
use crate::security::{auth, data_key, vault};
use crate::types::{DbState, DataKey, SessionState, SessionUser};
use tauri::{Manager, State};
use uuid::Uuid;
use zeroize::Zeroizing;

/// Exige una sesión activa con rol `administrador`.
/// Toda gestión sensible (usuarios, contraseñas, semilla, respaldos) pasa por
/// aquí — el rol nunca se confía en el frontend.
pub fn require_admin(session_state: &State<'_, SessionState>) -> Result<SessionUser, String> {
    let guard = session_state.0.lock().map_err(|_| "Lock poisoned")?;
    let session = guard.as_ref().ok_or("Sesión no iniciada. Ingresá de nuevo.")?;
    if session.role != "administrador" {
        return Err("Esta acción requiere rol administrador.".to_string());
    }
    Ok(session.clone())
}

/// Exige una sesión activa (cualquier rol). Para operaciones que todo
/// usuario autenticado puede hacer sobre **su propio** recurso — el
/// `user_id` nunca se confía en lo que manda el frontend.
pub fn require_session(session_state: &State<'_, SessionState>) -> Result<SessionUser, String> {
    let guard = session_state.0.lock().map_err(|_| "Lock poisoned")?;
    guard
        .as_ref()
        .cloned()
        .ok_or_else(|| "Sesión no iniciada. Ingresá de nuevo.".to_string())
}

/// Exige rol administrador o asistente — operaciones de admisión
/// (otorgar acceso de paciente) que no son clínicas ni requieren DataKey.
pub fn require_admin_or_assistant(
    session_state: &State<'_, SessionState>,
) -> Result<SessionUser, String> {
    let guard = session_state.0.lock().map_err(|_| "Lock poisoned")?;
    let session = guard.as_ref().ok_or("Sesión no iniciada. Ingresá de nuevo.")?;
    if session.role != "administrador" && session.role != "asistente" {
        return Err("Esta acción requiere rol administrador o asistente.".to_string());
    }
    Ok(session.clone())
}

/// Registra una acción sensible en `audit_log`.
pub fn record_audit(
    conn: &rusqlite::Connection,
    actor_user_id: &str,
    action: &str,
    target_user_id: Option<&str>,
    detail: &str,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO audit_log (actor_user_id, action, target_user_id, detail, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5);",
        rusqlite::params![
            actor_user_id,
            action,
            target_user_id,
            detail,
            chrono::Utc::now().to_rfc3339()
        ],
    )
    .map_err(|e| format!("Error registrando auditoría: {}", e))?;
    Ok(())
}

const VALID_ROLES: [&str; 4] = ["administrador", "asistente", "profesional", "paciente"];

#[derive(serde::Serialize)]
pub struct UserAdminRow {
    pub id: String,
    pub username: String,
    pub role: String,
    pub is_active: bool,
    pub created_at: String,
    pub last_login_at: Option<String>,
}

/// Cantidad total de usuarios — decide si AuthBox muestra el bootstrap
/// del primer administrador (0) o solo login (>0). Público a propósito.
#[tauri::command]
pub fn count_users(db_state: State<'_, DbState>) -> Result<i64, String> {
    with_conn(&db_state, |conn| {
        conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))
            .map_err(|e| e.to_string())
    })
}

/// Alta de usuarios desde el panel del administrador (reemplaza el
/// registro libre de la pantalla de login).
#[tauri::command]
pub fn admin_create_user(
    username: String,
    // Zeroizing: se pone a cero al salir del comando.
    password_plain: Zeroizing<String>,
    role: String,
    session_state: State<'_, SessionState>,
    db_state: State<'_, DbState>,
) -> Result<String, String> {
    let actor = require_admin(&session_state)?;

    if username.trim().is_empty() {
        return Err("El nombre de usuario no puede estar vacío.".to_string());
    }
    if password_plain.len() < 8 {
        return Err("La contraseña debe tener al menos 8 caracteres.".to_string());
    }
    if !VALID_ROLES.contains(&role.as_str()) {
        return Err(format!("Rol inválido: {}", role));
    }

    let hashed = auth::hash_password(&password_plain)?;
    let user_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    with_conn(&db_state, |conn| {
        conn.execute(
            "INSERT INTO users (id, username, password_hash, role, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5);",
            rusqlite::params![&user_id, &username, &hashed, &role, &now],
        )
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                "Ya existe un usuario con ese nombre.".to_string()
            } else {
                format!("Error al crear usuario: {}", e)
            }
        })?;

        conn.execute(
            "INSERT INTO professional_profiles (user_id, full_name_ciphertext, full_name_nonce,
                specialty_ciphertext, specialty_nonce,
                public_key, updated_at)
             VALUES (?1, '', '', '', '', ?, ?2);",
            rusqlite::params![&user_id, &now],
        )
        .map_err(|e| format!("Error al crear el perfil: {}", e))?;

        record_audit(
            conn,
            &actor.user_id,
            "user_create",
            Some(&user_id),
            &format!("username={} role={}", username.trim(), role),
        )?;

        println!("➕ [Admin] Usuario creado: {} ({}) por {}", username, role, actor.username);
        Ok(user_id)
    })
}

/// Listado de usuarios para el panel de administración.
#[tauri::command]
pub fn list_users(
    session_state: State<'_, SessionState>,
    db_state: State<'_, DbState>,
) -> Result<Vec<UserAdminRow>, String> {
    require_admin(&session_state)?;

    with_conn(&db_state, |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, username, role, is_active, created_at, last_login_at
                 FROM users ORDER BY created_at ASC;",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(UserAdminRow {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    role: row.get(2)?,
                    is_active: row.get::<_, i64>(3)? == 1,
                    created_at: row.get(4)?,
                    last_login_at: row.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut users = Vec::new();
        for row in rows {
            users.push(row.map_err(|e| e.to_string())?);
        }
        Ok(users)
    })
}

/// Baja/alta lógica de usuarios — el historial nunca se borra.
#[tauri::command]
pub fn set_user_active(
    user_id: String,
    active: bool,
    session_state: State<'_, SessionState>,
    db_state: State<'_, DbState>,
) -> Result<(), String> {
    let actor = require_admin(&session_state)?;

    if user_id == actor.user_id {
        return Err("No podés desactivar tu propia cuenta.".to_string());
    }

    with_conn(&db_state, |conn| {
        let (username, role): (String, String) = conn
            .query_row(
                "SELECT username, role FROM users WHERE id = ?1",
                rusqlite::params![&user_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| {
                if e.to_string().contains("QueryReturnedNoRows") {
                    "El usuario no existe.".to_string()
                } else {
                    e.to_string()
                }
            })?;

        if !active && role == "administrador" {
            let admins: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM users WHERE role = 'administrador' AND is_active = 1",
                    [],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            if admins <= 1 {
                return Err(
                    "No se puede desactivar: quedaría la instalación sin administradores activos."
                        .to_string(),
                );
            }
        }

        conn.execute(
            "UPDATE users SET is_active = ?1 WHERE id = ?2;",
            rusqlite::params![if active { 1 } else { 0 }, &user_id],
        )
        .map_err(|e| e.to_string())?;

        record_audit(
            conn,
            &actor.user_id,
            if active { "user_activate" } else { "user_deactivate" },
            Some(&user_id),
            &format!("username={}", username),
        )?;

        println!(
            "👤 [Admin] Usuario {} {} por {}",
            username,
            if active { "activado" } else { "desactivado" },
            actor.username
        );
        Ok(())
    })
}

/// Listado de auditoría (solo administrador).
#[tauri::command]
pub fn list_audit_log(
    session_state: State<'_, SessionState>,
    db_state: State<'_, DbState>,
) -> Result<Vec<serde_json::Value>, String> {
    require_admin(&session_state)?;

    with_conn(&db_state, |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT a.id, a.actor_user_id, u.username, a.action, a.target_user_id,
                        tu.username, a.detail, a.created_at
                 FROM audit_log a
                 LEFT JOIN users u ON u.id = a.actor_user_id
                 LEFT JOIN users tu ON tu.id = a.target_user_id
                 ORDER BY a.id DESC
                 LIMIT 200;",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, i64>(0)?,
                    "actor_user_id": row.get::<_, Option<String>>(1)?,
                    "actor_username": row.get::<_, Option<String>>(2)?,
                    "action": row.get::<_, String>(3)?,
                    "target_user_id": row.get::<_, Option<String>>(4)?,
                    "target_username": row.get::<_, Option<String>>(5)?,
                    "detail": row.get::<_, Option<String>>(6)?,
                    "created_at": row.get::<_, String>(7)?,
                }))
            })
            .map_err(|e| e.to_string())?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(|e| e.to_string())?);
        }
        Ok(entries)
    })
}

/// Reset de contraseña de OTRO usuario, sin conocer la anterior (solo
/// administrador). Regenera el vault con un keypair NUEVO:
/// - la contraseña vieja deja de servir;
/// - la llave de firma anterior se pierde, pero las entries firmadas antes
///   siguen verificándose contra el histórico `signing_keys`;
/// - la DataKey (datos clínicos) NO cambia: los datos siguen legibles.
///
/// Requiere la bóveda del administrador abierta (la DataKey está en RAM
/// para re-envolver el wrap personal del usuario reseteado).
#[tauri::command]
pub fn admin_reset_password(
    target_user_id: String,
    new_password: Zeroizing<String>,
    app_handle: tauri::AppHandle,
    session_state: State<'_, SessionState>,
    db_state: State<'_, DbState>,
    data_key_state: State<'_, DataKey>,
) -> Result<(), String> {
    let actor = require_admin(&session_state)?;

    if new_password.len() < 8 {
        return Err("La contraseña debe tener al menos 8 caracteres.".to_string());
    }
    if target_user_id == actor.user_id {
        return Err(
            "Para tu propia cuenta usá «Cambiar contraseña» desde tu perfil.".to_string(),
        );
    }

    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Error al obtener ruta del sistema: {}", e))?;

    // DataKey en RAM: solo hace falta para re-envolver el wrap de staff.
    // `None` si no está inicializada — el rol del target lo decide el core.
    let data_key_ram = super::get_data_key(&data_key_state).ok();

    with_conn(&db_state, |conn| {
        reset_password_core(
            app_dir,
            &actor.user_id,
            &target_user_id,
            new_password.as_str(),
            data_key_ram.as_ref(),
            conn,
        )
    })?;

    println!(
        "🔑 [Admin] Contraseña reseteada para {} por {}",
        target_user_id, actor.username
    );
    Ok(())
}

/// Backend puro de `admin_reset_password`, extraído para testear sin
/// `AppHandle`. Regenera la vault del target, rota `password_hash`, la llave
/// de firma y audita. La DataKey solo se re-envuelve para staff: el paciente
/// no tiene wrap personal — acá no se busca ni se crea (ausencia estructural).
fn reset_password_core(
    app_dir: std::path::PathBuf,
    actor_user_id: &str,
    target_user_id: &str,
    new_password: &str,
    data_key_ram: Option<&Zeroizing<[u8; 32]>>,
    conn: &rusqlite::Connection,
) -> Result<(), String> {
    let role: String = conn
        .query_row(
            "SELECT role FROM users WHERE id = ?1;",
            rusqlite::params![target_user_id],
            |r| r.get(0),
        )
        .map_err(|e| format!("Error consultando usuario destino: {}", e))?;

    // DataKey en RAM: sin ella no se puede re-envolver el wrap del usuario.
    // El paciente no tiene wrap → no se exige ni se guarda.
    let data_key = if role == "paciente" {
        None
    } else {
        Some(data_key_ram.ok_or_else(|| {
            "Data key no inicializada. ¿Reiniciaste la aplicación?".to_string()
        })?)
    };

    // 1. Regenerar vault (master nueva + keypair nuevo) — sin tocar la DB todavía.
    let (new_master, _new_priv, new_pub) =
        vault::reset_user_vault(app_dir.clone(), target_user_id, new_password)?;

    // 2. Re-envolver la MISMA DataKey con la nueva master (solo staff).
    if let Some(dk) = &data_key {
        data_key::save_user_wrap(&app_dir, target_user_id, dk, &new_master)?;
    }

    // 3. Hash de login + llave pública + rotación en signing_keys + auditoría.
    let hashed = auth::hash_password(new_password)?;
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "UPDATE users SET password_hash = ?1 WHERE id = ?2;",
        rusqlite::params![&hashed, target_user_id],
    )
    .map_err(|e| format!("Error actualizando contraseña en DB: {}", e))?;

    if role != "paciente" {
        conn.execute(
            "UPDATE professional_profiles SET public_key = ?1, updated_at = ?2 WHERE user_id = ?3;",
            rusqlite::params![&new_pub, &now, target_user_id],
        )
        .map_err(|e| format!("Error actualizando llave pública: {}", e))?;
    }

    // Rotación en el histórico de claves: la vieja queda vigente hasta
    // ahora, la nueva desde ahora → verificación histórica intacta.
    let current: Option<String> = conn
        .query_row(
            "SELECT public_key FROM signing_keys WHERE user_id = ?1 AND valid_to IS NULL",
            rusqlite::params![target_user_id],
            |r| r.get(0),
        )
        .ok();
    match current {
        None => {
            conn.execute(
                "INSERT INTO signing_keys (user_id, public_key, valid_from) VALUES (?1, ?2, ?3);",
                rusqlite::params![target_user_id, &new_pub, &now],
            )
            .map_err(|e| format!("Error registrando clave de firma: {}", e))?;
        }
        Some(pk) if pk != new_pub => {
            conn.execute(
                "UPDATE signing_keys SET valid_to = ?1 WHERE user_id = ?2 AND valid_to IS NULL;",
                rusqlite::params![&now, target_user_id],
            )
            .map_err(|e| format!("Error cerrando clave de firma: {}", e))?;
            conn.execute(
                "INSERT INTO signing_keys (user_id, public_key, valid_from) VALUES (?1, ?2, ?3);",
                rusqlite::params![target_user_id, &new_pub, &now],
            )
            .map_err(|e| format!("Error registrando clave de firma: {}", e))?;
        }
        _ => {}
    }

    record_audit(
        conn,
        actor_user_id,
        "password_reset",
        Some(target_user_id),
        "vault regenerado (keypair nuevo)",
    )?;

    Ok(())
}

/// Rota la frase semilla de la instalación con una frase YA verificada en el
/// frontend (generar → anotar → re-escribir → recién acá persistir). Reenvuelve
/// `.data_key.master`: la frase anterior queda invalidada.
///
/// Solo administrador con bóveda abierta (la DataKey está en RAM).
#[tauri::command]
pub fn admin_rotate_seed(
    phrase: Zeroizing<String>,
    app_handle: tauri::AppHandle,
    session_state: State<'_, SessionState>,
    db_state: State<'_, DbState>,
    data_key_state: State<'_, DataKey>,
) -> Result<(), String> {
    let actor = require_admin(&session_state)?;

    // Validación de formato/checksum de la mnemonic antes de persistir.
    if !crate::security::seed::verify_seed_phrase(phrase.trim())? {
        return Err("La frase semilla no es válida.".to_string());
    }

    let app_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Error al obtener ruta del sistema: {}", e))?;

    let data_key = super::get_data_key(&data_key_state)?;

    // Sobrescribe .data_key.master → la frase vieja deja de abrir el vault.
    data_key::save_master_wrap(&app_dir, &data_key, phrase.trim())?;

    with_conn(&db_state, |conn| {
        record_audit(
            conn,
            &actor.user_id,
            "seed_rotate",
            None,
            "frase semilla de la instalación rotada",
        )
    })?;

    println!("🌱 [Admin] Frase semilla rotada por {}", actor.username);
    Ok(())
}

// =======================================================================
// TESTS
// =======================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::database::init_db;
    use tempfile::TempDir;

    const NEW_PASSWORD: &str = "Nueva-Clave-9000";

    /// Tempdir + DB + usuario con el rol dado (sin vault ni wraps) + el
    /// actor `admin-1` real (audit_log tiene FK a users).
    fn setup_target(role: &str) -> (TempDir, rusqlite::Connection, String) {
        let dir = TempDir::new().unwrap();
        let conn = init_db(dir.path().to_path_buf()).unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO users (id, username, password_hash, role, created_at)
             VALUES ('admin-1', 'admin.test', 'hash_admin', 'administrador', ?1)",
            rusqlite::params![&now],
        )
        .unwrap();
        let uid = format!("target-{}", role);
        conn.execute(
            "INSERT INTO users (id, username, password_hash, role, created_at)
             VALUES (?1, ?2, 'hash_vieja', ?3, ?4)",
            rusqlite::params![&uid, &uid, role, &now],
        )
        .unwrap();
        (dir, conn, uid)
    }

    fn dk_ram() -> Zeroizing<[u8; 32]> {
        let mut dk = Zeroizing::new([0u8; 32]);
        rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut *dk);
        dk
    }

    #[test]
    fn test_reset_password_patient_no_wrap_created() {
        let (dir, conn, uid) = setup_target("paciente");
        // Sin DataKey en RAM: el reset del paciente no la necesita.
        reset_password_core(
            dir.path().to_path_buf(),
            "admin-1",
            &uid,
            NEW_PASSWORD,
            None,
            &conn,
        )
        .expect("reset de paciente no requiere DataKey");

        // Nunca se creó el wrap que contendría la DataKey de la clínica —
        // el artifact en disco es tan ausente como el de unlock_vault.
        assert!(
            !dir.path().join(format!(".data_key.{}", uid)).exists(),
            "el paciente no debe tener wrap personal"
        );

        // Sí rotó la contraseña de login.
        let hash: String = conn
            .query_row(
                "SELECT password_hash FROM users WHERE id = ?1",
                rusqlite::params![&uid],
                |r| r.get(0),
            )
            .unwrap();
        assert_ne!(hash, "hash_vieja");
        assert!(auth::verify_password(NEW_PASSWORD, &hash).unwrap());
    }

    #[test]
    fn test_reset_password_staff_requires_data_key() {
        let (dir, conn, uid) = setup_target("profesional");
        let err = reset_password_core(
            dir.path().to_path_buf(),
            "admin-1",
            &uid,
            NEW_PASSWORD,
            None,
            &conn,
        )
        .expect_err("staff exige DataKey en RAM");
        assert!(err.contains("Data key no inicializada"), "err: {}", err);

        // Falló ANTES de tocar nada: ni vault ni hash rotados.
        let hash: String = conn
            .query_row(
                "SELECT password_hash FROM users WHERE id = ?1",
                rusqlite::params![&uid],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(hash, "hash_vieja");
    }

    #[test]
    fn test_reset_password_staff_rewraps_data_key() {
        let (dir, conn, uid) = setup_target("administrador");
        let dk = dk_ram();
        reset_password_core(
            dir.path().to_path_buf(),
            "admin-1",
            &uid,
            NEW_PASSWORD,
            Some(&dk),
            &conn,
        )
        .expect("reset staff con DataKey");

        // El wrap del staff sí existe y se re-envuelve con la master nueva.
        assert!(dir.path().join(format!(".data_key.{}", uid)).exists());
        let hash: String = conn
            .query_row(
                "SELECT password_hash FROM users WHERE id = ?1",
                rusqlite::params![&uid],
                |r| r.get(0),
            )
            .unwrap();
        assert!(auth::verify_password(NEW_PASSWORD, &hash).unwrap());
    }

    #[test]
    fn test_reset_password_patient_ignores_data_key_even_if_provided() {
        let (dir, conn, uid) = setup_target("paciente");
        let dk = dk_ram();
        reset_password_core(
            dir.path().to_path_buf(),
            "admin-1",
            &uid,
            NEW_PASSWORD,
            Some(&dk),
            &conn,
        )
        .expect("reset paciente");

        // Aunque la DataKey estuviera en RAM, el paciente no recibe wrap.
        assert!(!dir.path().join(format!(".data_key.{}", uid)).exists());
    }

    #[test]
    fn test_reset_password_audits() {
        let (dir, conn, uid) = setup_target("paciente");
        reset_password_core(
            dir.path().to_path_buf(),
            "admin-1",
            &uid,
            NEW_PASSWORD,
            None,
            &conn,
        )
        .unwrap();
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audit_log
                 WHERE action = 'password_reset' AND actor_user_id = 'admin-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
    }
}
