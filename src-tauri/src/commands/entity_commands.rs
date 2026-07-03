// ============================================================
// COMANDOS EXPERIMENTALES — Template v1
// Conviven con patient_commands.rs (legacy).
// ============================================================

use crate::config_schema::SchemaConfig;
use crate::crypto;
use crate::lib_types::{CryptoState, DbState};
use rusqlite::params;
use serde::Serialize;
use std::collections::HashMap;
use tauri::State;

use super::get_key;

pub type EntityData = HashMap<String, String>;

#[derive(Serialize)]
pub struct EntityCreated {
    pub id: i64,
    pub entity_type: String,
    pub blind_index_hex: String,
}

#[derive(Serialize)]
pub struct EntityRecord {
    pub id: i64,
    pub entity_type: String,
    pub data: EntityData,
    pub created_at: String,
}

#[tauri::command]
pub fn create_entity(
    entity_type: String,
    data: EntityData,
    db_state: State<'_, DbState>,
    crypto_state: State<'_, CryptoState>,
    schema: State<'_, SchemaConfig>,
) -> Result<EntityCreated, String> {
    let master_key = get_key(&crypto_state)?;

    // Validación dinámica contra schema.json
    schema.validate_entity_data(&entity_type, &data)?;

    // Obtener campo de blind index dinámicamente
    let blind_index_field = schema.get_blind_index_field(&entity_type)?;
    let dni = data
        .get(&blind_index_field)
        .ok_or_else(|| format!("Campo de blind index faltante: {}", blind_index_field))?
        .trim()
        .to_lowercase();

    let blind_index = crypto::generate_blind_index(&dni);
    let blind_index_hex = blind_index.clone();

    // Cifrar cada campo
    let mut encrypted_fields: HashMap<String, String> = HashMap::new();
    for (field_name, field_value) in &data {
        let enc = crypto::encrypt_text(field_value, &master_key)?;
        let combined = format!("{}:{}", enc.ciphertext, enc.nonce);
        encrypted_fields.insert(field_name.clone(), combined);
    }

    let enc_data_json =
        serde_json::to_vec(&encrypted_fields).map_err(|e| format!("Error serializando: {}", e))?;

    let entity_id = super::with_conn(&db_state, |conn| {
        conn.execute(
            "INSERT INTO entities (entity_type, created_by_user_id, blind_index, enc_data_blob, created_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            params![entity_type, 1i64, blind_index, enc_data_json],
        ).map_err(|e| e.to_string())?;
        Ok(conn.last_insert_rowid())
    })?;

    Ok(EntityCreated {
        id: entity_id,
        entity_type,
        blind_index_hex,
    })
}

#[tauri::command]
pub fn get_entity(
    id: i64,
    db_state: State<'_, DbState>,
    crypto_state: State<'_, CryptoState>,
) -> Result<EntityRecord, String> {
    let master_key = get_key(&crypto_state)?;

    let record = super::with_conn(&db_state, |conn| {
        let mut stmt = conn
            .prepare("SELECT entity_type, enc_data_blob, created_at FROM entities WHERE id = ?1")
            .map_err(|e| e.to_string())?;

        let row = stmt
            .query_row(params![id], |row| {
                let entity_type: String = row.get(0)?;
                let enc_data_blob: Vec<u8> = row.get(1)?;
                let created_at: String = row.get(2)?;
                Ok((entity_type, enc_data_blob, created_at))
            })
            .map_err(|e| e.to_string())?;

        Ok(row)
    })?;

    let (entity_type, enc_data_blob, created_at) = record;

    // Parsear JSON de campos cifrados
    let encrypted_fields: HashMap<String, String> = serde_json::from_slice(&enc_data_blob)
        .map_err(|e| format!("Error parseando blob: {}", e))?;

    // Descifrar cada campo
    let mut data: EntityData = HashMap::new();
    for (field_name, combined) in encrypted_fields {
        // Separar ciphertext:nonce por el ':'
        let parts: Vec<&str> = combined.split(':').collect();
        if parts.len() != 2 {
            return Err(format!("Formato inválido en campo {}", field_name));
        }

        let enc = crypto::EncryptedData {
            ciphertext: parts[0].to_string(),
            nonce: parts[1].to_string(),
        };

        let decrypted = crypto::decrypt_text(&enc, &master_key)
            .map_err(|e| format!("Error descifrando campo {}: {}", field_name, e))?;

        data.insert(field_name, decrypted);
    }

    Ok(EntityRecord {
        id,
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

    let found_id = super::with_conn(&db_state, |conn| {
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
