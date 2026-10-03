// Traemos los sub-módulos
mod admin_commands;
mod auth_commands;
mod backup_commands;
mod entity_commands;
mod entry_commands;
mod export_commands;
mod professional_profile_commands;
mod seed_commands;

// Re-exportamos todo al exterior
pub use admin_commands::*;
pub use auth_commands::*;
pub use backup_commands::*;
pub use entity_commands::*;
pub use entry_commands::*;
pub use export_commands::*;
pub use professional_profile_commands::*;
pub use seed_commands::*;

// --- HELPERS ---

use crate::types::{CryptoState, DataKey, DbState};
use rusqlite::Connection;
use tauri::State;
use zeroize::Zeroizing;

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
///
/// Devuelve `Zeroizing` porque esta llamada **clona** la llave: esa copia
/// vivirá en el stack del caller y debe ponerse a cero al caer, no quedar
/// residual en el heap/stack. Deref coercion (`&key` → `&[u8; 32]`) mantiene
/// inalterados los call sites.
pub fn get_key(crypto_state: &State<'_, CryptoState>) -> Result<Zeroizing<[u8; 32]>, String> {
    let guard = crypto_state.0.lock().unwrap();
    guard
        .as_ref()
        .map(|k| Zeroizing::new(**k))
        .ok_or("Sesión no desbloqueada. La llave maestra no está en memoria.".to_string())
}

/// Helper: Extraer la data key del DataKey state.
/// Igual que `get_key`: la copia que se devuelve se zeroizea al soltarse.
pub fn get_data_key(data_key_state: &DataKey) -> Result<Zeroizing<[u8; 32]>, String> {
    let guard = data_key_state.0.lock().map_err(|_| "Lock poisoned")?;
    guard
        .as_ref()
        .map(|k| Zeroizing::new(**k))
        .ok_or("Data key no inicializada. ¿Reiniciaste la aplicación?".to_string())
}
