use std::sync::Mutex;
use tauri::Manager;

mod auth;
mod commands;
mod config_schema;
mod crypto;
mod database;
mod lib_types;
mod vault;

use lib_types::{CryptoState, DbState, SigningState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(DbState(Mutex::new(None)))
        .manage(CryptoState(Mutex::new(None)))
        .manage(SigningState(Mutex::new(None)))
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_local_data_dir()
                .map_err(|_| "No se pudo determinar la ruta de almacenamiento".to_string())?;

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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
