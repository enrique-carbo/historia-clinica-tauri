use crate::config_schema::NoteTemplatesConfig;
use crate::crypto;
use crate::lib_types::{DataKey, DbState, SigningState};
use rusqlite::params;
use serde::Serialize;
use std::collections::HashMap;
use tauri::State;
use uuid::Uuid;

use super::{get_data_key, with_conn};

pub type NoteFields = HashMap<String, String>;

#[derive(Serialize)]
pub struct NoteRecord {
    pub id: i64,
    pub external_id: Option<String>,
    pub entity_id: i64,
    pub template_id: String,
    pub fields: NoteFields,
    pub is_verified: bool,
    pub created_at: String,
}

#[tauri::command]
pub fn create_note(
    entity_id: i64,
    template_id: String,
    fields: NoteFields,
    user_id: String,
    db_state: State<'_, DbState>,
    data_key_state: State<'_, DataKey>, // ← NUEVO: para cifrado
    signing_state: State<'_, SigningState>, // ← para firma
    templates: State<'_, NoteTemplatesConfig>,
) -> Result<i64, String> {
    let data_key = get_data_key(&data_key_state)?; // ← Usar data_key para cifrado

    templates.validate_note_fields(&template_id, &fields)?;

    // Cifrar cada campo con DataKey (compartida)
    let mut encrypted_fields: HashMap<String, String> = HashMap::new();
    for (field_name, field_value) in &fields {
        let enc = crypto::encrypt_text(field_value, &data_key)?; // ← data_key
        let combined = format!("{}:{}", enc.ciphertext, enc.nonce);
        encrypted_fields.insert(field_name.clone(), combined);
    }

    let enc_fields_json = serde_json::to_vec(&encrypted_fields)
        .map_err(|e| format!("Error serializando campos: {}", e))?;

    // FIRMA DIGITAL con la llave privada del médico (SigningState)
    let sign_guard = signing_state.0.lock().unwrap();
    let private_key_bytes = sign_guard
        .ok_or("Error de Seguridad: No hay llave de firma en RAM. ¿Bóveda cerrada?".to_string())?;

    let payload = templates.build_signature_payload(&template_id, &fields)?;
    let hash = crypto::hash_document(&payload);
    let signature_hex = crypto::sign_hash(&hash, &private_key_bytes)?;

    let external_id = Uuid::new_v4().to_string();

    let note_id = with_conn(&db_state, |conn| {
        conn.execute(
            "INSERT INTO notes (external_id, entity_id, template_id, created_by_user_id, enc_fields, signature, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))",
            params![external_id, entity_id, template_id, user_id, enc_fields_json, signature_hex],
        ).map_err(|e| e.to_string())?;
        Ok(conn.last_insert_rowid())
    })?;

    Ok(note_id)
}

#[tauri::command]
pub fn get_note(
    id: i64,
    db_state: State<'_, DbState>,
    data_key_state: State<'_, DataKey>, // ← Cambiar CryptoState por DataKey
    templates: State<'_, NoteTemplatesConfig>,
) -> Result<NoteRecord, String> {
    let data_key = get_data_key(&data_key_state)?; // ← Usar data_key

    let row = with_conn(&db_state, |conn| {
        let mut stmt = conn.prepare(
            "SELECT external_id, entity_id, template_id, enc_fields, signature, created_at, created_by_user_id
             FROM notes WHERE id = ?1"
        ).map_err(|e| e.to_string())?;

        let result = stmt
            .query_row(params![id], |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            })
            .map_err(|e| e.to_string())?;

        Ok(result)
    })?;

    let (
        external_id,
        entity_id,
        template_id,
        enc_fields_blob,
        signature_hex,
        created_at,
        medico_id,
    ) = row;

    let fields = decrypt_note_fields(&enc_fields_blob, &data_key)?; // ← Usar data_key

    // VERIFICACIÓN DE FIRMA con payload dinámico según template
    let is_verified = match signature_hex {
        Some(sig) => {
            let payload = templates.build_signature_payload(&template_id, &fields)?;
            let hash = crypto::hash_document(&payload);

            let pub_key_result: Result<String, _> = with_conn(&db_state, |conn| {
                conn.query_row(
                    "SELECT public_key FROM professional_profiles WHERE user_id = ?1",
                    params![&medico_id],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())
            });

            match pub_key_result {
                Ok(pk) => crypto::verify_signature(&hash, &sig, &pk).unwrap_or(false),
                Err(_) => false,
            }
        }
        None => false,
    };

    Ok(NoteRecord {
        id,
        external_id,
        entity_id,
        template_id,
        fields,
        is_verified,
        created_at,
    })
}

#[tauri::command]
pub fn get_notes_by_entity(
    entity_id: i64,
    db_state: State<'_, DbState>,
    data_key_state: State<'_, DataKey>, // ← Cambiar
) -> Result<Vec<NoteRecord>, String> {
    let data_key = get_data_key(&data_key_state)?; // ← Usar data_key

    let rows =
        with_conn(&db_state, |conn| {
            let mut stmt = conn.prepare(
            "SELECT id, external_id, entity_id, template_id, enc_fields, signature, created_at
             FROM notes WHERE entity_id = ?1 ORDER BY created_at DESC"
        ).map_err(|e| e.to_string())?;

            let rows = stmt
                .query_map(params![entity_id], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, Vec<u8>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, String>(6)?,
                    ))
                })
                .map_err(|e| e.to_string())?;

            let mut results = Vec::new();
            for row in rows {
                results.push(row.map_err(|e| e.to_string())?);
            }
            Ok(results)
        })?;

    let mut notes = Vec::new();
    for (id, external_id, entity_id, template_id, enc_fields_blob, signature, created_at) in rows {
        let fields = decrypt_note_fields(&enc_fields_blob, &data_key)?; // ← Usar data_key
        let is_verified = signature.is_some();

        notes.push(NoteRecord {
            id,
            external_id,
            entity_id,
            template_id,
            fields,
            is_verified,
            created_at,
        });
    }

    Ok(notes)
}

fn decrypt_note_fields(
    enc_fields_blob: &[u8],
    data_key: &[u8; 32], // ← Renombrar parámetro
) -> Result<HashMap<String, String>, String> {
    let encrypted_fields: HashMap<String, String> = serde_json::from_slice(enc_fields_blob)
        .map_err(|e| format!("Error parseando campos cifrados: {}", e))?;

    let mut fields: HashMap<String, String> = HashMap::new();
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
            crypto::decrypt_text(&enc, data_key) // ← Usar data_key
                .map_err(|e| format!("Error descifrando campo {}: {}", field_name, e))?;

        fields.insert(field_name, decrypted);
    }

    Ok(fields)
}
