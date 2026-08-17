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
    // Nuevos campos
    pub address: String,
    pub phone: String,
    pub email: String,
    pub website: String,
}

#[derive(Deserialize)]
pub struct UpdateProfileInput {
    pub user_id: String,
    pub full_name: String,
    pub license_number: String,
    pub specialty: String,
    // Nuevos campos
    pub address: String,
    pub phone: String,
    pub email: String,
    pub website: String,
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

    // Helper interno para descifrar un par ciphertext/nonce
    let decrypt_field = |ct: String, nonce: String| -> String {
        if ct.is_empty() {
            return String::new();
        }
        let enc = crypto::EncryptedData {
            ciphertext: ct,
            nonce,
        };
        crypto::decrypt_text(&enc, &data_key).unwrap_or_default()
    };

    let row = with_conn(&db_state, |conn| {
        conn.query_row(
            "SELECT full_name_ciphertext, full_name_nonce,
                     license_number_ciphertext, license_number_nonce,
                     specialty_ciphertext, specialty_nonce,
                     public_key,
                     address_ciphertext, address_nonce,
                     phone_ciphertext, phone_nonce,
                     email_ciphertext, email_nonce,
                     website_ciphertext, website_nonce
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
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, String>(11)?,
                    row.get::<_, String>(12)?,
                    row.get::<_, String>(13)?,
                    row.get::<_, String>(14)?,
                ))
            },
        )
        .map_err(|e| e.to_string())
    })?;

    // Desempaquetar y descifrar
    let (
        fn_ct,
        fn_n,
        ln_ct,
        ln_n,
        sp_ct,
        sp_n,
        pk,
        addr_ct,
        addr_n,
        ph_ct,
        ph_n,
        em_ct,
        em_n,
        web_ct,
        web_n,
    ) = row;

    Ok(ProfessionalProfile {
        user_id,
        full_name: decrypt_field(fn_ct, fn_n),
        license_number: decrypt_field(ln_ct, ln_n),
        specialty: decrypt_field(sp_ct, sp_n),
        public_key: pk,
        address: decrypt_field(addr_ct, addr_n),
        phone: decrypt_field(ph_ct, ph_n),
        email: decrypt_field(em_ct, em_n),
        website: decrypt_field(web_ct, web_n),
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

    // Cifrar todos los campos sensibles
    let encrypt = |text: &str| -> Result<(String, String), String> {
        let enc = crypto::encrypt_text(text, &data_key)?;
        Ok((enc.ciphertext, enc.nonce))
    };

    let (fn_ct, fn_n) = encrypt(&input.full_name)?;
    let (ln_ct, ln_n) = encrypt(&input.license_number)?;
    let (sp_ct, sp_n) = encrypt(&input.specialty)?;
    let (addr_ct, addr_n) = encrypt(&input.address)?;
    let (ph_ct, ph_n) = encrypt(&input.phone)?;
    let (em_ct, em_n) = encrypt(&input.email)?;
    let (web_ct, web_n) = encrypt(&input.website)?;

    with_conn(&db_state, |conn| {
        conn.execute(
            "UPDATE professional_profiles
             SET full_name_ciphertext = ?1, full_name_nonce = ?2,
                 license_number_ciphertext = ?3, license_number_nonce = ?4,
                 specialty_ciphertext = ?5, specialty_nonce = ?6,
                 address_ciphertext = ?7, address_nonce = ?8,
                 phone_ciphertext = ?9, phone_nonce = ?10,
                 email_ciphertext = ?11, email_nonce = ?12,
                 website_ciphertext = ?13, website_nonce = ?14,
                 updated_at = datetime('now')
             WHERE user_id = ?15",
            params![
                fn_ct,
                fn_n,
                ln_ct,
                ln_n,
                sp_ct,
                sp_n,
                addr_ct,
                addr_n,
                ph_ct,
                ph_n,
                em_ct,
                em_n,
                web_ct,
                web_n,
                &input.user_id,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })?;
    Ok(true)
}
