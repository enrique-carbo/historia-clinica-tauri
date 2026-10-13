use crate::db::config_schema::SchemaConfig;
use crate::security::crypto;
use crate::types::{DataKey, DbState, SessionState}; // ← Cambiar CryptoState por DataKey
use rusqlite::params;
use rusqlite::OptionalExtension;
use serde::Serialize;
use std::collections::HashMap;
use tauri::State;
use uuid::Uuid;
use zeroize::Zeroizing;

use super::{
    get_data_key, record_audit, require_admin_or_assistant, require_session, with_conn,
}; // ← Usar get_data_key

pub type EntityData = HashMap<String, String>;

#[derive(Serialize)]
pub struct EntityCreated {
    pub id: i64,
    pub external_id: String,
    pub entity_type: String,
    pub blind_index_hex: String,
}

#[derive(Serialize)]
pub struct EntityRecord {
    pub id: i64,
    pub external_id: Option<String>,
    pub entity_type: String,
    pub data: EntityData,
    pub created_at: String,
}

#[tauri::command]
pub fn create_entity(
    entity_type: String,
    data: EntityData,
    session_state: State<'_, SessionState>,
    db_state: State<'_, DbState>,
    data_key_state: State<'_, DataKey>, // ← Cambiar CryptoState por DataKey
    schema: State<'_, SchemaConfig>,
) -> Result<EntityCreated, String> {
    let data_key = get_data_key(&data_key_state)?; // ← Usar data_key
    // Identidad desde la sesión: el user_id nunca se confía en el frontend.
    let created_by = require_session(&session_state)?.user_id;
    schema.validate_entity_data(&entity_type, &data)?;

    let blind_index_field = schema.get_blind_index_field(&entity_type)?;
    let dni: String = data
        .get(&blind_index_field)
        .ok_or_else(|| format!("Campo de blind index faltante: {}", blind_index_field))?
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_uppercase();

    let blind_index = crypto::generate_blind_index(&dni);
    let blind_index_hex = blind_index.clone();

    // Cifrar campos con DataKey (compartida)
    let mut encrypted_fields: HashMap<String, String> = HashMap::new();
    for (field_name, field_value) in &data {
        let enc = crypto::encrypt_text(field_value, &data_key)?; // ← Usar data_key
        let combined = format!("{}:{}", enc.ciphertext, enc.nonce);
        encrypted_fields.insert(field_name.clone(), combined);
    }

    let enc_data_json =
        serde_json::to_vec(&encrypted_fields).map_err(|e| format!("Error serializando: {}", e))?;

    let external_id = Uuid::new_v4().to_string();

    let entity_id = with_conn(&db_state, |conn| {
        conn.execute(
            "INSERT INTO entities (external_id, entity_type, created_by_user_id, blind_index, enc_data_blob, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))",
            params![external_id, entity_type, created_by, blind_index, enc_data_json],
        ).map_err(|e| e.to_string())?;
        Ok(conn.last_insert_rowid())
    })?;

    Ok(EntityCreated {
        id: entity_id,
        external_id,
        entity_type,
        blind_index_hex,
    })
}

#[tauri::command]
pub fn get_entity(
    id: i64,
    db_state: State<'_, DbState>,
    data_key_state: State<'_, DataKey>, // ← Cambiar
) -> Result<EntityRecord, String> {
    let data_key = get_data_key(&data_key_state)?; // ← Usar data_key

    let record = with_conn(&db_state, |conn| {
        let mut stmt = conn
            .prepare("SELECT external_id, entity_type, enc_data_blob, created_at FROM entities WHERE id = ?1")
            .map_err(|e| e.to_string())?;

        let row = stmt
            .query_row(params![id], |row| {
                let external_id: Option<String> = row.get(0)?;
                let entity_type: String = row.get(1)?;
                let enc_data_blob: Vec<u8> = row.get(2)?;
                let created_at: String = row.get(3)?;
                Ok((external_id, entity_type, enc_data_blob, created_at))
            })
            .map_err(|e| e.to_string())?;

        Ok(row)
    })?;

    let (external_id, entity_type, enc_data_blob, created_at) = record;

    let encrypted_fields: HashMap<String, String> = serde_json::from_slice(&enc_data_blob)
        .map_err(|e| format!("Error parseando blob: {}", e))?;

    let mut data: EntityData = HashMap::new();
    for (field_name, combined) in encrypted_fields {
        let parts: Vec<&str> = combined.split(':').collect();
        if parts.len() != 2 {
            return Err(format!("Formato inválido en campo {}", field_name));
        }

        let enc = crypto::EncryptedData {
            ciphertext: parts[0].to_string(),
            nonce: parts[1].to_string(),
        };

        let decrypted =
            crypto::decrypt_text(&enc, &data_key) // ← Usar data_key
                .map_err(|e| format!("Error descifrando campo {}: {}", field_name, e))?;

        data.insert(field_name, decrypted);
    }

    Ok(EntityRecord {
        id,
        external_id,
        entity_type,
        data,
        created_at,
    })
}

#[tauri::command]
pub fn find_entity_by_blind_index(
    entity_type: String,
    dni: String,
    db_state: State<'_, DbState>,
) -> Result<Option<i64>, String> {
    // Normalización en Rust: quita todo lo que no sea alfanumérico y pone en mayúsculas
    // Esto compatibiliza con formatos internacionales: "12.345.678", "12345678", "12345678A"
    let dni_normalized: String = dni
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_uppercase();
    let blind_index = crypto::generate_blind_index(&dni_normalized);

    let found_id = with_conn(&db_state, |conn| {
        let mut stmt = conn
            .prepare("SELECT id FROM entities WHERE entity_type = ?1 AND blind_index = ?2 LIMIT 1")
            .map_err(|e| e.to_string())?;
        let result = stmt
            .query_row(params![entity_type, blind_index], |row| {
                row.get::<_, i64>(0)
            })
            .ok();
        Ok(result)
    })?;

    Ok(found_id)
}

#[tauri::command]
pub fn list_entities(
    entity_type: String,
    limit: i64,
    offset: i64,
    db_state: State<'_, DbState>,
    data_key_state: State<'_, DataKey>, // ← Cambiar
) -> Result<Vec<EntityRecord>, String> {
    let data_key = get_data_key(&data_key_state)?; // ← Usar data_key

    let rows = with_conn(&db_state, |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, external_id, entity_type, blind_index, enc_data_blob, created_at
             FROM entities
             WHERE entity_type = ?1
             ORDER BY created_at DESC
             LIMIT ?2 OFFSET ?3",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![entity_type, limit, offset], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Vec<u8>>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })
            .map_err(|e| e.to_string())?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| e.to_string())?);
        }
        Ok(results)
    })?;

    let mut entities = Vec::new();
    for (id, external_id, entity_type, _, enc_data_blob, created_at) in rows {
        let data = decrypt_entity_blob(&enc_data_blob, &data_key)?; // ← Usar data_key

        entities.push(EntityRecord {
            id,
            external_id,
            entity_type,
            data,
            created_at,
        });
    }

    Ok(entities)
}

#[tauri::command]
pub fn search_entities(
    entity_type: String,
    query: String,
    db_state: State<'_, DbState>,
    data_key_state: State<'_, DataKey>, // ← Cambiar
) -> Result<Vec<EntityRecord>, String> {
    let data_key = get_data_key(&data_key_state)?; // ← Usar data_key

    let rows = with_conn(&db_state, |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, external_id, entity_type, blind_index, enc_data_blob, created_at
             FROM entities
             WHERE entity_type = ?1
             ORDER BY created_at DESC
             LIMIT 200",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![entity_type], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Vec<u8>>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })
            .map_err(|e| e.to_string())?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| e.to_string())?);
        }
        Ok(results)
    })?;

    let mut matches = Vec::new();
    for (id, external_id, entity_type, _blind_index, enc_data_blob, created_at) in rows {
        let data = decrypt_entity_blob(&enc_data_blob, &data_key)?; // ← Usar data_key

        let mut found = false;
        for value in data.values() {
            if value.to_lowercase().contains(&query.to_lowercase()) {
                found = true;
                break;
            }
        }

        if found {
            matches.push(EntityRecord {
                id,
                external_id,
                entity_type,
                data,
                created_at,
            });
        }
    }

    Ok(matches)
}

fn decrypt_entity_blob(enc_data_blob: &[u8], data_key: &[u8; 32]) -> Result<EntityData, String> {
    let encrypted_fields: HashMap<String, String> = serde_json::from_slice(enc_data_blob)
        .map_err(|e| format!("Error parseando blob: {}", e))?;

    let mut data: EntityData = HashMap::new();
    for (field_name, combined) in encrypted_fields {
        let parts: Vec<&str> = combined.split(':').collect();
        if parts.len() != 2 {
            return Err(format!("Formato inválido en campo {}", field_name));
        }

        let enc = crypto::EncryptedData {
            ciphertext: parts[0].to_string(),
            nonce: parts[1].to_string(),
        };

        let decrypted = crypto::decrypt_text(&enc, data_key)
            .map_err(|e| format!("Error descifrando campo {}: {}", field_name, e))?;

        data.insert(field_name, decrypted);
    }

    Ok(data)
}

#[tauri::command]
pub fn update_entity(
    id: i64,
    data: EntityData,
    session_state: State<'_, SessionState>,
    db_state: State<'_, DbState>,
    data_key_state: State<'_, DataKey>,
    schema: State<'_, SchemaConfig>,
) -> Result<(), String> {
    let data_key = get_data_key(&data_key_state)?;
    // Solo autenticación: created_by_user_id conserva al creador original.
    require_session(&session_state)?;

    // 1. Validar que la entidad existe y obtener su tipo
    let entity_type = with_conn(&db_state, |conn| {
        conn.query_row(
            "SELECT entity_type FROM entities WHERE id = ?1",
            params![id],
            |row| row.get::<_, String>(0),
        )
        .map_err(|e| format!("Entidad no encontrada o error de DB: {}", e))
    })?;

    // 2. Validar los datos contra el schema (opcional pero recomendado)
    schema.validate_entity_data(&entity_type, &data)?;

    // 3. Cifrar los nuevos datos con DataKey (mismo patrón que create_entity)
    let mut encrypted_fields: HashMap<String, String> = HashMap::new();
    for (field_name, field_value) in &data {
        let enc = crypto::encrypt_text(field_value, &data_key)?;
        let combined = format!("{}:{}", enc.ciphertext, enc.nonce);
        encrypted_fields.insert(field_name.clone(), combined);
    }

    // 4. Serializar a JSON
    let enc_data_json =
        serde_json::to_vec(&encrypted_fields).map_err(|e| format!("Error serializando: {}", e))?;

    // 5. Actualizar en SQLite
    with_conn(&db_state, |conn| {
        conn.execute(
            "UPDATE entities SET enc_data_blob = ? WHERE id = ?",
            params![enc_data_json, id],
        )
        .map_err(|e| format!("Error actualizando entidad: {}", e))?;
        Ok(())
    })
}

/// Acceso de paciente ligado a una ficha (username + estado).
#[derive(Serialize, Debug)]
pub struct PatientAccess {
    pub user_id: String,
    pub username: String,
    pub entity_id: i64,
    pub is_active: bool,
}

/// Otorga acceso de paciente a una ficha: crea el usuario
/// `role: 'paciente'` ligado con `users.entity_id` (**alta lazy** — el
/// usuario se crea solo cuando hace falta, nunca en create_entity).
/// Admin o asistente; no requiere DataKey (es información de cuenta).
#[tauri::command]
pub fn grant_patient_access(
    entity_id: i64,
    username: String,
    password_plain: Zeroizing<String>,
    session_state: State<'_, SessionState>,
    db_state: State<'_, DbState>,
) -> Result<PatientAccess, String> {
    let actor = require_admin_or_assistant(&session_state)?;

    let username = username.trim().to_string();
    if username.is_empty() {
        return Err("El nombre de usuario no puede estar vacío.".to_string());
    }
    if password_plain.len() < 8 {
        return Err("La contraseña debe tener al menos 8 caracteres.".to_string());
    }
    let hashed = crate::security::auth::hash_password(&password_plain)?;
    let user_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    with_conn(&db_state, |conn| {
        let entity_type: String = conn
            .query_row(
                "SELECT entity_type FROM entities WHERE id = ?1",
                params![entity_id],
                |r| r.get(0),
            )
            .map_err(|_| "Ficha no encontrada.".to_string())?;
        if entity_type != "patient" {
            return Err(
                "El acceso de paciente solo aplica a fichas de tipo paciente.".to_string(),
            );
        }

        let active: Option<String> = conn
            .query_row(
                "SELECT username FROM users WHERE entity_id = ?1 AND is_active = 1 LIMIT 1",
                params![entity_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if let Some(existing) = active {
            return Err(format!(
                "La ficha ya tiene un acceso activo: @{}",
                existing
            ));
        }

        conn.execute(
            "INSERT INTO users (id, username, password_hash, role, entity_id, created_at)
             VALUES (?1, ?2, ?3, 'paciente', ?4, ?5)",
            params![&user_id, &username, &hashed, entity_id, &now],
        )
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                "Ya existe un usuario con ese nombre.".to_string()
            } else {
                format!("Error creando el acceso: {}", e)
            }
        })?;

        record_audit(
            conn,
            &actor.user_id,
            "patient_access_grant",
            Some(&user_id),
            &format!("username={} entity_id={}", username, entity_id),
        )?;

        println!("🎟️ [Access] Acceso de paciente otorgado: @{} → entity {}", username, entity_id);
        Ok(PatientAccess {
            user_id,
            username,
            entity_id,
            is_active: true,
        })
    })
}

/// Estado del acceso de paciente de una ficha, si existe (la última
/// cuenta ligada, priorizando activas). No requiere DataKey. Misma puerta
/// que `grant_patient_access`: un paciente no debe enumerar accesos.
#[tauri::command]
pub fn get_entity_access(
    entity_id: i64,
    session_state: State<'_, SessionState>,
    db_state: State<'_, DbState>,
) -> Result<Option<PatientAccess>, String> {
    require_admin_or_assistant(&session_state)?;

    with_conn(&db_state, |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, username, is_active FROM users
                 WHERE entity_id = ?1
                 ORDER BY is_active DESC, created_at DESC
                 LIMIT 1",
            )
            .map_err(|e| e.to_string())?;
        stmt.query_row(params![entity_id], |r| {
            Ok(PatientAccess {
                user_id: r.get(0)?,
                username: r.get(1)?,
                entity_id,
                is_active: r.get::<_, i64>(2)? == 1,
            })
        })
        .optional()
        .map_err(|e| e.to_string())
    })
}

// =======================================================================
// TESTS
// =======================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::database::init_db;
    use crate::types::SessionUser;
    use std::sync::Mutex;
    use tauri::Manager;

    struct AccessEnv {
        app: tauri::App<tauri::test::MockRuntime>,
        entity_id: i64,
        _dir: tempfile::TempDir,
    }

    /// DB temporal + sesión con el rol dado + una ficha `patient` válida
    /// (con su creador, porque foreign_keys está ON).
    fn setup(role: &str) -> AccessEnv {
        let dir = tempfile::tempdir().expect("tempdir");
        let conn = init_db(dir.path().to_path_buf()).expect("init_db");
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO users (id, username, password_hash, role, created_at)
             VALUES ('actor-1', 'actor.test', 'hash', ?1, ?2)",
            params![role, &now],
        )
        .expect("insert actor");
        conn.execute(
            "INSERT INTO entities (entity_type, created_by_user_id, enc_data_blob, created_at)
             VALUES ('patient', 'actor-1', x'00', ?1)",
            params![&now],
        )
        .expect("insert entity");
        let entity_id = conn.last_insert_rowid();

        let app = tauri::test::mock_app();
        app.manage(DbState(Mutex::new(Some(conn))));
        app.manage(SessionState(Mutex::new(Some(SessionUser {
            user_id: "actor-1".into(),
            username: "actor".into(),
            role: role.into(),
        }))));

        AccessEnv {
            app,
            entity_id,
            _dir: dir,
        }
    }

    fn grant(env: &AccessEnv, username: &str) -> Result<PatientAccess, String> {
        grant_patient_access(
            env.entity_id,
            username.to_string(),
            Zeroizing::new("secreto123".to_string()),
            env.app.state::<SessionState>(),
            env.app.state::<DbState>(),
        )
    }

    fn q<T>(env: &AccessEnv, f: impl FnOnce(&rusqlite::Connection) -> T) -> T {
        let state = env.app.state::<DbState>();
        let guard = state.0.lock().unwrap();
        f(guard.as_ref().unwrap())
    }

    #[test]
    fn test_grant_patient_access_ok_admin() {
        let env = setup("administrador");
        let access = grant(&env, "paciente.uno").expect("grant");

        assert_eq!(access.entity_id, env.entity_id);
        assert!(access.is_active);

        q(&env, |conn| {
            let (role, entity, active): (String, i64, i64) = conn
                .query_row(
                    "SELECT role, entity_id, is_active FROM users WHERE id = ?1",
                    params![access.user_id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .unwrap();
            assert_eq!(role, "paciente");
            assert_eq!(entity, env.entity_id);
            assert_eq!(active, 1);

            // Auditoría: solo nombres de campos, nunca valores.
            let n: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM audit_log
                     WHERE action = 'patient_access_grant' AND target_user_id = ?1",
                    params![access.user_id],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(n, 1);
        });
    }

    #[test]
    fn test_grant_patient_access_ok_asistente() {
        let env = setup("asistente");
        grant(&env, "paciente.dos").expect("asistente puede otorgar acceso");
    }

    #[test]
    fn test_grant_patient_access_requires_admin_or_assistant() {
        for role in ["profesional", "paciente"] {
            let env = setup(role);
            let err = grant(&env, "paciente.tres").expect_err("debe rechazar");
            assert!(
                err.contains("administrador o asistente"),
                "role {}: {}",
                role,
                err
            );
        }
    }

    #[test]
    fn test_grant_patient_access_duplicate_active_rejected() {
        let env = setup("administrador");
        grant(&env, "paciente.uno").unwrap();
        let err = grant(&env, "paciente.otro").expect_err("segundo grant");
        assert!(err.contains("acceso activo"), "err: {}", err);
    }

    #[test]
    fn test_grant_patient_access_wrong_entity_type_rejected() {
        let env = setup("administrador");
        q(&env, |conn| {
            conn.execute(
                "INSERT INTO entities (entity_type, created_by_user_id, enc_data_blob, created_at)
                 VALUES ('staff', 'actor-1', x'00', datetime('now'))",
                [],
            )
            .unwrap();
        });
        let other_id = q(&env, |conn| conn.last_insert_rowid());
        let wrong = grant_patient_access(
            other_id,
            "paciente.cuatro".into(),
            Zeroizing::new("secreto123".to_string()),
            env.app.state::<SessionState>(),
            env.app.state::<DbState>(),
        )
        .expect_err("tipo incorrecto");
        assert!(wrong.contains("tipo paciente"), "err: {}", wrong);
    }

    #[test]
    fn test_grant_patient_access_duplicate_username_rejected() {
        let env = setup("administrador");
        grant(&env, "mismo.username").unwrap();
        // Otra ficha, mismo username → UNIQUE de users.
        q(&env, |conn| {
            conn.execute(
                "INSERT INTO entities (entity_type, created_by_user_id, enc_data_blob, created_at)
                 VALUES ('patient', 'actor-1', x'00', datetime('now'))",
                [],
            )
            .unwrap();
        });
        let other_id = q(&env, |conn| conn.last_insert_rowid());
        let err = grant_patient_access(
            other_id,
            "mismo.username".into(),
            Zeroizing::new("secreto123".to_string()),
            env.app.state::<SessionState>(),
            env.app.state::<DbState>(),
        )
        .expect_err("username duplicado");
        assert!(err.contains("Ya existe"), "err: {}", err);
    }

    #[test]
    fn test_get_entity_access_requires_admin_or_assistant() {
        // Un paciente no debe poder sondear accesos de otras fichas
        // (el resultado es un username → leak evitable).
        let env = setup("paciente");
        let err = get_entity_access(
            env.entity_id,
            env.app.state::<SessionState>(),
            env.app.state::<DbState>(),
        )
        .expect_err("debe rechazar");
        assert!(
            err.contains("administrador o asistente"),
            "err: {}",
            err
        );
    }

    #[test]
    fn test_get_entity_access_states() {
        let env = setup("asistente");

        // Sin acceso → None
        let before = get_entity_access(
            env.entity_id,
            env.app.state::<SessionState>(),
            env.app.state::<DbState>(),
        )
        .unwrap();
        assert!(before.is_none());

        // Con acceso → Some, activo
        let access = grant(&env, "paciente.uno").unwrap();
        let after = get_entity_access(
            env.entity_id,
            env.app.state::<SessionState>(),
            env.app.state::<DbState>(),
        )
        .unwrap()
        .expect("debe existir");
        assert_eq!(after.user_id, access.user_id);
        assert!(after.is_active);

        // Desactivado → sigue visible, is_active = false
        q(&env, |conn| {
            conn.execute(
                "UPDATE users SET is_active = 0 WHERE id = ?1",
                params![access.user_id],
            )
            .unwrap();
        });
        let inactive = get_entity_access(
            env.entity_id,
            env.app.state::<SessionState>(),
            env.app.state::<DbState>(),
        )
        .unwrap()
        .expect("sigue visible");
        assert!(!inactive.is_active);
    }
}
