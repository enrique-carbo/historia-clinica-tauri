use crate::crypto;
use tauri::State;
use uuid::Uuid;

use super::get_key;

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
}

#[tauri::command]
pub fn save_soap_consultation(
    form: SoapInput,
    db_state: State<'_, crate::lib_types::DbState>,
    crypto_state: State<'_, crate::lib_types::CryptoState>,
) -> Result<String, String> {
    let master_key = get_key(&crypto_state)?;

    let s_enc = crypto::encrypt_text(&form.subjetivo, &master_key)?;
    let o_enc = crypto::encrypt_text(&form.objetivo, &master_key)?;
    let a_enc = crypto::encrypt_text(&form.analisis, &master_key)?;
    let p_enc = crypto::encrypt_text(&form.plan, &master_key)?;

    let consultation_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    super::with_conn(&db_state, |conn| {
        conn.execute(
            "INSERT INTO in_person_consultations (
                id, paciente_id, medico_id,
                s_subjetivo_ciphertext, s_subjetivo_nonce,
                o_objetivo_ciphertext, o_objetivo_nonce,
                a_analisis_ciphertext, a_analisis_nonce,
                p_plan_ciphertext, p_plan_nonce,
                is_synced, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 0, ?12, ?13);",
            [
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
                &now,
                &now,
            ],
        )
        .map_err(|e| format!("Error al persistir en SQLite: {}", e))?;

        println!(
            "📥 [SQLite] Consulta SOAP guardada con llave dinámica. ID: {}",
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
                    is_synced, created_at
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

                let subjetivo = crypto::decrypt_text(&s_enc, &master_key)
                    .unwrap_or_else(|_| "[Error S]".to_string());
                let objetivo = crypto::decrypt_text(&o_enc, &master_key)
                    .unwrap_or_else(|_| "[Error O]".to_string());
                let analisis = crypto::decrypt_text(&a_enc, &master_key)
                    .unwrap_or_else(|_| "[Error A]".to_string());
                let plan = crypto::decrypt_text(&p_enc, &master_key)
                    .unwrap_or_else(|_| "[Error P]".to_string());

                let is_synced_num: i32 = row.get(11)?;

                Ok(SoapRecord {
                    id: row.get(0)?,
                    paciente_id: row.get(1)?,
                    medico_id: row.get(2)?,
                    subjetivo,
                    objetivo,
                    analisis,
                    plan,
                    is_synced: is_synced_num == 1,
                    created_at: row.get(12)?,
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
