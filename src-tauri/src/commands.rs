use crate::auth;
use crate::crypto;
use crate::lib_types::DbState;
use chrono::Utc;
use tauri::State;
use uuid::Uuid;

const MOCK_MASTER_KEY: &[u8; 32] = b"una-clave-secreta-de-32-bytes---";

// Estructura que viene desde la UI de React en texto plano
#[derive(serde::Deserialize)]
pub struct SoapInput {
    pub paciente_id: String,
    pub medico_id: String,
    pub subjetivo: String,
    pub objetivo: String,
    pub analisis: String,
    pub plan: String,
}

#[derive(serde::Deserialize)]
pub struct RegisterInput {
    pub username: String,
    pub password_plain: String,
    pub role: String, // 'admin', 'medico', 'paciente'
}

#[derive(serde::Deserialize)]
pub struct LoginInput {
    pub username: String,
    pub password_plain: String,
}

#[derive(serde::Serialize)]
pub struct AuthResponse {
    pub user_id: String,
    pub username: String,
    pub role: String,
}

#[derive(serde::Deserialize)]
pub struct CreatePatientInput {
    pub created_by_user_id: String,
    pub full_name: String,
    pub identity_doc: String, // DNI o Pasaporte en texto plano (solo vive en RAM temporalmente)
}

#[derive(serde::Serialize)]
pub struct PatientRecord {
    pub id: String,
    pub full_name: String,
    pub created_at: String,
}

#[tauri::command]
pub fn save_soap_consultation(
    form: SoapInput,
    state: State<'_, DbState>,
) -> Result<String, String> {
    // 1. Cifrar de forma independiente cada componente del estándar SOAP
    let s_enc = crypto::encrypt_text(&form.subjetivo, MOCK_MASTER_KEY)?;
    let o_enc = crypto::encrypt_text(&form.objetivo, MOCK_MASTER_KEY)?;
    let a_enc = crypto::encrypt_text(&form.analisis, MOCK_MASTER_KEY)?;
    let p_enc = crypto::encrypt_text(&form.plan, MOCK_MASTER_KEY)?;

    // 2. Generar metadatos locales (ID único y timestamps en formato ISO 8601 UTC)
    let consultation_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    // 3. Bloquear temporalmente el Mutex de la conexión a la base de datos para escribir
    let state_guard = state.0.lock().unwrap();
    let conn = state_guard
        .as_ref()
        .ok_or("Base de datos no inicializada")?;

    // 4. Ejecutar el INSERT físico en SQLite
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
        "📥 [SQLite] Consulta SOAP guardada localmente con ID: {}",
        consultation_id
    );
    Ok(consultation_id)
}

// Estructura que le devolveremos a la UI con los datos ya descifrados en el Core
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
pub fn get_patient_history(
    paciente_id: String,
    state: State<'_, DbState>,
) -> Result<Vec<SoapRecord>, String> {
    // 1. Bloquear el Mutex para lectura
    let state_guard = state.0.lock().unwrap();
    let conn = state_guard
        .as_ref()
        .ok_or("Base de datos no inicializada")?;

    // 2. Preparar la Query SQL para traer el historial de un paciente específico
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

    // 3. Mapear y DESENCRIPTAR los resultados en caliente (en RAM) antes de armar el vector
    let records_iter = stmt
        .query_map([&paciente_id], |row| {
            // Reconstruimos las estructuras encriptadas intermedias para pasárselas al módulo crypto
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

            // Desciframos usando nuestra Clave Maestra en memoria
            let subjetivo = crypto::decrypt_text(&s_enc, MOCK_MASTER_KEY)
                .unwrap_or_else(|_| "[Error al descifrar S]".to_string());
            let objetivo = crypto::decrypt_text(&o_enc, MOCK_MASTER_KEY)
                .unwrap_or_else(|_| "[Error al descifrar O]".to_string());
            let analisis = crypto::decrypt_text(&a_enc, MOCK_MASTER_KEY)
                .unwrap_or_else(|_| "[Error al descifrar A]".to_string());
            let plan = crypto::decrypt_text(&p_enc, MOCK_MASTER_KEY)
                .unwrap_or_else(|_| "[Error al descifrar P]".to_string());

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
        .map_err(|e| format!("Error en la ejecución de la query: {}", e))?;

    // 4. Consolidar el iterador en un Vector limpio
    let mut history = Vec::new();
    for record in records_iter {
        history.push(record.map_err(|e| format!("Error al procesar fila: {}", e))?);
    }

    println!(
        "📤 [SQLite] Leídas y descifradas {} consultas para el paciente: {}",
        history.len(),
        paciente_id
    );
    Ok(history)
}

// Mantenemos tu comando de prueba por si acaso
#[tauri::command]
pub fn test_crypto_flow(text: String) -> Result<String, String> {
    let encrypted = crypto::encrypt_text(&text, MOCK_MASTER_KEY)?;
    let decrypted = crypto::decrypt_text(&encrypted, MOCK_MASTER_KEY)?;
    Ok(format!(
        "Cifrado Hex: {}. Descifrado: {}",
        encrypted.ciphertext, decrypted
    ))
}

#[tauri::command]
pub fn register_user(form: RegisterInput, state: State<'_, DbState>) -> Result<String, String> {
    // 1. Hashear contraseña en RAM usando Argon2id
    let hashed = auth::hash_password(&form.password_plain)?;
    let user_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    // 2. Persistir en SQLite
    let state_guard = state.0.lock().unwrap();
    let conn = state_guard
        .as_ref()
        .ok_or("Base de datos no inicializada")?;

    conn.execute(
        "INSERT INTO users (id, username, password_hash, role, created_at) VALUES (?1, ?2, ?3, ?4, ?5);",
        [&user_id, &form.username, &hashed, &form.role, &now],
    ).map_err(|e| {
        if e.to_string().contains("UNIQUE constraint failed") {
            "El nombre de usuario ya existe en este sistema local.".to_string()
        } else {
            format!("Error al registrar usuario: {}", e)
        }
    })?;

    println!(
        "👤 [SQLite] Nuevo usuario registrado -> [{}]: {}",
        form.role, form.username
    );
    Ok(user_id)
}

#[tauri::command]
pub fn login_user(form: LoginInput, state: State<'_, DbState>) -> Result<AuthResponse, String> {
    let state_guard = state.0.lock().unwrap();
    let conn = state_guard
        .as_ref()
        .ok_or("Base de datos no inicializada")?;

    // 1. Buscar al usuario en la DB
    let mut stmt = conn
        .prepare("SELECT id, password_hash, role FROM users WHERE username = ?1;")
        .map_err(|e| e.to_string())?;

    let mut rows = stmt.query([&form.username]).map_err(|e| e.to_string())?;

    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        // 💡 Mapeamos los errores de rusqlite a String antes de usar el ?
        let user_id: String = row.get(0).map_err(|e| e.to_string())?;
        let hash_guardado: String = row.get(1).map_err(|e| e.to_string())?;
        let role: String = row.get(2).map_err(|e| e.to_string())?;

        // 2. Validar contraseña contra el hash de Argon2id
        let is_valid = auth::verify_password(&form.password_plain, &hash_guardado)?;

        if is_valid {
            println!("🔓 [Auth] Login exitoso para el usuario: {}", form.username);
            Ok(AuthResponse {
                user_id,
                username: form.username,
                role,
            })
        } else {
            Err("Contraseña incorrecta".to_string())
        }
    } else {
        Err("El usuario no existe".to_string())
    }
}

#[tauri::command]
pub fn create_patient(
    form: CreatePatientInput,
    state: State<'_, DbState>,
) -> Result<String, String> {
    // 1. Cifrar el nombre completo del paciente (AES-GCM-256)
    let name_enc = crypto::encrypt_text(&form.full_name, MOCK_MASTER_KEY)?;

    // 2. Generar el Índice Ciego del documento para control de duplicados
    let blind_index = crypto::generate_blind_index(&form.identity_doc);

    let patient_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let state_guard = state.0.lock().unwrap();
    let conn = state_guard
        .as_ref()
        .ok_or("Base de datos no inicializada")?;

    // 3. Insertar en SQLite
    conn.execute(
        "INSERT INTO patients (id, created_by_user_id, full_name_ciphertext, full_name_nonce, identity_blind_index, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
        [
            &patient_id,
            &form.created_by_user_id,
            &name_enc.ciphertext,
            &name_enc.nonce,
            &blind_index,
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
        "📇 [SQLite] Paciente creado de forma segura. ID: {}",
        patient_id
    );
    Ok(patient_id)
}

#[tauri::command]
pub fn get_patients_list(state: State<'_, DbState>) -> Result<Vec<PatientRecord>, String> {
    let state_guard = state.0.lock().unwrap();
    let conn = state_guard
        .as_ref()
        .ok_or("Base de datos no inicializada")?;

    let mut stmt = conn.prepare("SELECT id, full_name_ciphertext, full_name_nonce, created_at FROM patients ORDER BY created_at DESC;")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            let enc = crypto::EncryptedData {
                ciphertext: row.get(1)?,
                nonce: row.get(2)?,
            };
            let full_name = crypto::decrypt_text(&enc, MOCK_MASTER_KEY)
                .unwrap_or_else(|_| "[Error]".to_string());

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
}

#[tauri::command]
pub fn search_patients(
    query_name: String,
    state: State<'_, DbState>,
) -> Result<Vec<PatientRecord>, String> {
    let state_guard = state.0.lock().unwrap();
    let conn = state_guard
        .as_ref()
        .ok_or("Base de datos no inicializada")?;

    // Traemos todos los pacientes para buscarlos en memoria RAM de forma segura (Zero-Knowledge)
    let mut stmt = conn
        .prepare("SELECT id, full_name_ciphertext, full_name_nonce, created_at FROM patients;")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            let enc = crypto::EncryptedData {
                ciphertext: row.get(1)?,
                nonce: row.get(2)?,
            };
            // Desciframos el nombre en la RAM del médico
            let full_name = crypto::decrypt_text(&enc, MOCK_MASTER_KEY)
                .unwrap_or_else(|_| "[Error de Descifrado]".to_string());

            Ok(PatientRecord {
                id: row.get(0)?,
                full_name,
                created_at: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let query_lower = query_name.to_lowercase().trim().to_string();
    let mut filtered = Vec::new();

    for r in rows {
        let patient = r.map_err(|e| e.to_string())?;
        // Filtramos de forma reactiva en memoria
        if query_lower.is_empty() || patient.full_name.to_lowercase().contains(&query_lower) {
            filtered.push(patient);
        }
    }

    Ok(filtered)
}
