use chrono::Utc;
use tauri::State;
use uuid::Uuid;

use crate::lib_types::{CryptoState, DbState};

#[derive(serde::Deserialize)]
pub struct SaveMetricInput {
    pub paciente_id: String,
    pub medico_id: String, // <--- NUEVO: Necesario para la DB
    pub metric_type: String,
    pub sub_metric: String,
    pub value_num: f64,
    pub value_text: Option<String>, // Notas opcionales
}

#[tauri::command]
pub fn save_patient_metric(
    form: SaveMetricInput,
    db_state: State<'_, DbState>,
    crypto_state: State<'_, CryptoState>,
) -> Result<String, String> {
    let metric_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    // Normalizamos los textos a minúsculas
    let metric_type_norm = form.metric_type.to_lowercase();
    let sub_metric_norm = form.sub_metric.to_lowercase();

    // Si hay una nota de texto, la ciframos. Si no, queda NULL gracias al "!" de Option
    let (text_ciphertext, text_nonce) = match form.value_text {
        Some(text) if !text.is_empty() => {
            let master_key = super::get_key(&crypto_state)?;
            let enc = crate::crypto::encrypt_text(&text, &master_key)?;
            (Some(enc.ciphertext), Some(enc.nonce))
        }
        _ => (None, None),
    };

    super::with_conn(&db_state, |conn| {
        conn.execute(
            "INSERT INTO patient_metrics (id, paciente_id, medico_id, metric_type, sub_metric, value_num, value_text_ciphertext, value_text_nonce, measured_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                &metric_id,
                &form.paciente_id,
                &form.medico_id, // <--- NUEVO
                &metric_type_norm,
                &sub_metric_norm,
                form.value_num,
                text_ciphertext,
                text_nonce,
                &now,
            ],
        ).map_err(|e| format!("Error al guardar métrica: {}", e))?;

        println!(
            "📊 [SQLite] Métrica guardada: {} -> {} = {}",
            metric_type_norm, sub_metric_norm, form.value_num
        );
        Ok(metric_id)
    })
}

#[derive(serde::Serialize)]
pub struct MetricRecord {
    pub id: String,
    pub metric_type: String,
    pub sub_metric: String,
    pub value_num: f64,
    pub note: String, // Ya descifrada para React
    pub measured_at: String,
}

#[tauri::command]
pub fn get_patient_metrics(
    paciente_id: String,
    db_state: State<'_, DbState>,
    crypto_state: State<'_, CryptoState>,
) -> Result<Vec<MetricRecord>, String> {
    let master_key = super::get_key(&crypto_state)?;

    super::with_conn(&db_state, |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, metric_type, sub_metric, value_num, value_text_ciphertext, value_text_nonce, measured_at
                 FROM patient_metrics
                 WHERE paciente_id = ?1
                 ORDER BY measured_at DESC;",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([&paciente_id], |row| {
                let enc_text: Option<String> = row.get(4)?;
                let enc_nonce: Option<String> = row.get(5)?;

                // Desciframos la nota si existe, si no, devolvemos string vacío
                let note = match (enc_text, enc_nonce) {
                    (Some(ciph), Some(nonce)) => {
                        let enc_data = crate::crypto::EncryptedData {
                            ciphertext: ciph,
                            nonce,
                        };
                        // Pasamos master_key directamente
                        crate::crypto::decrypt_text(&enc_data, &master_key)
                            .unwrap_or_else(|_| "[Error al descifrar nota]".to_string())
                    }
                    _ => String::new(),
                };

                Ok(MetricRecord {
                    id: row.get(0)?,
                    metric_type: row.get(1)?,
                    sub_metric: row.get(2)?,
                    value_num: row.get::<_, f64>(3)?,
                    note,
                    measured_at: row.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut metrics = Vec::new();
        for r in rows {
            metrics.push(r.map_err(|e| e.to_string())?);
        }
        Ok(metrics)
    })
}
