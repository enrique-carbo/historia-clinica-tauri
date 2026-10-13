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
        .plugin(tauri_plugin_dialog::init())
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

            // NOTA: La data_key YA NO se carga aquí en claro.
            // Se resuelve en unlock_vault() con la master_key del usuario
            // (o con la seed en el bootstrap de nuevos usuarios).

            let conn = db::database::init_db(app_data_dir)
                .map_err(|e| format!("Fallo crítico de DB: {}", e))?;

            let state = app.state::<DbState>();
            *state.0.lock().unwrap() = Some(conn);

            // CARGAR SCHEMA EMPAQUETADO (entity_schema.json, fuente única)
            let schema = db::config_schema::from_embedded()?;

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
            commands::grant_patient_access,
            commands::get_entity_access,
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
            commands::export_history,
            commands::generate_seed,
            commands::verify_seed,
            commands::derive_key_from_seed,
            commands::hash_seed,
            commands::create_backup,
            commands::validate_backup,
            commands::restore_backup,
            commands::seed_recovery,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
