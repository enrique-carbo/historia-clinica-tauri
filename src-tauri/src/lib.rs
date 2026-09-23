use rand::RngCore;
use std::sync::Mutex;
use tauri::Manager;

mod auth;
mod commands;
mod config_schema;
mod crypto;
mod data_key;
mod database;
mod lib_types;
mod seed;
mod seed_commands;
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

            // NOTA: La data_key YA NO se carga aquí en claro.
            // Se resuelve en unlock_vault() con la master_key del usuario
            // (o con la seed en el bootstrap de nuevos usuarios).

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
            commands::register_user,
            commands::login_user,
            commands::unlock_vault,
            commands::lock_vault,
            commands::create_entity,
            commands::find_entity_by_blind_index,
            commands::get_entity,
            commands::update_entity,
            commands::create_note,
            commands::get_note,
            commands::list_entities,
            commands::search_entities,
            commands::get_notes_by_entity,
            commands::is_vault_unlocked,
            commands::get_my_profile,
            commands::update_my_profile,
            commands::setup_seed_master_wrap,
            commands::create_entry,
            commands::get_entry,
            commands::get_entries_by_subject,
            commands::get_entries_by_category,
            commands::search_entries,
            seed_commands::generate_seed,
            seed_commands::verify_seed,
            seed_commands::derive_key_from_seed,
            seed_commands::hash_seed,
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
