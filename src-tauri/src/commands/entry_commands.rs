use crate::crypto;
use crate::lib_types::{DataKey, DbState, SigningState};
use rusqlite::params;
use serde::Serialize;
use std::collections::HashMap;
use tauri::State;
use uuid::Uuid;

use super::{get_data_key, with_conn};

pub type EntryPayload = HashMap<String, String>;

#[derive(Serialize)]
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

    // Construir payload para firma: category|subject_id|title|status|timestamp
    let timestamp = chrono::Utc::now().to_rfc3339();
    let signature_payload = format!(
        "{}|{}|{}|{}|{}",
        category, subject_id, title, status, timestamp
    );
    let hash = crypto::hash_document(&signature_payload);
    let signature_hex = crypto::sign_hash(&hash, private_key_bytes)?;

    let entry_id = Uuid::new_v4().to_string();
    let entry_id_clone = entry_id.clone();

    with_conn(&db_state, |conn| {
        conn.execute(
            "INSERT INTO entries (id, category, subject_id, author_id, title, status, timestamp, payload, signature, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, datetime('now'))",
            params![
                entry_id,
                category,
                subject_id,
                author_id,
                title,
                status,
                timestamp,
                payload_json,
                signature_hex
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
    let signature_payload = format!(
        "{}|{}|{}|{}|{}",
        category, subject_id, title, status, timestamp
    );
    let hash = crypto::hash_document(&signature_payload);

    // Obtener llave pública del autor para verificar firma
    let pub_key_result: Result<String, _> = with_conn(&db_state, |conn| {
        conn.query_row(
            "SELECT public_key FROM professional_profiles WHERE user_id = ?1",
            params![&author_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())
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
        let signature_payload = format!(
            "{}|{}|{}|{}|{}",
            category, subject_id, title, status, timestamp
        );
        let hash = crypto::hash_document(&signature_payload);

        let pub_key_result: Result<String, _> = with_conn(&db_state, |conn| {
            conn.query_row(
                "SELECT public_key FROM professional_profiles WHERE user_id = ?1",
                params![&author_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())
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
        let signature_payload = format!(
            "{}|{}|{}|{}|{}",
            category, subject_id, title, status, timestamp
        );
        let hash = crypto::hash_document(&signature_payload);

        let pub_key_result: Result<String, _> = with_conn(&db_state, |conn| {
            conn.query_row(
                "SELECT public_key FROM professional_profiles WHERE user_id = ?1",
                params![&author_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())
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
        let signature_payload = format!(
            "{}|{}|{}|{}|{}",
            category, subject_id, title, status, timestamp
        );
        let hash = crypto::hash_document(&signature_payload);

        let pub_key_result: Result<String, _> = with_conn(&db_state, |conn| {
            conn.query_row(
                "SELECT public_key FROM professional_profiles WHERE user_id = ?1",
                params![&author_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())
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
