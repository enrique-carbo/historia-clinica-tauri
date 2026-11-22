//! Funciones de servicio del Core (`docs/core-api.md` §3).
//!
//! Todo acá es Rust puro: recibe `&Connection`, la DataKey ya resuelta y el
//! actor de la sesión. La capa Tauri (`ModuleContext` in-process) es la que
//! toma los locks; estas funciones no saben de `State`.

use crate::core::dto::{AuditEvent, FiliationDto, FiliationFieldDto, License, ProfessionalDto};
use crate::db::config_schema::SchemaConfig;
use crate::error::{
    CommandError, ERR_ENTITY_ACCESS_DENIED, ERR_ENTITY_NOT_FOUND, ERR_NOT_PATIENT_ENTITY,
    ERR_USER_NOT_FOUND,
};
use crate::security::crypto;
use crate::types::SessionUser;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;

/// Campos con los que se arma el atajo `full_name`. Ambos perfiles los
/// comparten (misma convención que `export/render.rs`).
const GIVEN_NAME_FIELD: &str = "nombre";
const FAMILY_NAME_FIELD: &str = "apellido";

/// Scoping de seguridad: un actor `paciente` solo puede operar sobre su
/// propia ficha (`users.entity_id`) y con cuenta activa (`is_active = 1`).
/// Staff (`administrador`, `asistente`, `profesional`) pasa; cualquier otro
/// rol desconocido falla cerrado (`AccessDenied`).
fn ensure_entity_scope(
    conn: &Connection,
    actor: &SessionUser,
    entity_id: i64,
) -> Result<(), CommandError> {
    match actor.role.as_str() {
        "administrador" | "asistente" | "profesional" => Ok(()),
        "paciente" => {
            let own: Option<Option<i64>> = conn
                .query_row(
                    "SELECT entity_id FROM users WHERE id = ?1 AND is_active = 1",
                    params![actor.user_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?;

            match own {
                Some(Some(id)) if id == entity_id => Ok(()),
                _ => Err(CommandError::new(ERR_ENTITY_ACCESS_DENIED)),
            }
        }
        _ => Err(CommandError::new(ERR_ENTITY_ACCESS_DENIED)),
    }
}

/// `true` si la ficha existe y el actor puede verla.
pub fn entity_exists(
    conn: &Connection,
    actor: &SessionUser,
    entity_id: i64,
) -> Result<bool, CommandError> {
    ensure_entity_scope(conn, actor, entity_id)?;
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM entities WHERE id = ?1)",
            params![entity_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(exists)
}

/// Descifra el blob de campos de una entidad (`{ campo: "ciphertext:nonce" }`).
/// Único camino de descifrado compartido entre `core::api` y `entity_commands`.
pub(crate) fn decrypt_entity_blob(
    enc_data_blob: &[u8],
    data_key: &[u8; 32],
) -> Result<HashMap<String, String>, CommandError> {
    let encrypted_fields: HashMap<String, String> = serde_json::from_slice(enc_data_blob)
        .map_err(|e| format!("Error parseando blob: {}", e))?;

    let mut data = HashMap::with_capacity(encrypted_fields.len());
    for (field_name, combined) in encrypted_fields {
        let parts: Vec<&str> = combined.split(':').collect();
        if parts.len() != 2 {
            return Err(CommandError::internal(format!(
                "Formato inválido en campo {}",
                field_name
            )));
        }

        let enc = crypto::EncryptedData {
            ciphertext: parts[0].to_string(),
            nonce: parts[1].to_string(),
        };
        let value = crypto::decrypt_text(&enc, data_key)
            .map_err(|e| format!("Error descifrando campo {}: {}", field_name, e))?;
        data.insert(field_name, value);
    }
    Ok(data)
}

fn load_entity_fields(
    conn: &Connection,
    data_key: &[u8; 32],
    entity_id: i64,
) -> Result<(String, HashMap<String, String>), CommandError> {
    let row: Option<(String, Vec<u8>)> = conn
        .query_row(
            "SELECT entity_type, enc_data_blob FROM entities WHERE id = ?1",
            params![entity_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    let (entity_type, blob) =
        row.ok_or_else(|| CommandError::new(ERR_ENTITY_NOT_FOUND))?;
    let data = decrypt_entity_blob(&blob, data_key)?;
    Ok((entity_type, data))
}

/// Proyección de la ficha del paciente. Incluye `full_name` y `document`
/// como atajos, y `fields` con todos los campos no vacíos en el orden del
/// schema activo (Med o Vet), sin listas hardcodeadas.
pub fn fetch_filiation(
    conn: &Connection,
    data_key: &[u8; 32],
    schema: &SchemaConfig,
    actor: &SessionUser,
    entity_id: i64,
) -> Result<FiliationDto, CommandError> {
    ensure_entity_scope(conn, actor, entity_id)?;
    let (entity_type, data) = load_entity_fields(conn, data_key, entity_id)?;

    if entity_type != schema.default_entity {
        return Err(CommandError::new(ERR_NOT_PATIENT_ENTITY));
    }

    let document_field = schema.get_blind_index_field(&entity_type)?;
    let document = data.get(&document_field).cloned().unwrap_or_default();

    let full_name = [GIVEN_NAME_FIELD, FAMILY_NAME_FIELD]
        .iter()
        .filter_map(|key| data.get(*key))
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    let fields = schema
        .field_labels(&entity_type)?
        .into_iter()
        .filter_map(|(key, label)| {
            let value = data.get(&key)?;
            if value.trim().is_empty() {
                return None;
            }
            Some(FiliationFieldDto {
                key,
                label,
                value: value.clone(),
            })
        })
        .collect();

    Ok(FiliationDto {
        entity_id,
        full_name,
        document,
        fields,
    })
}

const PROFILE_COLUMNS: usize = 9;

/// Proyección del profesional (nombre, profesión, matrículas y pública).
/// Duplica la selección acotada de `load_profile`; si el perfil profesional
/// suma campos, mantener ambos en sync.
pub fn fetch_professional(
    conn: &Connection,
    data_key: &[u8; 32],
    user_id: &str,
) -> Result<ProfessionalDto, CommandError> {
    let cols: Option<Vec<String>> = conn
        .query_row(
            "SELECT full_name_ciphertext, full_name_nonce,
                    profession_ciphertext, profession_nonce,
                    specialty_ciphertext, specialty_nonce,
                    licenses_ciphertext, licenses_nonce,
                    public_key
             FROM professional_profiles
             WHERE user_id = ?1",
            params![user_id],
            |row| {
                (0..PROFILE_COLUMNS)
                    .map(|i| row.get::<_, String>(i))
                    .collect::<rusqlite::Result<Vec<String>>>()
            },
        )
        .optional()
        .map_err(|e| e.to_string())?;

    let cols = cols.ok_or_else(|| CommandError::new(ERR_USER_NOT_FOUND))?;

    // Par vacío = campo nunca cargado → String vacío (mismo criterio que
    // `load_profile`). Un JSON de matrículas inválido sí es corrupción.
    let decrypt = |ct: &str, nonce: &str| -> Result<String, String> {
        if ct.is_empty() {
            return Ok(String::new());
        }
        let enc = crypto::EncryptedData {
            ciphertext: ct.to_string(),
            nonce: nonce.to_string(),
        };
        crypto::decrypt_text(&enc, data_key)
            .map_err(|e| format!("Error descifrando perfil: {}", e))
    };

    let licenses_json = decrypt(&cols[6], &cols[7])?;
    let licenses: Vec<License> = if licenses_json.is_empty() {
        Vec::new()
    } else {
        serde_json::from_str(&licenses_json).map_err(|e| {
            format!(
                "No se pudieron leer las matrículas del perfil: {}",
                e
            )
        })?
    };

    Ok(ProfessionalDto {
        user_id: user_id.to_string(),
        full_name: decrypt(&cols[0], &cols[1])?,
        profession: decrypt(&cols[2], &cols[3])?,
        specialty: decrypt(&cols[4], &cols[5])?,
        licenses,
        public_key_hex: cols[8].clone(),
    })
}

/// Registra un `AuditEvent` cerrado. No hay texto libre: el detalle sale del
/// enum, así que es estructuralmente imposible filtrar PHI.
pub fn audit(
    conn: &Connection,
    actor_user_id: &str,
    event: &AuditEvent,
) -> Result<(), CommandError> {
    record_audit(conn, actor_user_id, event.action(), None, &event.to_detail())?;
    Ok(())
}

/// Registra una acción sensible en `audit_log`.
///
/// Movido desde `commands/admin_commands.rs` para que `core` no dependa de la
/// capa IPC; `commands/mod.rs` lo re-exporta para conservar los call sites.
pub fn record_audit(
    conn: &Connection,
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

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; 32] = [7u8; 32];

    /// Fixture propia: prueba el MECANISMO de proyección, no el contenido de
    /// `profiles/{med,vet}/entity_schema.json` (regla de `AGENTS.md`).
    fn fixture_schema() -> SchemaConfig {
        serde_json::from_str(
            r#"{
                "version": 1,
                "default_entity": "patient",
                "entities": {
                    "patient": {
                        "label": "Paciente",
                        "label_plural": "Pacientes",
                        "blind_index_field": "dni",
                        "fields": [
                            {"name": "nombre", "label": "Nombre", "type": "text",
                             "required": true, "encrypted": true},
                            {"name": "apellido", "label": "Apellido", "type": "text",
                             "required": true, "encrypted": true},
                            {"name": "dni", "label": "DNI / Documento", "type": "text",
                             "required": true, "encrypted": false, "blind_index": true},
                            {"name": "fecha_nacimiento", "label": "Fecha de Nacimiento",
                             "type": "date", "required": false, "encrypted": true},
                            {"name": "sexo", "label": "Sexo", "type": "select",
                             "required": false, "encrypted": true},
                            {"name": "seguro_salud", "label": "Seguro Salud", "type": "text",
                             "required": false, "encrypted": true},
                            {"name": "afiliado", "label": "Afiliado", "type": "text",
                             "required": false, "encrypted": true},
                            {"name": "responsable", "label": "Responsable - Tutor",
                             "type": "group", "required": false,
                             "fields": [
                                {"name": "nombre_completo", "label": "Nombre y Apellido Tutor",
                                 "type": "text", "required": false, "encrypted": true}
                             ]}
                        ]
                    }
                }
            }"#,
        )
        .expect("fixture de schema válida")
    }

    fn session(user_id: &str, role: &str) -> SessionUser {
        SessionUser {
            user_id: user_id.to_string(),
            username: "test".to_string(),
            role: role.to_string(),
        }
    }

    fn db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            r#"
            CREATE TABLE users (
                id TEXT PRIMARY KEY, username TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL, role TEXT NOT NULL,
                entity_id INTEGER, created_at TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1, last_login_at TEXT
            );
            CREATE TABLE entities (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                external_id TEXT UNIQUE,
                entity_type TEXT NOT NULL,
                created_by_user_id TEXT NOT NULL,
                blind_index TEXT UNIQUE,
                enc_data_blob BLOB NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE professional_profiles (
                user_id TEXT PRIMARY KEY,
                full_name_ciphertext TEXT NOT NULL, full_name_nonce TEXT NOT NULL,
                specialty_ciphertext TEXT NOT NULL, specialty_nonce TEXT NOT NULL,
                profession_ciphertext TEXT NOT NULL DEFAULT '', profession_nonce TEXT NOT NULL DEFAULT '',
                licenses_ciphertext TEXT NOT NULL DEFAULT '', licenses_nonce TEXT NOT NULL DEFAULT '',
                city_ciphertext TEXT NOT NULL DEFAULT '', city_nonce TEXT NOT NULL DEFAULT '',
                country_ciphertext TEXT NOT NULL DEFAULT '', country_nonce TEXT NOT NULL DEFAULT '',
                public_key TEXT NOT NULL,
                address_ciphertext TEXT NOT NULL DEFAULT '', address_nonce TEXT NOT NULL DEFAULT '',
                phone_ciphertext TEXT NOT NULL DEFAULT '', phone_nonce TEXT NOT NULL DEFAULT '',
                email_ciphertext TEXT NOT NULL DEFAULT '', email_nonce TEXT NOT NULL DEFAULT '',
                website_ciphertext TEXT NOT NULL DEFAULT '', website_nonce TEXT NOT NULL DEFAULT '',
                updated_at TEXT NOT NULL
            );
            CREATE TABLE audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                actor_user_id TEXT, action TEXT NOT NULL,
                target_user_id TEXT, detail TEXT, created_at TEXT NOT NULL
            );
            "#,
        )
        .expect("ddl de prueba");
        conn
    }

    fn insert_user(conn: &Connection, id: &str, role: &str, entity_id: Option<i64>) {
        conn.execute(
            "INSERT INTO users (id, username, password_hash, role, entity_id, created_at)
             VALUES (?1, ?1, 'hash_test', ?2, ?3, '2026-01-01T00:00:00Z')",
            params![id, role, entity_id],
        )
        .expect("insert user");
    }

    fn enc_pair(value: &str) -> (String, String) {
        let enc = crypto::encrypt_text(value, &KEY).expect("encrypt");
        (enc.ciphertext, enc.nonce)
    }

    fn enc_field(value: &str) -> String {
        let (ciphertext, nonce) = enc_pair(value);
        format!("{}:{}", ciphertext, nonce)
    }

    fn insert_entity(conn: &Connection, id: i64, entity_type: &str, data: &[(&str, &str)]) {
        let encrypted: HashMap<String, String> = data
            .iter()
            .map(|(key, value)| ((*key).to_string(), enc_field(value)))
            .collect();
        let blob = serde_json::to_vec(&encrypted).expect("serialize blob");
        conn.execute(
            "INSERT INTO entities
                (id, external_id, entity_type, created_by_user_id, blind_index, enc_data_blob, created_at)
             VALUES (?1, NULL, ?2, 'staff', NULL, ?3, '2026-01-01T00:00:00Z')",
            params![id, entity_type, blob],
        )
        .expect("insert entity");
    }

    fn insert_profile(
        conn: &Connection,
        user_id: &str,
        full_name: &str,
        profession: &str,
        specialty: &str,
        licenses: &[License],
    ) {
        let (fn_ct, fn_n) = enc_pair(full_name);
        let (pr_ct, pr_n) = enc_pair(profession);
        let (sp_ct, sp_n) = enc_pair(specialty);
        let licenses_json = serde_json::to_string(licenses).expect("licenses json");
        let (li_ct, li_n) = enc_pair(&licenses_json);
        conn.execute(
            "INSERT INTO professional_profiles
                (user_id, full_name_ciphertext, full_name_nonce,
                 specialty_ciphertext, specialty_nonce,
                 profession_ciphertext, profession_nonce,
                 licenses_ciphertext, licenses_nonce,
                 public_key, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'pk-test', '2026-01-01T00:00:00Z')",
            params![user_id, fn_ct, fn_n, sp_ct, sp_n, pr_ct, pr_n, li_ct, li_n],
        )
        .expect("insert profile");
    }

    #[test]
    fn fetch_filiation_projects_name_document_and_schema_fields() {
        let conn = db();
        insert_user(&conn, "staff", "profesional", None);
        insert_entity(
            &conn,
            1,
            "patient",
            &[
                ("nombre", "Ana"),
                ("apellido", "Pérez"),
                ("dni", "30.111.222"),
                ("fecha_nacimiento", "1990-05-04"),
                ("seguro_salud", "OSDE"),
                ("afiliado", "123"),
            ],
        );

        let dto = fetch_filiation(
            &conn,
            &KEY,
            &fixture_schema(),
            &session("staff", "profesional"),
            1,
        )
        .expect("filiación");

        assert_eq!(dto.entity_id, 1);
        assert_eq!(dto.full_name, "Ana Pérez");
        assert_eq!(dto.document, "30.111.222");
        let keys: Vec<&str> = dto.fields.iter().map(|f| f.key.as_str()).collect();
        assert_eq!(
            keys,
            vec![
                "nombre",
                "apellido",
                "dni",
                "fecha_nacimiento",
                "seguro_salud",
                "afiliado"
            ]
        );
        let afiliado = dto
            .fields
            .iter()
            .find(|f| f.key == "afiliado")
            .expect("afiliado presente");
        assert_eq!(afiliado.label, "Afiliado");
        assert_eq!(afiliado.value, "123");
    }

    #[test]
    fn fetch_filiation_patient_can_read_own_entity() {
        let conn = db();
        insert_user(&conn, "pac", "paciente", Some(1));
        insert_entity(&conn, 1, "patient", &[("nombre", "Ana")]);

        fetch_filiation(
            &conn,
            &KEY,
            &fixture_schema(),
            &session("pac", "paciente"),
            1,
        )
        .expect("paciente lee su propia ficha");
    }

    #[test]
    fn fetch_filiation_patient_cannot_read_foreign_entity() {
        let conn = db();
        insert_user(&conn, "pac", "paciente", Some(1));
        insert_entity(&conn, 2, "patient", &[("nombre", "Otra")]);

        let err = fetch_filiation(
            &conn,
            &KEY,
            &fixture_schema(),
            &session("pac", "paciente"),
            2,
        )
        .expect_err("ficha ajena debe denegarse");
        assert_eq!(err.code, ERR_ENTITY_ACCESS_DENIED);
    }

    #[test]
    fn fetch_filiation_patient_without_entity_id_is_denied() {
        let conn = db();
        insert_user(&conn, "pac", "paciente", None);
        insert_entity(&conn, 1, "patient", &[("nombre", "Ana")]);

        let err = fetch_filiation(
            &conn,
            &KEY,
            &fixture_schema(),
            &session("pac", "paciente"),
            1,
        )
        .expect_err("paciente sin entity_id no tiene acceso");
        assert_eq!(err.code, ERR_ENTITY_ACCESS_DENIED);
    }

    #[test]
    fn fetch_filiation_missing_entity_returns_not_found() {
        let conn = db();
        insert_user(&conn, "staff", "profesional", None);

        let err = fetch_filiation(
            &conn,
            &KEY,
            &fixture_schema(),
            &session("staff", "profesional"),
            99,
        )
        .expect_err("ficha inexistente");
        assert_eq!(err.code, ERR_ENTITY_NOT_FOUND);
    }

    #[test]
    fn fetch_filiation_rejects_non_patient_entity() {
        let conn = db();
        insert_user(&conn, "staff", "profesional", None);
        insert_entity(&conn, 1, "provider", &[("nombre", "Clínica")]);

        let err = fetch_filiation(
            &conn,
            &KEY,
            &fixture_schema(),
            &session("staff", "profesional"),
            1,
        )
        .expect_err("no es una ficha de paciente");
        assert_eq!(err.code, ERR_NOT_PATIENT_ENTITY);
    }

    #[test]
    fn entity_exists_scopes_patient_to_own_entity() {
        let conn = db();
        insert_user(&conn, "pac", "paciente", Some(1));
        insert_entity(&conn, 1, "patient", &[("nombre", "Ana")]);
        insert_entity(&conn, 2, "patient", &[("nombre", "Otra")]);

        assert!(
            entity_exists(&conn, &session("pac", "paciente"), 1).expect("propia")
        );

        let err = entity_exists(&conn, &session("pac", "paciente"), 2)
            .expect_err("ficha ajena");
        assert_eq!(err.code, ERR_ENTITY_ACCESS_DENIED);

        assert!(!entity_exists(&conn, &session("staff", "profesional"), 99)
            .expect("staff consulta inexistente"));
    }

    #[test]
    fn fetch_professional_projects_profession_and_licenses() {
        let conn = db();
        insert_user(&conn, "user-a", "profesional", None);
        insert_profile(
            &conn,
            "user-a",
            "Dra. Ana Pérez",
            "Médica",
            "Cardiología",
            &[
                License {
                    number: "MP-11111".to_string(),
                    jurisdiction: "Córdoba".to_string(),
                },
                License {
                    number: "MN-22222".to_string(),
                    jurisdiction: "CABA".to_string(),
                },
            ],
        );

        let dto = fetch_professional(&conn, &KEY, "user-a").expect("perfil");
        assert_eq!(dto.user_id, "user-a");
        assert_eq!(dto.full_name, "Dra. Ana Pérez");
        assert_eq!(dto.profession, "Médica");
        assert_eq!(dto.specialty, "Cardiología");
        assert_eq!(dto.public_key_hex, "pk-test");
        assert_eq!(dto.licenses.len(), 2);
        assert_eq!(dto.licenses[0].number, "MP-11111");
        assert_eq!(dto.licenses[0].jurisdiction, "Córdoba");
    }

    #[test]
    fn fetch_filiation_rejects_unknown_role_fail_closed() {
        let conn = db();
        insert_user(&conn, "weird", "visitante", None);
        insert_entity(&conn, 1, "patient", &[("nombre", "Ana")]);

        let err = fetch_filiation(
            &conn,
            &KEY,
            &fixture_schema(),
            &session("weird", "visitante"),
            1,
        )
        .expect_err("rol desconocido debe denegarse fail-closed");
        assert_eq!(err.code, ERR_ENTITY_ACCESS_DENIED);
    }

    #[test]
    fn fetch_filiation_inactive_patient_is_denied() {
        let conn = db();
        insert_user(&conn, "pac", "paciente", Some(1));
        conn.execute("UPDATE users SET is_active = 0 WHERE id = 'pac'", [])
            .expect("deactivate");
        insert_entity(&conn, 1, "patient", &[("nombre", "Ana")]);

        let err = fetch_filiation(
            &conn,
            &KEY,
            &fixture_schema(),
            &session("pac", "paciente"),
            1,
        )
        .expect_err("paciente inactivo no debe acceder");
        assert_eq!(err.code, ERR_ENTITY_ACCESS_DENIED);
    }

    #[test]
    fn fetch_professional_missing_returns_user_not_found() {
        let conn = db();
        let err = fetch_professional(&conn, &KEY, "inexistente")
            .expect_err("perfil inexistente");
        assert_eq!(err.code, ERR_USER_NOT_FOUND);
    }

    #[test]
    fn audit_records_closed_event_without_phi() {
        let conn = db();
        insert_user(&conn, "user-a", "profesional", None);

        audit(
            &conn,
            "user-a",
            &AuditEvent::SolicitudIssued {
                subject_id: 7,
                solicitud_id: "sol-1".to_string(),
                sha256: "abc123".to_string(),
            },
        )
        .expect("audit");

        let (action, detail): (String, String) = conn
            .query_row(
                "SELECT action, detail FROM audit_log ORDER BY id DESC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("fila de auditoría");

        assert_eq!(action, "solicitud_issued");
        assert_eq!(detail, "subject_id=7 solicitud_id=sol-1 sha256=abc123");
        assert!(!detail.contains("Pérez"), "detalle: {}", detail);
        assert!(!detail.contains(".md"), "detalle: {}", detail);
    }
}
