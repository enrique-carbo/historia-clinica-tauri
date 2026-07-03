use crate::crypto;
use tauri::State;
use uuid::Uuid;

use super::get_key;

// 1. Ampliamos el input para aceptar todos los datos sensibles adicionales
#[derive(serde::Deserialize)]
pub struct CreatePatientInput {
    pub created_by_user_id: String,
    pub full_name: String,
    pub identity_doc: String,

    // Nuevos datos opcionales que irán dentro del Blob Cifrado
    pub birth_date: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

// 2. Estructura para devolver la lista de pacientes (sin exponer el Blob)
#[derive(serde::Serialize, Clone)]
pub struct PatientRecord {
    pub id: String,
    pub full_name: String,
    pub created_at: String,
}

#[tauri::command]
pub fn create_patient(
    form: CreatePatientInput,
    db_state: State<'_, crate::lib_types::DbState>,
    crypto_state: State<'_, crate::lib_types::CryptoState>,
) -> Result<String, String> {
    let master_key = get_key(&crypto_state)?;

    // Ciframos el nombre por separado (para mostrarlo rápido en listas)
    let name_enc = crypto::encrypt_text(&form.full_name, &master_key)?;

    // Generamos los índices ciegos (DNI y Nombre)
    let blind_index = crypto::generate_blind_index(&form.identity_doc);
    let name_blind_index = crypto::generate_blind_index(&form.full_name);

    // --- NUEVO: Construcción del Blob Cifrado ---
    // Empaquetamos el resto de los datos en un JSON
    let patient_data_blob = serde_json::json!({
        "birth_date": form.birth_date,
        "address": form.address,
        "phone": form.phone,
        "email": form.email
        // Aquí puedes agregar más campos en el futuro sin tocar la DB
    })
    .to_string();

    // Ciframos el JSON completo
    let blob_enc = crypto::encrypt_text(&patient_data_blob, &master_key)?;

    let patient_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    super::with_conn(&db_state, |conn| {
        conn.execute(
            "INSERT INTO patients (id, created_by_user_id, identity_blind_index, name_blind_index, full_name_ciphertext, full_name_nonce, encrypted_data_blob, encrypted_data_nonce, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9);",
            rusqlite::params![
                &patient_id,
                &form.created_by_user_id,
                &blind_index,
                &name_blind_index,
                &name_enc.ciphertext,
                &name_enc.nonce,
                &blob_enc.ciphertext,
                &blob_enc.nonce,
                &now,
            ],
        ).map_err(|e| {
            if e.to_string().contains("UNIQUE constraint failed") {
                "Error Crítico: Ya existe un paciente registrado con ese Documento de Identidad.".to_string()
            } else {
                format!("Error de persistencia: {}", e)
            }
        })?;

        println!(
            "📇 [SQLite] Paciente creado con Blob Cifrado. ID: {}",
            patient_id
        );
        Ok(patient_id)
    })
}

#[tauri::command]
pub fn get_patients_list(
    db_state: State<'_, crate::lib_types::DbState>,
    crypto_state: State<'_, crate::lib_types::CryptoState>,
) -> Result<Vec<PatientRecord>, String> {
    let master_key = get_key(&crypto_state)?;

    super::with_conn(&db_state, |conn| {
        let mut stmt = conn
            .prepare("SELECT id, full_name_ciphertext, full_name_nonce, created_at FROM patients ORDER BY created_at DESC;")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                let enc = crypto::EncryptedData {
                    ciphertext: row.get(1)?,
                    nonce: row.get(2)?,
                };
                let full_name = crypto::decrypt_text(&enc, &master_key)
                    .unwrap_or_else(|_| "[Error de descifrado]".to_string());

                Ok(PatientRecord {
                    id: row.get(0)?,
                    full_name,
                    created_at: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(|e| e.to_string())?);
        }
        Ok(list)
    })
}

#[tauri::command]
pub fn search_patients(
    query_name: String,
    db_state: State<'_, crate::lib_types::DbState>,
    crypto_state: State<'_, crate::lib_types::CryptoState>,
) -> Result<Vec<PatientRecord>, String> {
    let master_key = get_key(&crypto_state)?;

    super::with_conn(&db_state, |conn| {
        // Obtenemos todos los pacientes (solo el nombre, para no traer el Blob pesado)
        let mut stmt = conn
            .prepare("SELECT id, full_name_ciphertext, full_name_nonce, created_at FROM patients ORDER BY created_at DESC;")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| map_patient_row(row, &master_key))
            .map_err(|e| e.to_string())?;

        let query_lower = query_name.to_lowercase().trim().to_string();
        let mut filtered = Vec::new();

        // Filtramos en memoria (Rust es ultra rápido para esto)
        for r in rows {
            let patient = r.map_err(|e| e.to_string())?;
            if query_lower.is_empty() || patient.full_name.to_lowercase().contains(&query_lower) {
                filtered.push(patient);
            }
        }

        Ok(filtered)
    })
}

// Helper para mapear el resultado de la DB a la estructura PatientRecord
fn map_patient_row(row: &rusqlite::Row, master_key: &[u8]) -> rusqlite::Result<PatientRecord> {
    let enc = crypto::EncryptedData {
        ciphertext: row.get(1)?,
        nonce: row.get(2)?,
    };

    // Convertimos el slice &[u8] a un arreglo &[u8; 32] para satisfacer al compilador
    let key_array: &[u8; 32] = master_key.try_into().unwrap_or(&[0u8; 32]);

    let full_name = crypto::decrypt_text(&enc, key_array)
        .unwrap_or_else(|_| "[Error de Descifrado]".to_string());

    Ok(PatientRecord {
        id: row.get(0)?,
        full_name,
        created_at: row.get(3)?,
    })
}
