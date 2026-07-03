use crate::crypto;
use tauri::State;
use uuid::Uuid;

use super::get_key;

// Función helper para convertir el error de String a un error de Rusqlite
fn map_decrypt_err(e: String) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        e,
    )))
}

#[derive(serde::Deserialize)]
pub struct SoapInput {
    pub paciente_id: String,
    pub medico_id: String,
    pub subjetivo: String,
    pub objetivo: String,
    pub analisis: String,
    pub plan: String,
}

#[derive(serde::Serialize)]
pub struct SoapRecord {
    pub id: String,
    pub paciente_id: String,
    pub medico_id: String,
    pub subjetivo: String,
    pub objetivo: String,
    pub analisis: String,
    pub plan: String,
    pub is_synced: bool,
    pub created_at: String,
    pub is_verified: bool,
}

#[tauri::command]
pub fn save_soap_consultation(
    form: SoapInput,
    db_state: State<'_, crate::lib_types::DbState>,
    crypto_state: State<'_, crate::lib_types::CryptoState>,
    signing_state: State<'_, crate::lib_types::SigningState>,
) -> Result<String, String> {
    let master_key = get_key(&crypto_state)?;

    let s_enc = crypto::encrypt_text(&form.subjetivo, &master_key)?;
    let o_enc = crypto::encrypt_text(&form.objetivo, &master_key)?;
    let a_enc = crypto::encrypt_text(&form.analisis, &master_key)?;
    let p_enc = crypto::encrypt_text(&form.plan, &master_key)?;

    let consultation_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    // PROCESO DE FIRMA DIGITAL
    let sign_guard = signing_state.0.lock().unwrap();
    let private_key_bytes = sign_guard
        .ok_or("Error de Seguridad: No hay llave de firma en RAM. ¿Bóveda cerrada?".to_string())?;

    let document_payload = format!(
        "{}|{}|{}|{}",
        form.subjetivo, form.objetivo, form.analisis, form.plan
    );
    let hash = crypto::hash_document(&document_payload);
    let signature_hex = crypto::sign_hash(&hash, &private_key_bytes)?;

    super::with_conn(&db_state, |conn| {
        conn.execute(
            "INSERT INTO in_person_consultations (
                id, paciente_id, medico_id,
                s_subjetivo_ciphertext, s_subjetivo_nonce,
                o_objetivo_ciphertext, o_objetivo_nonce,
                a_analisis_ciphertext, a_analisis_nonce,
                p_plan_ciphertext, p_plan_nonce,
                digital_signature, is_synced, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 0, ?13, ?14);",
            rusqlite::params![
                &consultation_id,
                &form.paciente_id,
                &form.medico_id,
                &s_enc.ciphertext,
                &s_enc.nonce,
                &o_enc.ciphertext,
                &o_enc.nonce,
                &a_enc.ciphertext,
                &a_enc.nonce,
                &p_enc.ciphertext,
                &p_enc.nonce,
                &signature_hex,
                &now,
                &now,
            ],
        )
        .map_err(|e| format!("Error al persistir en SQLite: {}", e))?;

        println!(
            "📥 [SQLite] Consulta SOAP guardada y firmada. ID: {}",
            consultation_id
        );
        Ok(consultation_id)
    })
}

#[tauri::command]
pub fn get_patient_history(
    paciente_id: String,
    db_state: State<'_, crate::lib_types::DbState>,
    crypto_state: State<'_, crate::lib_types::CryptoState>,
) -> Result<Vec<SoapRecord>, String> {
    let master_key = get_key(&crypto_state)?;

    super::with_conn(&db_state, |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, paciente_id, medico_id,
                    s_subjetivo_ciphertext, s_subjetivo_nonce,
                    o_objetivo_ciphertext, o_objetivo_nonce,
                    a_analisis_ciphertext, a_analisis_nonce,
                    p_plan_ciphertext, p_plan_nonce,
                    is_synced, created_at, digital_signature
                 FROM in_person_consultations
                 WHERE paciente_id = ?1
                 ORDER BY created_at DESC;",
            )
            .map_err(|e| format!("Error al preparar SELECT: {}", e))?;

        let records_iter = stmt
            .query_map([&paciente_id], |row| {
                let s_enc = crypto::EncryptedData {
                    ciphertext: row.get(3)?,
                    nonce: row.get(4)?,
                };
                let o_enc = crypto::EncryptedData {
                    ciphertext: row.get(5)?,
                    nonce: row.get(6)?,
                };
                let a_enc = crypto::EncryptedData {
                    ciphertext: row.get(7)?,
                    nonce: row.get(8)?,
                };
                let p_enc = crypto::EncryptedData {
                    ciphertext: row.get(9)?,
                    nonce: row.get(10)?,
                };

                // Usamos nuestro helper para manejar el error limpiamente
                let subjetivo =
                    crypto::decrypt_text(&s_enc, &master_key).map_err(map_decrypt_err)?;
                let objetivo =
                    crypto::decrypt_text(&o_enc, &master_key).map_err(map_decrypt_err)?;
                let analisis =
                    crypto::decrypt_text(&a_enc, &master_key).map_err(map_decrypt_err)?;
                let plan = crypto::decrypt_text(&p_enc, &master_key).map_err(map_decrypt_err)?;

                let is_synced_num: i32 = row.get(11)?;
                let created_at: String = row.get(12)?;
                let signature_hex: Option<String> = row.get(13)?;
                let medico_id: String = row.get(2)?;

                // LÓGICA DE VERIFICACIÓN
                let is_verified = match signature_hex {
                    Some(sig) => {
                        let doc_payload =
                            format!("{}|{}|{}|{}", subjetivo, objetivo, analisis, plan);
                        let current_hash = crate::crypto::hash_document(&doc_payload);

                        let pub_key_result: Result<String, _> = conn.query_row(
                            "SELECT public_key FROM professional_profiles WHERE user_id = ?1",
                            [&medico_id],
                            |r| r.get(0),
                        );

                        match pub_key_result {
                            Ok(pk) => crate::crypto::verify_signature(&current_hash, &sig, &pk)
                                .unwrap_or(false),
                            Err(_) => false,
                        }
                    }
                    None => false,
                };

                Ok(SoapRecord {
                    id: row.get(0)?,
                    paciente_id: row.get(1)?,
                    medico_id,
                    subjetivo,
                    objetivo,
                    analisis,
                    plan,
                    is_synced: is_synced_num == 1,
                    created_at,
                    is_verified,
                })
            })
            .map_err(|e| format!("Error en la query: {}", e))?;

        let mut history = Vec::new();
        for record in records_iter {
            history.push(record.map_err(|e| format!("Error al procesar fila: {}", e))?);
        }
        Ok(history)
    })
}
