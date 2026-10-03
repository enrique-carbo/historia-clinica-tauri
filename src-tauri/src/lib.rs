use rand::RngCore;
use std::sync::Mutex;
use tauri::Manager;

mod commands;
mod db;
mod export;
mod security;
mod types;

use types::{CryptoState, DataKey, DbState, SessionState, SigningState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(DbState(Mutex::new(None)))
        .manage(CryptoState(Mutex::new(None)))
        .manage(SigningState(Mutex::new(None)))
        .manage(DataKey(Mutex::new(None)))
        .manage(SessionState(Mutex::new(None)))
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_local_data_dir()
                .map_err(|_| "No se pudo determinar la ruta de almacenamiento".to_string())?;

            // Inicializar sal de instalación para blind index
            let installation_salt = get_or_create_installation_salt(&app_data_dir)?;
            security::crypto::init_blind_index_salt(installation_salt)
                .map_err(|e| format!("Error inicializando blind index salt: {}", e))?;

            // NOTA: La data_key YA NO se carga aquí en claro.
            // Se resuelve en unlock_vault() con la master_key del usuario
            // (o con la seed en el bootstrap de nuevos usuarios).

            let conn = db::database::init_db(app_data_dir)
                .map_err(|e| format!("Fallo crítico de DB: {}", e))?;

            let state = app.state::<DbState>();
            *state.0.lock().unwrap() = Some(conn);

            // CARGAR SCHEMAS EMPAQUETADOS
            const SCHEMA_JSON: &str = include_str!("../config/schema.json");
            let schema: db::config_schema::SchemaConfig =
                serde_json::from_str(SCHEMA_JSON).map_err(|e| format!("Schema inválido: {}", e))?;

            app.manage(schema);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::test_crypto_flow,
            commands::register_user,
            commands::login_user,
            commands::count_users,
            commands::admin_create_user,
            commands::list_users,
            commands::set_user_active,
            commands::list_audit_log,
            commands::admin_reset_password,
            commands::admin_rotate_seed,
            commands::unlock_vault,
            commands::lock_vault,
            commands::change_password,
            commands::create_entity,
            commands::find_entity_by_blind_index,
            commands::get_entity,
            commands::update_entity,
            commands::list_entities,
            commands::search_entities,
            commands::is_vault_unlocked,
            commands::get_my_profile,
            commands::update_my_profile,
            commands::setup_seed_master_wrap,
            commands::create_entry,
            commands::get_entry,
            commands::get_entries_by_subject,
            commands::get_entries_by_category,
            commands::search_entries,
            commands::get_export_snapshot,
            commands::export_history,
            commands::generate_seed,
            commands::verify_seed,
            commands::derive_key_from_seed,
            commands::hash_seed,
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
