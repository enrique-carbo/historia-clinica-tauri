use crate::config_schema::SchemaConfig;
use crate::crypto;
use crate::lib_types::{DataKey, DbState}; // ← Cambiar CryptoState por DataKey
use rusqlite::params;
use serde::Serialize;
use std::collections::HashMap;
use tauri::State;
use uuid::Uuid;

use super::{get_data_key, with_conn}; // ← Usar get_data_key

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
    user_id: String,
    db_state: State<'_, DbState>,
    data_key_state: State<'_, DataKey>, // ← Cambiar CryptoState por DataKey
    schema: State<'_, SchemaConfig>,
) -> Result<EntityCreated, String> {
    let data_key = get_data_key(&data_key_state)?; // ← Usar data_key
    schema.validate_entity_data(&entity_type, &data)?;

    let blind_index_field = schema.get_blind_index_field(&entity_type)?;
    let dni = data
        .get(&blind_index_field)
        .ok_or_else(|| format!("Campo de blind index faltante: {}", blind_index_field))?
        .trim()
        .to_lowercase();

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
            params![external_id, entity_type, user_id, blind_index, enc_data_json],
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
    let dni_normalized = dni.trim().to_lowercase();
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
        for (_, value) in &data {
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
