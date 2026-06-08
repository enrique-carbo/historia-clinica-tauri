use std::sync::Mutex;
use tauri::Manager;

// Declaramos todos los módulos de nuestra aplicación
mod auth;
mod commands;
mod crypto;
mod database;
mod lib_types;

// Traemos al alcance el estado que aislamos
use lib_types::DbState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // Inicializamos el estado global vacío
        .manage(DbState(Mutex::new(None)))
        // Configuramos la base de datos local al arrancar
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_local_data_dir()
                .map_err(|_| "No se pudo determinar la ruta de almacenamiento".to_string())?;

            let conn = database::init_db(app_data_dir)
                .map_err(|e| format!("Fallo crítico de DB: {}", e))?;

            let state = app.state::<DbState>();
            *state.0.lock().unwrap() = Some(conn);

            Ok(())
        })
        // 🚀 IMPORTANTE: Registramos el comando llamando al módulo externo
        .invoke_handler(tauri::generate_handler![
            commands::test_crypto_flow,
            commands::save_soap_consultation,
            commands::get_patient_history,
            commands::register_user,
            commands::login_user,
            commands::create_patient,
            commands::get_patients_list,
            commands::search_patients
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
