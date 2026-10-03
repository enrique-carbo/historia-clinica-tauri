use crate::security::crypto;
use crate::types::{DataKey, DbState, SigningState};
use rusqlite::params;
use serde::Serialize;
use std::collections::HashMap;
use tauri::State;
use uuid::Uuid;

use super::{get_data_key, with_conn};

pub type EntryPayload = HashMap<String, String>;

#[derive(Serialize, Debug)]
pub struct EntryRecord {
    pub id: String,
    pub category: String,
    pub subject_id: i64,
    pub author_id: String,
    pub author_name: String,
    pub title: String,
    pub status: String,
    pub timestamp: String,
    pub payload: EntryPayload,
    pub signature: String,
    pub is_verified: bool,
    pub created_at: String,
}

/// Crea una nueva entry inmutable (append-only).
/// NO existe update ni delete para entries.
#[tauri::command]
pub fn create_entry(
    category: String,
    subject_id: i64,
    author_id: String,
    title: String,
    status: String,
    payload: EntryPayload,
    db_state: State<'_, DbState>,
    data_key_state: State<'_, DataKey>,
    signing_state: State<'_, SigningState>,
) -> Result<String, String> {
    let data_key = get_data_key(&data_key_state)?;

    // Validar categoría
    let valid_categories = ["SOAP_NOTE", "MEDICATION", "ALLERGY", "CONDITION"];
    if !valid_categories.contains(&category.as_str()) {
        return Err(format!(
            "Categoría inválida: {}. Válidas: {:?}",
            category, valid_categories
        ));
    }

    // Validar status
    let valid_statuses = ["ACTIVE", "RESOLVED", "COMPLETED"];
    if !valid_statuses.contains(&status.as_str()) {
        return Err(format!(
            "Status inválido: {}. Válidos: {:?}",
            status, valid_statuses
        ));
    }

    // Cifrar cada campo del payload con DataKey
    let mut encrypted_payload: HashMap<String, String> = HashMap::new();
    for (field_name, field_value) in &payload {
        let enc = crypto::encrypt_text(field_value, &data_key)?;
        let combined = format!("{}:{}", enc.ciphertext, enc.nonce);
        encrypted_payload.insert(field_name.clone(), combined);
    }

    let payload_json = serde_json::to_vec(&encrypted_payload)
        .map_err(|e| format!("Error serializando payload: {}", e))?;

    // FIRMA DIGITAL con la llave privada del usuario
    let sign_guard = signing_state.0.lock().unwrap();
    let private_key_bytes = sign_guard
        .as_ref()
        .ok_or("Error de Seguridad: No hay llave de firma en RAM. ¿Bóveda cerrada?".to_string())?;

    // id y created_at se generan ANTES de firmar: ambos entran en el material.
    let entry_id = Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now().to_rfc3339();
    // Mismo formato que producía `datetime('now')` de SQLite (UTC, sin offset)
    // para no romper el parsing del frontend.
    let created_at = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let hash = signed_hash(
        &category,
        subject_id,
        &title,
        &status,
        &timestamp,
        &author_id,
        &entry_id,
        &created_at,
        &payload_json,
    );
    let signature_hex = crypto::sign_hash(&hash, private_key_bytes)?;

    let entry_id_clone = entry_id.clone();

    with_conn(&db_state, |conn| {
        conn.execute(
            "INSERT INTO entries (id, category, subject_id, author_id, title, status, timestamp, payload, signature, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                entry_id,
                category,
                subject_id,
                author_id,
                title,
                status,
                timestamp,
                payload_json,
                signature_hex,
                created_at
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })?;

    println!("✅ [Entry] Nueva entry creada: {}", entry_id_clone);
    Ok(entry_id_clone)
}

/// Obtiene una entry por ID (con verificación de firma)
#[tauri::command]
pub fn get_entry(
    id: String,
    db_state: State<'_, DbState>,
    data_key_state: State<'_, DataKey>,
) -> Result<EntryRecord, String> {
    let data_key = get_data_key(&data_key_state)?;

    let row = with_conn(&db_state, |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT e.id, e.category, e.subject_id, e.author_id, e.title, e.status, e.timestamp,
                    e.payload, e.signature, e.created_at,
                    pp.full_name_ciphertext, pp.full_name_nonce
                 FROM entries e
                 LEFT JOIN professional_profiles pp ON pp.user_id = e.author_id
                 WHERE e.id = ?1",
            )
            .map_err(|e| e.to_string())?;

        let result = stmt
            .query_row(params![id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Vec<u8>>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, Option<String>>(10)?, // full_name_ciphertext
                    row.get::<_, Option<String>>(11)?, // full_name_nonce
                ))
            })
            .map_err(|e| e.to_string())?;

        Ok(result)
    })?;

    let (
        id,
        category,
        subject_id,
        author_id,
        title,
        status,
        timestamp,
        payload_blob,
        signature_hex,
        created_at,
        name_ct,
        name_nonce,
    ) = row;

    // Descifrar nombre del autor
    let author_name = match (name_ct, name_nonce) {
        (Some(ct), Some(nonce)) if !ct.is_empty() => {
            let enc = crypto::EncryptedData {
                ciphertext: ct,
                nonce,
            };
            crypto::decrypt_text(&enc, &data_key)
                .unwrap_or_else(|_| "Autor desconocido".to_string())
        }
        _ => "Autor desconocido".to_string(),
    };

    let payload = decrypt_payload(&payload_blob, &data_key)?;

    // Verificar firma
    let hash = signed_hash(
        &category,
        subject_id,
        &title,
        &status,
        &timestamp,
        &author_id,
        &id,
        &created_at,
        &payload_blob,
    );

    // Obtener la llave pública del autor vigente en la fecha de la entry
    let pub_key_result = with_conn(&db_state, |conn| {
        public_key_at(conn, &author_id, &timestamp)
    });

    let is_verified = match pub_key_result {
        Ok(pk) => crypto::verify_signature(&hash, &signature_hex, &pk).unwrap_or(false),
        Err(_) => false,
    };

    Ok(EntryRecord {
        id,
        category,
        subject_id,
        author_id,
        author_name,
        title,
        status,
        timestamp,
        payload,
        signature: signature_hex,
        is_verified,
        created_at,
    })
}

/// Obtiene entries de un paciente con paginación
#[tauri::command]
pub fn get_entries_by_subject(
    subject_id: i64,
    limit: i64,
    offset: i64,
    db_state: State<'_, DbState>,
    data_key_state: State<'_, DataKey>,
) -> Result<Vec<EntryRecord>, String> {
    let data_key = get_data_key(&data_key_state)?;

    let rows = with_conn(&db_state, |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT e.id, e.category, e.subject_id, e.author_id, e.title, e.status, e.timestamp,
                    e.payload, e.signature, e.created_at,
                    pp.full_name_ciphertext, pp.full_name_nonce
                 FROM entries e
                 LEFT JOIN professional_profiles pp ON pp.user_id = e.author_id
                 WHERE e.subject_id = ?1
                 ORDER BY e.timestamp DESC
                 LIMIT ?2 OFFSET ?3",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![subject_id, limit, offset], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Vec<u8>>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                ))
            })
            .map_err(|e| e.to_string())?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| e.to_string())?);
        }
        Ok(results)
    })?;

    let mut entries = Vec::new();
    for (
        id,
        category,
        subject_id,
        author_id,
        title,
        status,
        timestamp,
        payload_blob,
        signature_hex,
        created_at,
        name_ct,
        name_nonce,
    ) in rows
    {
        // Descifrar nombre del autor
        let author_name = match (name_ct, name_nonce) {
            (Some(ct), Some(nonce)) if !ct.is_empty() => {
                let enc = crypto::EncryptedData {
                    ciphertext: ct,
                    nonce,
                };
                crypto::decrypt_text(&enc, &data_key)
                    .unwrap_or_else(|_| "Autor desconocido".to_string())
            }
            _ => "Autor desconocido".to_string(),
        };

        let payload = decrypt_payload(&payload_blob, &data_key)?;
        let hash = signed_hash(
            &category,
            subject_id,
            &title,
            &status,
            &timestamp,
            &author_id,
            &id,
            &created_at,
            &payload_blob,
        );

        let pub_key_result = with_conn(&db_state, |conn| {
            public_key_at(conn, &author_id, &timestamp)
        });

        let is_verified = match pub_key_result {
            Ok(pk) => crypto::verify_signature(&hash, &signature_hex, &pk).unwrap_or(false),
            Err(_) => false,
        };

        entries.push(EntryRecord {
            id,
            category,
            subject_id,
            author_id,
            author_name,
            title,
            status,
            timestamp,
            payload,
            signature: signature_hex,
            is_verified,
            created_at,
        });
    }

    Ok(entries)
}

/// Obtiene entries filtradas por categoría con paginación
#[tauri::command]
pub fn get_entries_by_category(
    subject_id: i64,
    category: String,
    limit: i64,
    offset: i64,
    db_state: State<'_, DbState>,
    data_key_state: State<'_, DataKey>,
) -> Result<Vec<EntryRecord>, String> {
    let data_key = get_data_key(&data_key_state)?;

    let rows = with_conn(&db_state, |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT e.id, e.category, e.subject_id, e.author_id, e.title, e.status, e.timestamp,
                    e.payload, e.signature, e.created_at,
                    pp.full_name_ciphertext, pp.full_name_nonce
                 FROM entries e
                 LEFT JOIN professional_profiles pp ON pp.user_id = e.author_id
                 WHERE e.subject_id = ?1 AND e.category = ?2
                 ORDER BY e.timestamp DESC
                 LIMIT ?3 OFFSET ?4",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![subject_id, category, limit, offset], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Vec<u8>>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, Option<String>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                ))
            })
            .map_err(|e| e.to_string())?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| e.to_string())?);
        }
        Ok(results)
    })?;

    let mut entries = Vec::new();
    for (
        id,
        category,
        subject_id,
        author_id,
        title,
        status,
        timestamp,
        payload_blob,
        signature_hex,
        created_at,
        name_ct,
        name_nonce,
    ) in rows
    {
        // Descifrar nombre del autor
        let author_name = match (name_ct, name_nonce) {
            (Some(ct), Some(nonce)) if !ct.is_empty() => {
                let enc = crypto::EncryptedData {
                    ciphertext: ct,
                    nonce,
                };
                crypto::decrypt_text(&enc, &data_key)
                    .unwrap_or_else(|_| "Autor desconocido".to_string())
            }
            _ => "Autor desconocido".to_string(),
        };

        let payload = decrypt_payload(&payload_blob, &data_key)?;
        let hash = signed_hash(
            &category,
            subject_id,
            &title,
            &status,
            &timestamp,
            &author_id,
            &id,
            &created_at,
            &payload_blob,
        );

        let pub_key_result = with_conn(&db_state, |conn| {
            public_key_at(conn, &author_id, &timestamp)
        });

        let is_verified = match pub_key_result {
            Ok(pk) => crypto::verify_signature(&hash, &signature_hex, &pk).unwrap_or(false),
            Err(_) => false,
        };

        entries.push(EntryRecord {
            id,
            category,
            subject_id,
            author_id,
            author_name,
            title,
            status,
            timestamp,
            payload,
            signature: signature_hex,
            is_verified,
            created_at,
        });
    }

    Ok(entries)
}

/// Busca entries por título (búsqueda parcial, case-insensitive)
#[tauri::command]
pub fn search_entries(
    subject_id: i64,
    query: String,
    category: Option<String>,
    limit: i64,
    offset: i64,
    db_state: State<'_, DbState>,
    data_key_state: State<'_, DataKey>,
) -> Result<Vec<EntryRecord>, String> {
    let data_key = get_data_key(&data_key_state)?;
    let search_pattern = format!("%{}%", query.trim().to_lowercase());

    type RowTuple = (
        String,
        String,
        i64,
        String,
        String,
        String,
        String,
        Vec<u8>,
        String,
        String,
        Option<String>,
        Option<String>,
    );

    let rows: Vec<RowTuple> = with_conn(&db_state, |conn| {
        let (sql, n_params) = match &category {
            Some(_) => (
                "SELECT e.id, e.category, e.subject_id, e.author_id, e.title, e.status, e.timestamp,
                    e.payload, e.signature, e.created_at,
                    pp.full_name_ciphertext, pp.full_name_nonce
                 FROM entries e
                 LEFT JOIN professional_profiles pp ON pp.user_id = e.author_id
                 WHERE e.subject_id = ?1 AND e.category = ?2
                    AND LOWER(e.title) LIKE ?3
                 ORDER BY e.timestamp DESC
                 LIMIT ?4 OFFSET ?5",
                5,
            ),
            None => (
                "SELECT e.id, e.category, e.subject_id, e.author_id, e.title, e.status, e.timestamp,
                    e.payload, e.signature, e.created_at,
                    pp.full_name_ciphertext, pp.full_name_nonce
                 FROM entries e
                 LEFT JOIN professional_profiles pp ON pp.user_id = e.author_id
                 WHERE e.subject_id = ?1
                    AND LOWER(e.title) LIKE ?2
                 ORDER BY e.timestamp DESC
                 LIMIT ?3 OFFSET ?4",
                4,
            ),
        };

        let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;

        let row_mapper = |row: &rusqlite::Row| -> rusqlite::Result<RowTuple> {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get(8)?,
                row.get(9)?,
                row.get(10)?,
                row.get(11)?,
            ))
        };

        let mut results = Vec::new();
        if n_params == 5 {
            let cat = category.clone().unwrap_or_default();
            let rows_iter = stmt
                .query_map(
                    params![subject_id, cat, search_pattern, limit, offset],
                    row_mapper,
                )
                .map_err(|e| e.to_string())?;
            for row in rows_iter {
                results.push(row.map_err(|e| e.to_string())?);
            }
        } else {
            let rows_iter = stmt
                .query_map(params![subject_id, search_pattern, limit, offset], row_mapper)
                .map_err(|e| e.to_string())?;
            for row in rows_iter {
                results.push(row.map_err(|e| e.to_string())?);
            }
        }
        Ok(results)
    })?;

    let mut entries = Vec::new();
    for (
        id,
        category,
        subject_id,
        author_id,
        title,
        status,
        timestamp,
        payload_blob,
        signature_hex,
        created_at,
        name_ct,
        name_nonce,
    ) in rows
    {
        let author_name = match (name_ct, name_nonce) {
            (Some(ct), Some(nonce)) if !ct.is_empty() => {
                let enc = crypto::EncryptedData {
                    ciphertext: ct,
                    nonce,
                };
                crypto::decrypt_text(&enc, &data_key)
                    .unwrap_or_else(|_| "Autor desconocido".to_string())
            }
            _ => "Autor desconocido".to_string(),
        };

        let payload = decrypt_payload(&payload_blob, &data_key)?;
        let hash = signed_hash(
            &category,
            subject_id,
            &title,
            &status,
            &timestamp,
            &author_id,
            &id,
            &created_at,
            &payload_blob,
        );

        let pub_key_result = with_conn(&db_state, |conn| {
            public_key_at(conn, &author_id, &timestamp)
        });

        let is_verified = match pub_key_result {
            Ok(pk) => crypto::verify_signature(&hash, &signature_hex, &pk).unwrap_or(false),
            Err(_) => false,
        };

        entries.push(EntryRecord {
            id,
            category,
            subject_id,
            author_id,
            author_name,
            title,
            status,
            timestamp,
            payload,
            signature: signature_hex,
            is_verified,
            created_at,
        });
    }

    Ok(entries)
}

/// Material canónico que se firma/verifica con Ed25519.
///
/// Se serializa como JSON (campos en orden de declaración → determinista), lo
/// que elimina la ambigüedad de re-partición del viejo formato concatenado con
/// `|`: un title que contiene el delimitador ya no puede desplazar la frontera
/// con los campos vecinos. Cubre todos los atributos de la fila, no solo la
/// metadata clínica: `id`, `author_id` y `created_at` también quedan atados a
/// la firma.
///
/// `payload_hex` son los BYTES CRUDOS persistidos (JSON del mapa cifrado) en
/// hex: cualquier re-escritura del payload —incluso re-cifrado con la misma
/// DataKey— rompe la firma. Nunca se re-serializa el `HashMap` del payload
/// (su orden de claves no está garantizado).
#[derive(Serialize)]
struct SignedMaterial<'a> {
    category: &'a str,
    subject_id: i64,
    title: &'a str,
    status: &'a str,
    timestamp: &'a str,
    author_id: &'a str,
    id: &'a str,
    created_at: &'a str,
    payload_hex: String,
}

/// Hash SHA-256 del material canónico. Es la ÚNICA fuente de verdad del
/// formato firmado: creación y las 4 rutas de verificación pasan por acá.
/// `#[allow]` porque agrupa todos los campos de la fila en una sola firma
/// atómica — dispersarlos en call sites es lo que introduce divergencias.
#[allow(clippy::too_many_arguments)]
fn signed_hash(
    category: &str,
    subject_id: i64,
    title: &str,
    status: &str,
    timestamp: &str,
    author_id: &str,
    id: &str,
    created_at: &str,
    payload_blob: &[u8],
) -> [u8; 32] {
    let material = SignedMaterial {
        category,
        subject_id,
        title,
        status,
        timestamp,
        author_id,
        id,
        created_at,
        payload_hex: hex::encode(payload_blob),
    };
    // Serializar una struct de campos escalar no puede fallar en la práctica.
    let json = serde_json::to_string(&material)
        .expect("serialización de SignedMaterial no puede fallar");
    crypto::hash_document(&json)
}

/// Llave pública del autor vigente en `entry_timestamp`, según el histórico
/// `signing_keys`. Permite verificar entries firmadas antes de un reset de
/// contraseña o una rotación de llave. Fallback a `professional_profiles`
/// para registros previos al histórico.
fn public_key_at(
    conn: &rusqlite::Connection,
    author_id: &str,
    entry_timestamp: &str,
) -> Result<String, String> {
    let historical: Option<String> = conn
        .query_row(
            "SELECT public_key FROM signing_keys
             WHERE user_id = ?1 AND valid_from <= ?2 AND (valid_to IS NULL OR valid_to > ?2)
             ORDER BY valid_from DESC
             LIMIT 1",
            params![author_id, entry_timestamp],
            |r| r.get(0),
        )
        .ok();

    match historical {
        Some(pk) => Ok(pk),
        None => conn
            .query_row(
                "SELECT public_key FROM professional_profiles WHERE user_id = ?1",
                params![author_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string()),
    }
}

/// Descifra el payload de una entry
fn decrypt_payload(
    payload_blob: &[u8],
    data_key: &[u8; 32],
) -> Result<EntryPayload, String> {
    let encrypted_fields: HashMap<String, String> = serde_json::from_slice(payload_blob)
        .map_err(|e| format!("Error parseando payload cifrado: {}", e))?;

    let mut payload: EntryPayload = HashMap::new();
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
            crypto::decrypt_text(&enc, data_key)
                .map_err(|e| format!("Error descifrando campo {}: {}", field_name, e))?;

        payload.insert(field_name, decrypted);
    }

    Ok(payload)
}

// =======================================================================
// TESTS
// =======================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::database::init_db;
    use std::sync::Mutex;
    use std::thread;
    use std::time::Duration;
    use tauri::Manager;
    use zeroize::Zeroizing;

    const DATA_KEY: [u8; 32] = [7u8; 32];

    /// Entorno de prueba: DB temporal real con schema completo, app mock de
    /// Tauri con los 3 states gestionados (DbState, DataKey, SigningState).
    struct TestEnv {
        app: tauri::App<tauri::test::MockRuntime>,
        author_id: String,
        entity_id: i64,
        _dir: tempfile::TempDir,
    }

    fn setup_with(data_key_present: bool, signing_present: bool) -> TestEnv {
        let dir = tempfile::tempdir().expect("tempdir");
        let conn = init_db(dir.path().to_path_buf()).expect("init_db");

        let (signing_key, verifying_key) = crypto::generate_keypair();
        let private_bytes = signing_key.to_bytes();
        let public_hex = hex::encode(verifying_key.to_bytes());

        let author_id = "author-test-1".to_string();
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO users (id, username, password_hash, role, created_at)
             VALUES (?1, ?2, 'hash_test', 'medico', ?3)",
            params![author_id, "dra.test", now],
        )
        .expect("insert user");

        let name_enc = crypto::encrypt_text("Dra. Test", &DATA_KEY).expect("encrypt name");
        conn.execute(
            "INSERT INTO professional_profiles
                (user_id, full_name_ciphertext, full_name_nonce,
                 license_number_ciphertext, license_number_nonce,
                 specialty_ciphertext, specialty_nonce,
                 public_key, updated_at)
             VALUES (?1, ?2, ?3, '', '', '', '', ?4, ?5)",
            params![
                author_id,
                name_enc.ciphertext,
                name_enc.nonce,
                public_hex,
                now
            ],
        )
        .expect("insert profile");

        conn.execute(
            "INSERT INTO entities (entity_type, created_by_user_id, enc_data_blob, created_at)
             VALUES ('paciente', ?1, x'00', ?2)",
            params![author_id, now],
        )
        .expect("insert entity");
        let entity_id = conn.last_insert_rowid();

        let app = tauri::test::mock_app();
        app.manage(DbState(Mutex::new(Some(conn))));
        app.manage(DataKey(Mutex::new(if data_key_present {
            Some(Zeroizing::new(DATA_KEY))
        } else {
            None
        })));
        app.manage(SigningState(Mutex::new(if signing_present {
            Some(Zeroizing::new(private_bytes))
        } else {
            None
        })));

        TestEnv {
            app,
            author_id,
            entity_id,
            _dir: dir,
        }
    }

    fn setup() -> TestEnv {
        setup_with(true, true)
    }

    fn payload_sample() -> EntryPayload {
        let mut p = EntryPayload::new();
        p.insert("subjetivo".to_string(), "Dolor de cabeza".to_string());
        p.insert("objetivo".to_string(), "TA 120/80".to_string());
        p
    }

    fn create_with(
        env: &TestEnv,
        category: &str,
        status: &str,
        subject_id: i64,
        title: &str,
    ) -> Result<String, String> {
        create_entry(
            category.to_string(),
            subject_id,
            env.author_id.clone(),
            title.to_string(),
            status.to_string(),
            payload_sample(),
            env.app.state::<DbState>(),
            env.app.state::<DataKey>(),
            env.app.state::<SigningState>(),
        )
    }

    fn create(env: &TestEnv, subject_id: i64, title: &str) -> Result<String, String> {
        create_with(env, "SOAP_NOTE", "ACTIVE", subject_id, title)
    }

    fn get(env: &TestEnv, id: &str) -> Result<EntryRecord, String> {
        get_entry(id.to_string(), env.app.state::<DbState>(), env.app.state::<DataKey>())
    }

    fn list_subject(
        env: &TestEnv,
        subject_id: i64,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<EntryRecord>, String> {
        get_entries_by_subject(
            subject_id,
            limit,
            offset,
            env.app.state::<DbState>(),
            env.app.state::<DataKey>(),
        )
    }

    fn list_category(
        env: &TestEnv,
        subject_id: i64,
        category: &str,
    ) -> Result<Vec<EntryRecord>, String> {
        get_entries_by_category(
            subject_id,
            category.to_string(),
            50,
            0,
            env.app.state::<DbState>(),
            env.app.state::<DataKey>(),
        )
    }

    fn search(
        env: &TestEnv,
        subject_id: i64,
        query: &str,
        category: Option<&str>,
    ) -> Result<Vec<EntryRecord>, String> {
        search_entries(
            subject_id,
            query.to_string(),
            category.map(|c| c.to_string()),
            50,
            0,
            env.app.state::<DbState>(),
            env.app.state::<DataKey>(),
        )
    }

    fn insert_entity(env: &TestEnv) -> i64 {
        let db = env.app.state::<DbState>();
        with_conn(&db, |conn| {
            conn.execute(
                "INSERT INTO entities (entity_type, created_by_user_id, enc_data_blob, created_at)
                 VALUES ('paciente', ?1, x'00', ?2)",
                params![env.author_id, chrono::Utc::now().to_rfc3339()],
            )
            .map_err(|e| e.to_string())?;
            Ok(conn.last_insert_rowid())
        })
        .expect("insert entity")
    }

    // --- create_entry ---

    #[test]
    fn test_create_and_get_entry_roundtrip() {
        let env = setup();
        let id = create(&env, env.entity_id, "Control general").expect("create");

        let entry = get(&env, &id).expect("get");

        assert_eq!(entry.category, "SOAP_NOTE");
        assert_eq!(entry.subject_id, env.entity_id);
        assert_eq!(entry.author_id, env.author_id);
        assert_eq!(entry.title, "Control general");
        assert_eq!(entry.status, "ACTIVE");
        assert_eq!(entry.payload.len(), 2);
        assert_eq!(
            entry.payload.get("subjetivo").unwrap(),
            "Dolor de cabeza"
        );
        assert_eq!(entry.payload.get("objetivo").unwrap(), "TA 120/80");
        assert!(
            entry.is_verified,
            "la firma debió verificar con la pública registrada del autor"
        );
        assert_eq!(entry.author_name, "Dra. Test");
    }

    #[test]
    fn test_create_entry_persists_encrypted_payload() {
        let env = setup();
        let id = create_with(&env, "ALLERGY", "ACTIVE", env.entity_id, "Alergia penicilina")
            .expect("create");

        let raw: Vec<u8> = {
            let db = env.app.state::<DbState>();
            with_conn(&db, |conn| {
                conn.query_row(
                    "SELECT payload FROM entries WHERE id = ?1",
                    params![id],
                    |r| r.get::<_, Vec<u8>>(0),
                )
                .map_err(|e| e.to_string())
            })
            .expect("select payload")
        };

        let as_text = String::from_utf8(raw).expect("payload utf8");
        assert!(
            !as_text.contains("Dolor de cabeza"),
            "el payload en DB no debe contener texto plano"
        );

        let map: HashMap<String, String> = serde_json::from_str(&as_text).expect("json");
        assert_eq!(map.len(), 2);
        for value in map.values() {
            let parts: Vec<&str> = value.split(':').collect();
            assert_eq!(parts.len(), 2, "formato debe ser ciphertext:nonce");
            assert!(hex::decode(parts[0]).is_ok(), "ciphertext hex");
            assert!(hex::decode(parts[1]).is_ok(), "nonce hex");
        }
    }

    #[test]
    fn test_create_entry_rejects_invalid_category() {
        let env = setup();
        let err = create_with(&env, "NOTES", "ACTIVE", env.entity_id, "x").unwrap_err();
        assert!(err.contains("Categoría inválida"), "err: {}", err);
    }

    #[test]
    fn test_create_entry_rejects_invalid_status() {
        let env = setup();
        let err = create_with(&env, "SOAP_NOTE", "BORRADOR", env.entity_id, "x").unwrap_err();
        assert!(err.contains("Status inválido"), "err: {}", err);
    }

    #[test]
    fn test_create_entry_requires_data_key() {
        let env = setup_with(false, true);
        let err = create(&env, env.entity_id, "x").unwrap_err();
        assert!(err.contains("Data key no inicializada"), "err: {}", err);
    }

    #[test]
    fn test_create_entry_requires_signing_key() {
        let env = setup_with(true, false);
        let err = create(&env, env.entity_id, "x").unwrap_err();
        assert!(err.contains("llave de firma"), "err: {}", err);
    }

    // --- get_entry ---

    #[test]
    fn test_get_entry_not_found() {
        let env = setup();
        assert!(get(&env, "no-existe").is_err());
    }

    #[test]
    fn test_get_entry_tampered_title_breaks_signature() {
        let env = setup();
        let id = create(&env, env.entity_id, "Original").expect("create");

        let db = env.app.state::<DbState>();
        with_conn(&db, |conn| {
            conn.execute(
                "UPDATE entries SET title = 'Manipulado' WHERE id = ?1",
                params![id],
            )
            .map_err(|e| e.to_string())
        })
        .expect("update title");

        let entry = get(&env, &id).expect("get");
        assert_eq!(entry.title, "Manipulado");
        assert!(
            !entry.is_verified,
            "alterar el título debe romper la verificación de firma"
        );
    }

    #[test]
    fn test_get_entry_tampered_payload_breaks_signature() {
        let env = setup();
        let id = create(&env, env.entity_id, "Original").expect("create");

        // Re-cifrar un campo alterado con la MISMA DataKey: un atacante con la
        // DataKey puede producir un payload que descifra sin errores, pero la
        // firma debe romperse igual porque cubre los bytes persistidos.
        let enc = crypto::encrypt_text("Contenido manipulado", &DATA_KEY).expect("encrypt");
        let tampered = serde_json::json!({
            "subjetivo": format!("{}:{}", enc.ciphertext, enc.nonce),
            "objetivo": format!("{}:{}", enc.ciphertext, enc.nonce),
        });
        let blob = serde_json::to_vec(&tampered).expect("to_vec");

        let db = env.app.state::<DbState>();
        with_conn(&db, |conn| {
            conn.execute(
                "UPDATE entries SET payload = ?1 WHERE id = ?2",
                params![blob, id],
            )
            .map_err(|e| e.to_string())
        })
        .expect("update payload");

        let entry = get(&env, &id).expect("get");
        assert_eq!(
            entry.payload.get("subjetivo").unwrap(),
            "Contenido manipulado"
        );
        assert!(
            !entry.is_verified,
            "alterar el payload (aunque se descifre correctamente) debe romper la firma"
        );
    }

    #[test]
    fn test_get_entry_tampered_created_at_breaks_signature() {
        let env = setup();
        let id = create(&env, env.entity_id, "Original").expect("create");

        let db = env.app.state::<DbState>();
        with_conn(&db, |conn| {
            conn.execute(
                "UPDATE entries SET created_at = '1999-01-01 00:00:00' WHERE id = ?1",
                params![id],
            )
            .map_err(|e| e.to_string())
        })
        .expect("update created_at");

        let entry = get(&env, &id).expect("get");
        assert_eq!(entry.created_at, "1999-01-01 00:00:00");
        assert!(
            !entry.is_verified,
            "alterar created_at debe romper la firma (falla abierta antes de este cambio)"
        );
    }

    #[test]
    fn test_get_entry_tampered_id_breaks_signature() {
        let env = setup();
        let id = create(&env, env.entity_id, "Original").expect("create");

        // Re-identificar la fila: con el material viejo la firma seguía
        // validando y la entry aparecía "verificada" bajo otra identidad
        // (rompía la unión por id del sync).
        let db = env.app.state::<DbState>();
        with_conn(&db, |conn| {
            conn.execute(
                "UPDATE entries SET id = 'id-reidentificado' WHERE id = ?1",
                params![id],
            )
            .map_err(|e| e.to_string())
        })
        .expect("update id");

        let entry = get(&env, "id-reidentificado").expect("get");
        assert!(
            !entry.is_verified,
            "re-identificar la fila debe romper la firma"
        );
    }

    #[test]
    fn test_signed_hash_no_delimiter_ambiguity() {
        // Bajo el viejo formato `a|b|c` estas dos asignaciones producían la
        // MISMA cadena (title absorbía el pipe de status) → misma firma.
        // El JSON canónico las distingue.
        let h1 = signed_hash(
            "SOAP_NOTE", 1, "Nota|vieja", "ACTIVE", "t", "u", "i", "c", b"p",
        );
        let h2 = signed_hash(
            "SOAP_NOTE", 1, "Nota", "vieja|ACTIVE", "t", "u", "i", "c", b"p",
        );
        assert_ne!(
            h1, h2,
            "la frontera entre campos debe ser inequívoca sin importar pipes en los valores"
        );
    }

    #[test]
    fn test_signed_hash_covers_every_row_field() {
        // Cada campo de la fila debe alterar el hash individualmente: si uno
        // no está en el material, su alteración pasaría inadvertida.
        let base = |cat: &str, sid: i64, t: &str, st: &str, ts: &str, a: &str, id: &str, ca: &str, p: &[u8]| {
            signed_hash(cat, sid, t, st, ts, a, id, ca, p)
        };
        let reference = base("SOAP_NOTE", 1, "T", "ACTIVE", "ts", "au", "id", "ca", b"p");

        assert_ne!(reference, base("MEDICATION", 1, "T", "ACTIVE", "ts", "au", "id", "ca", b"p"));
        assert_ne!(reference, base("SOAP_NOTE", 2, "T", "ACTIVE", "ts", "au", "id", "ca", b"p"));
        assert_ne!(reference, base("SOAP_NOTE", 1, "X", "ACTIVE", "ts", "au", "id", "ca", b"p"));
        assert_ne!(reference, base("SOAP_NOTE", 1, "T", "RESOLVED", "ts", "au", "id", "ca", b"p"));
        assert_ne!(reference, base("SOAP_NOTE", 1, "T", "ACTIVE", "ts2", "au", "id", "ca", b"p"));
        assert_ne!(reference, base("SOAP_NOTE", 1, "T", "ACTIVE", "ts", "au2", "id", "ca", b"p"));
        assert_ne!(reference, base("SOAP_NOTE", 1, "T", "ACTIVE", "ts", "au", "id2", "ca", b"p"));
        assert_ne!(reference, base("SOAP_NOTE", 1, "T", "ACTIVE", "ts", "au", "id", "ca2", b"p"));
        assert_ne!(reference, base("SOAP_NOTE", 1, "T", "ACTIVE", "ts", "au", "id", "ca", b"q"));
    }

    #[test]
    fn test_get_entry_with_wrong_data_key_fails() {
        let env = setup();
        let id = create(&env, env.entity_id, "Secreta").expect("create");

        {
            let dk = env.app.state::<DataKey>();
            let mut guard = dk.0.lock().unwrap();
            *guard = Some(Zeroizing::new([0u8; 32]));
        }

        let err = get(&env, &id).unwrap_err();
        assert!(err.contains("descifrando"), "err: {}", err);
    }

    #[test]
    fn test_entry_without_profile_shows_unknown_author() {
        let env = setup();

        let db = env.app.state::<DbState>();
        with_conn(&db, |conn| {
            conn.execute(
                "INSERT INTO users (id, username, password_hash, role, created_at)
                 VALUES ('author-2', 'sin.perfil', 'hash', 'medico', ?1)",
                params![chrono::Utc::now().to_rfc3339()],
            )
            .map_err(|e| e.to_string())
        })
        .expect("insert user");

        let id = create_entry(
            "SOAP_NOTE".to_string(),
            env.entity_id,
            "author-2".to_string(),
            "Nota sin perfil".to_string(),
            "ACTIVE".to_string(),
            payload_sample(),
            env.app.state::<DbState>(),
            env.app.state::<DataKey>(),
            env.app.state::<SigningState>(),
        )
        .expect("create");

        let entry = get(&env, &id).expect("get");
        assert_eq!(entry.author_name, "Autor desconocido");
        assert!(!entry.is_verified, "sin pública registrada no verifica");
    }

    // --- listados y paginación ---

    #[test]
    fn test_get_entries_by_subject_pagination_and_isolation() {
        let env = setup();

        let a = create(&env, env.entity_id, "Primera").expect("a");
        thread::sleep(Duration::from_millis(5));
        let b = create(&env, env.entity_id, "Segunda").expect("b");
        thread::sleep(Duration::from_millis(5));
        let c = create(&env, env.entity_id, "Tercera").expect("c");

        let other_entity = insert_entity(&env);
        let d = create(&env, other_entity, "De otro paciente").expect("d");

        let page1 = list_subject(&env, env.entity_id, 2, 0).expect("page1");
        let titles1: Vec<&str> = page1.iter().map(|e| e.title.as_str()).collect();
        assert_eq!(titles1, vec!["Tercera", "Segunda"], "orden DESC por timestamp");

        let page2 = list_subject(&env, env.entity_id, 2, 2).expect("page2");
        assert_eq!(page2.len(), 1);
        assert_eq!(page2[0].id, a);

        let all = list_subject(&env, env.entity_id, 10, 0).expect("all");
        assert_eq!(all.len(), 3);
        let ids: Vec<&str> = all.iter().map(|e| e.id.as_str()).collect();
        assert!(ids.contains(&b.as_str()) && ids.contains(&c.as_str()));

        let other = list_subject(&env, other_entity, 10, 0).expect("other");
        assert_eq!(other.len(), 1);
        assert_eq!(other[0].id, d);
    }

    #[test]
    fn test_get_entries_by_category_filters() {
        let env = setup();

        let soap = create(&env, env.entity_id, "Nota clínica").expect("soap");
        thread::sleep(Duration::from_millis(5));
        let allergy =
            create_with(&env, "ALLERGY", "ACTIVE", env.entity_id, "Alergia marisco").expect("alg");
        thread::sleep(Duration::from_millis(5));
        let med =
            create_with(&env, "MEDICATION", "COMPLETED", env.entity_id, "Analgesico").expect("med");

        let allergies = list_category(&env, env.entity_id, "ALLERGY").expect("allergies");
        assert_eq!(allergies.len(), 1);
        assert_eq!(allergies[0].id, allergy);
        assert_eq!(allergies[0].category, "ALLERGY");

        let soap_only = list_category(&env, env.entity_id, "SOAP_NOTE").expect("soap only");
        assert_eq!(soap_only.len(), 1);
        assert_eq!(soap_only[0].id, soap);

        let meds = list_category(&env, env.entity_id, "MEDICATION").expect("meds");
        assert_eq!(meds.len(), 1);
        assert_eq!(meds[0].id, med);
        assert_eq!(meds[0].status, "COMPLETED");
    }

    // --- búsqueda ---

    #[test]
    fn test_search_entries_case_insensitive_and_scoped() {
        let env = setup();

        let e1 = create(&env, env.entity_id, "Control mensual").expect("e1");
        thread::sleep(Duration::from_millis(5));
        let e2 =
            create_with(&env, "ALLERGY", "ACTIVE", env.entity_id, "Alergia a penicilina").expect("e2");

        let other_entity = insert_entity(&env);
        let e3 = create(&env, other_entity, "Control inicial").expect("e3");

        // case-insensitive + parcial
        let hits = search(&env, env.entity_id, "CONTROL", None).expect("control");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, e1);

        let hits = search(&env, env.entity_id, "penic", None).expect("penic");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, e2);

        // con filtro de categoría
        let hits = search(&env, env.entity_id, "control", Some("ALLERGY")).expect("cat miss");
        assert!(hits.is_empty(), "control es SOAP, no ALLERGY");

        let hits = search(&env, env.entity_id, "control", Some("SOAP_NOTE")).expect("cat hit");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, e1);

        // aislamiento por subject
        let hits = search(&env, other_entity, "control", None).expect("other subject");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, e3);

        // sin coincidencias
        let hits = search(&env, env.entity_id, "zzz-no-existe", None).expect("empty");
        assert!(hits.is_empty());
    }

    // --- decrypt_payload (unidad) ---

    #[test]
    fn test_decrypt_payload_roundtrip() {
        let enc = crypto::encrypt_text("valor sensible", &DATA_KEY).expect("encrypt");
        let map = serde_json::json!({ "campo": format!("{}:{}", enc.ciphertext, enc.nonce) });
        let blob = serde_json::to_vec(&map).expect("to_vec");

        let out = decrypt_payload(&blob, &DATA_KEY).expect("decrypt");
        assert_eq!(out.get("campo").unwrap(), "valor sensible");
    }

    #[test]
    fn test_decrypt_payload_rejects_bad_format() {
        let map = serde_json::json!({ "campo": "sin_delimitador" });
        let blob = serde_json::to_vec(&map).expect("to_vec");
        let err = decrypt_payload(&blob, &DATA_KEY).unwrap_err();
        assert!(err.contains("Formato inválido"), "err: {}", err);

        let err = decrypt_payload(b"no-es-json", &DATA_KEY).unwrap_err();
        assert!(err.contains("parseando"), "err: {}", err);
    }

    // --- Histórico de claves de firma (public_key_at) ---

    fn insert_user_with_profile(conn: &rusqlite::Connection, user_id: &str, public_key: &str) {
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO users (id, username, password_hash, role, created_at)
             VALUES (?1, ?2, 'hash', 'medico', ?3)",
            params![user_id, user_id, now],
        )
        .expect("insert user");
        conn.execute(
            "INSERT INTO professional_profiles
                (user_id, full_name_ciphertext, full_name_nonce,
                 license_number_ciphertext, license_number_nonce,
                 specialty_ciphertext, specialty_nonce, public_key, updated_at)
             VALUES (?1, '', '', '', '', '', '', ?2, ?3)",
            params![user_id, public_key, now],
        )
        .expect("insert profile");
    }

    #[test]
    fn test_public_key_at_returns_key_vigente_en_el_timestamp() {
        let dir = tempfile::tempdir().unwrap();
        let conn = init_db(dir.path().to_path_buf()).unwrap();

        insert_user_with_profile(&conn, "u-hist", "PK_PROFILE_ACTUAL");

        // Historial: clave vieja hasta 2026-01-01, nueva desde entonces.
        conn.execute(
            "INSERT INTO signing_keys (user_id, public_key, valid_from, valid_to)
             VALUES ('u-hist', 'PK_VIEJA', '2025-01-01T00:00:00+00:00', '2026-01-01T00:00:00+00:00')",
            [],
        )
        .expect("insert key vieja");
        conn.execute(
            "INSERT INTO signing_keys (user_id, public_key, valid_from)
             VALUES ('u-hist', 'PK_NUEVA', '2026-01-01T00:00:00+00:00')",
            [],
        )
        .expect("insert key nueva");

        // Entry firmada en 2025 → la clave de esa época
        assert_eq!(
            public_key_at(&conn, "u-hist", "2025-06-15T10:00:00+00:00").unwrap(),
            "PK_VIEJA"
        );

        // Entry firmada hoy → la clave vigente
        assert_eq!(
            public_key_at(&conn, "u-hist", "2026-10-03T10:00:00+00:00").unwrap(),
            "PK_NUEVA"
        );

        // Exactamente en el instante del cierre → ya cuenta la nueva
        assert_eq!(
            public_key_at(&conn, "u-hist", "2026-01-01T00:00:00+00:00").unwrap(),
            "PK_NUEVA"
        );
    }

    #[test]
    fn test_public_key_at_falls_back_a_profile_sin_historico() {
        let dir = tempfile::tempdir().unwrap();
        let conn = init_db(dir.path().to_path_buf()).unwrap();

        insert_user_with_profile(&conn, "u-nuevo", "PK_PROFILE");

        // Sin filas en signing_keys → se usa professional_profiles
        assert_eq!(
            public_key_at(&conn, "u-nuevo", "2026-10-03T10:00:00+00:00").unwrap(),
            "PK_PROFILE"
        );
    }

    #[test]
    fn test_public_key_at_sin_usuario_devuelve_error() {
        let dir = tempfile::tempdir().unwrap();
        let conn = init_db(dir.path().to_path_buf()).unwrap();

        assert!(public_key_at(&conn, "no-existe", "2026-10-03T10:00:00+00:00").is_err());
    }
}
