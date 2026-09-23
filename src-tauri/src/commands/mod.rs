// Traemos los sub-módulos
mod auth_commands;
mod entry_commands;
mod entity_commands;
mod note_commands;
mod professional_profile_commands;

// Re-exportamos todo al exterior
pub use auth_commands::*;
pub use entry_commands::*;
pub use entity_commands::*;
pub use note_commands::*;
pub use professional_profile_commands::*;

// --- HELPERS ---

use crate::lib_types::{CryptoState, DataKey, DbState};
use rusqlite::Connection;
use tauri::State;

/// Ejecuta operaciones sobre la DB manteniendo el Mutex bloqueado el menor tiempo posible.
/// Recibe una función (closure) que usa la conexión, y devuelve su resultado.
pub fn with_conn<F, R>(db_state: &State<'_, DbState>, callback: F) -> Result<R, String>
where
    F: FnOnce(&Connection) -> Result<R, String>,
{
    let guard = db_state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or("Base de datos no inicializada".to_string())?;
    callback(conn) // Se ejecuta, y al salir de esta línea, el Mutex se libera
}

/// Extrae la llave maestra de la RAM.
/// Aquí sí usamos .cloned() porque un arreglo de 32 bytes ([u8; 32]) SÍ es seguro de clonar.
pub fn get_key(crypto_state: &State<'_, CryptoState>) -> Result<[u8; 32], String> {
    let guard = crypto_state.0.lock().unwrap();
    guard
        .as_ref()
        .cloned()
        .ok_or("Sesión no desbloqueada. La llave maestra no está en memoria.".to_string())
}

/// Helper: Extraer la data key del DataKey state
pub fn get_data_key(data_key_state: &DataKey) -> Result<[u8; 32], String> {
    let guard = data_key_state.0.lock().map_err(|_| "Lock poisoned")?;
    guard.ok_or("Data key no inicializada. ¿Reiniciaste la aplicación?".to_string())
}
