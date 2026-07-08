use rand::RngCore;
use std::sync::Mutex;
use tauri::Manager;

mod auth;
mod commands;
mod config_schema;
mod crypto;
mod database;
mod lib_types;
mod vault;

use lib_types::{CryptoState, DataKey, DbState, SigningState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(DbState(Mutex::new(None)))
        .manage(CryptoState(Mutex::new(None)))
        .manage(SigningState(Mutex::new(None)))
        .manage(DataKey(Mutex::new(None))) // ← NUEVO
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_local_data_dir()
                .map_err(|_| "No se pudo determinar la ruta de almacenamiento".to_string())?;

            // Inicializar sal de instalación para blind index
            let installation_salt = get_or_create_installation_salt(&app_data_dir)?;
            crypto::init_blind_index_salt(installation_salt)
                .map_err(|e| format!("Error inicializando blind index salt: {}", e))?;

            // ← NUEVO: Generar o leer llave de cifrado compartida para datos médicos
            let data_key = get_or_create_data_key(&app_data_dir)?;
            let data_key_state = app.state::<DataKey>();
            *data_key_state.0.lock().unwrap() = Some(data_key);
            println!("🔐 [Crypto] Llave de datos médicos inicializada");

            let conn = database::init_db(app_data_dir)
                .map_err(|e| format!("Fallo crítico de DB: {}", e))?;

            let state = app.state::<DbState>();
            *state.0.lock().unwrap() = Some(conn);

            // CARGAR SCHEMAS EMPAQUETADOS
            const SCHEMA_JSON: &str = include_str!("../config/schema.json");
            let schema: config_schema::SchemaConfig =
                serde_json::from_str(SCHEMA_JSON).map_err(|e| format!("Schema inválido: {}", e))?;

            const NOTE_TEMPLATES_JSON: &str = include_str!("../config/note_templates/soap.json");
            let note_templates: config_schema::NoteTemplatesConfig =
                serde_json::from_str(NOTE_TEMPLATES_JSON)
                    .map_err(|e| format!("Note templates inválidos: {}", e))?;

            app.manage(schema);
            app.manage(note_templates);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::test_crypto_flow,
            commands::save_soap_consultation,
            commands::get_patient_history,
            commands::register_user,
            commands::login_user,
            commands::create_patient,
            commands::get_patients_list,
            commands::search_patients,
            commands::unlock_vault,
            commands::lock_vault,
            commands::save_patient_metric,
            commands::get_patient_metrics,
            commands::create_entity,
            commands::find_entity_by_blind_index,
            commands::get_entity,
            commands::create_note,
            commands::get_note,
            commands::list_entities,
            commands::search_entities,
            commands::get_notes_by_entity,
            commands::is_vault_unlocked,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Genera o lee la sal de instalación para blind index.
fn get_or_create_installation_salt(app_dir: &std::path::PathBuf) -> Result<Vec<u8>, String> {
    let salt_path = app_dir.join(".installation_salt");

    if salt_path.exists() {
        std::fs::read(&salt_path).map_err(|e| format!("Error leyendo sal de instalación: {}", e))
    } else {
        let mut salt = vec![0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut salt);
        std::fs::write(&salt_path, &salt)
            .map_err(|e| format!("Error guardando sal de instalación: {}", e))?;
        println!("🔧 [Crypto] Nueva sal de instalación generada para blind index");
        Ok(salt)
    }
}

/// ← NUEVO: Genera o lee la llave de cifrado compartida para datos médicos.
/// Esta llave es compartida entre todos los usuarios de la misma instalación,
/// permitiendo que cualquier médico descifre los datos de pacientes y notas.
fn get_or_create_data_key(app_dir: &std::path::PathBuf) -> Result<[u8; 32], String> {
    let key_path = app_dir.join(".data_key");

    if key_path.exists() {
        let hex = std::fs::read_to_string(&key_path)
            .map_err(|e| format!("Error leyendo data key: {}", e))?;
        let bytes = hex::decode(hex.trim()).map_err(|_| "Data key corrupta".to_string())?;
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);
        println!("🔐 [Crypto] Llave de datos médicos cargada");
        Ok(key)
    } else {
        let mut key = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut key);
        std::fs::write(&key_path, hex::encode(key))
            .map_err(|e| format!("Error guardando data key: {}", e))?;
        println!("🔐 [Crypto] Nueva llave de datos médicos generada");
        Ok(key)
    }
}
