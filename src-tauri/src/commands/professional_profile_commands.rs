// src/commands/professional_profile_commands.rs

use crate::crypto;
use crate::lib_types::{DataKey, DbState};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

use super::{get_data_key, with_conn};

// =======================================================================
// TIPOS
// =======================================================================

#[derive(Serialize)]
pub struct ProfessionalProfile {
    pub user_id: String,
    pub full_name: String,
    pub license_number: String,
    pub specialty: String,
    pub public_key: String,
}

#[derive(Deserialize)]
pub struct UpdateProfileInput {
    pub user_id: String,
    pub full_name: String,
    pub license_number: String,
    pub specialty: String,
}

// =======================================================================
// COMANDOS
// =======================================================================

/// Obtiene el perfil del médico autenticado, descifrando los campos sensibles
#[tauri::command]
pub fn get_my_profile(
    user_id: String,
    db_state: State<'_, DbState>,
    data_key_state: State<'_, DataKey>,
) -> Result<ProfessionalProfile, String> {
    let data_key = get_data_key(&data_key_state)?;

    let row = with_conn(&db_state, |conn| {
        conn.query_row(
            "SELECT full_name_ciphertext, full_name_nonce,
                    license_number_ciphertext, license_number_nonce,
                    specialty_ciphertext, specialty_nonce,
                    public_key
             FROM professional_profiles
             WHERE user_id = ?1",
            params![&user_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            },
        )
        .map_err(|e| e.to_string())
    })?;

    let (
        full_name_ct,
        full_name_nonce,
        license_ct,
        license_nonce,
        specialty_ct,
        specialty_nonce,
        public_key,
    ) = row;

    // Descifrar campos (si están vacíos, devolver string vacío)
    let full_name = if full_name_ct.is_empty() {
        String::new()
    } else {
        let enc = crypto::EncryptedData {
            ciphertext: full_name_ct,
            nonce: full_name_nonce,
        };
        crypto::decrypt_text(&enc, &data_key).unwrap_or_default()
    };

    let license_number = if license_ct.is_empty() {
        String::new()
    } else {
        let enc = crypto::EncryptedData {
            ciphertext: license_ct,
            nonce: license_nonce,
        };
        crypto::decrypt_text(&enc, &data_key).unwrap_or_default()
    };

    let specialty = if specialty_ct.is_empty() {
        String::new()
    } else {
        let enc = crypto::EncryptedData {
            ciphertext: specialty_ct,
            nonce: specialty_nonce,
        };
        crypto::decrypt_text(&enc, &data_key).unwrap_or_default()
    };

    Ok(ProfessionalProfile {
        user_id,
        full_name,
        license_number,
        specialty,
        public_key,
    })
}

/// Actualiza el perfil del médico, cifrando los campos sensibles
#[tauri::command]
pub fn update_my_profile(
    input: UpdateProfileInput,
    db_state: State<'_, DbState>,
    data_key_state: State<'_, DataKey>,
) -> Result<bool, String> {
    let data_key = get_data_key(&data_key_state)?;

    // Cifrar campos
    let full_name_enc = crypto::encrypt_text(&input.full_name, &data_key)?;
    let license_enc = crypto::encrypt_text(&input.license_number, &data_key)?;
    let specialty_enc = crypto::encrypt_text(&input.specialty, &data_key)?;

    with_conn(&db_state, |conn| {
        conn.execute(
            "UPDATE professional_profiles
             SET full_name_ciphertext = ?1,
                 full_name_nonce = ?2,
                 license_number_ciphertext = ?3,
                 license_number_nonce = ?4,
                 specialty_ciphertext = ?5,
                 specialty_nonce = ?6
             WHERE user_id = ?7",
            params![
                full_name_enc.ciphertext,
                full_name_enc.nonce,
                license_enc.ciphertext,
                license_enc.nonce,
                specialty_enc.ciphertext,
                specialty_enc.nonce,
                &input.user_id,
            ],
        )
        .map_err(|e| e.to_string())?;

        Ok(())
    })?;

    Ok(true)
}
