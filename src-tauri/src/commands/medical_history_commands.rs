use crate::config_schema::MedicalHistorySchemaConfig;
use crate::crypto;
use crate::lib_types::{DataKey, DbState};
use rusqlite::params;
use serde::Serialize;
use std::collections::HashMap;
use tauri::State;
use uuid::Uuid;

use super::{get_data_key, with_conn};

pub type MedicalHistoryData = HashMap<String, String>;

#[derive(Serialize)]
pub struct MedicalHistoryRecord {
    pub id: i64,
    pub external_id: String,
    pub entity_id: i64,
    pub data: MedicalHistoryData,
    pub updated_at: String,
}

#[tauri::command]
pub fn upsert_medical_history(
    entity_id: i64,
    data: MedicalHistoryData,
    user_id: String,
    db_state: State<'_, DbState>,
    data_key_state: State<'_, DataKey>,
    schema: State<'_, MedicalHistorySchemaConfig>,
) -> Result<MedicalHistoryRecord, String> {
    let data_key = get_data_key(&data_key_state)?;
    schema.validate(&data)?;

    // Cifrar cada campo individualmente (mismo patrón que notes/entities)
    let mut encrypted_fields: HashMap<String, String> = HashMap::new();
    for (field_name, field_value) in &data {
        let enc = crypto::encrypt_text(field_value, &data_key)?;
        encrypted_fields.insert(
            field_name.clone(),
            format!("{}:{}", enc.ciphertext, enc.nonce),
        );
    }
    let enc_blob = serde_json::to_vec(&encrypted_fields)
        .map_err(|e| format!("Error serializando campos: {}", e))?;

    let external_id = Uuid::new_v4().to_string();
    let updated_at = chrono::Utc::now().to_rfc3339();

    with_conn(&db_state, |conn| {
        conn.execute(
            "INSERT INTO medical_history (external_id, entity_id, created_by_user_id, enc_fields, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(entity_id) DO UPDATE SET
               enc_fields = excluded.enc_fields,
               updated_at = excluded.updated_at",
            params![external_id, entity_id, user_id, enc_blob, updated_at],
        ).map_err(|e| format!("Error upsert medical_history: {}", e))?;

        let id: i64 = conn
            .query_row(
                "SELECT id FROM medical_history WHERE entity_id = ?1",
                params![entity_id],
                |r| r.get(0),
            )
            .map_err(|e| format!("Error leyendo id tras upsert: {}", e))?;

        Ok(MedicalHistoryRecord {
            id,
            external_id,
            entity_id,
            data,
            updated_at,
        })
    })
}

#[tauri::command]
pub fn get_medical_history(
    entity_id: i64,
    db_state: State<'_, DbState>,
    data_key_state: State<'_, DataKey>,
) -> Result<Option<MedicalHistoryRecord>, String> {
    let data_key = get_data_key(&data_key_state)?;

    with_conn(&db_state, |conn| {
        let result = conn.query_row(
            "SELECT id, external_id, entity_id, enc_fields, updated_at
             FROM medical_history WHERE entity_id = ?1",
            params![entity_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        );

        match result {
            Ok((id, external_id, entity_id, enc_blob, updated_at)) => {
                let encrypted_fields: HashMap<String, String> =
                    serde_json::from_slice(&enc_blob)
                        .map_err(|e| format!("Error deserializando campos: {}", e))?;

                let mut data = HashMap::new();
                for (field_name, combined) in encrypted_fields {
                    let parts: Vec<&str> = combined.splitn(2, ':').collect();
                    if parts.len() != 2 {
                        return Err(format!("Formato inválido en campo {}", field_name));
                    }
                    let enc = crypto::EncryptedData {
                        ciphertext: parts[0].to_string(),
                        nonce: parts[1].to_string(),
                    };
                    let decrypted = crypto::decrypt_text(&enc, &data_key)
                        .map_err(|e| format!("Error descifrando campo {}: {}", field_name, e))?;
                    data.insert(field_name, decrypted);
                }

                Ok(Some(MedicalHistoryRecord {
                    id,
                    external_id,
                    entity_id,
                    data,
                    updated_at,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("Error consultando medical_history: {}", e)),
        }
    })
}
