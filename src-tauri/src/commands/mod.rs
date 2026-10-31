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

use crate::error::{CommandError, ERR_DB_NOT_INITIALIZED, ERR_VAULT_LOCKED};
use crate::types::{CryptoState, DataKey, DbState};
use rusqlite::Connection;
use tauri::State;
use zeroize::Zeroizing;

/// Ejecuta operaciones sobre la DB manteniendo el Mutex bloqueado el menor tiempo posible.
/// Recibe una función (closure) que usa la conexión, y devuelve su resultado.
///
/// El closure trabaja en `CommandError`: los `map_err(|e| e.to_string())?` y
/// helpers que todavía devuelven `String` convierten solos con `?`.
pub fn with_conn<F, R>(db_state: &State<'_, DbState>, callback: F) -> Result<R, CommandError>
where
    F: FnOnce(&Connection) -> Result<R, CommandError>,
{
    let guard = db_state.0.lock().unwrap();
    let conn = guard
        .as_ref()
        .ok_or_else(|| CommandError::new(ERR_DB_NOT_INITIALIZED))?;
    callback(conn) // Se ejecuta, y al salir de esta línea, el Mutex se libera
}

/// Extrae la llave maestra de la RAM.
///
/// Devuelve `Zeroizing` porque esta llamada **clona** la llave: esa copia
/// vivirá en el stack del caller y debe ponerse a cero al caer, no quedar
/// residual en el heap/stack. Deref coercion (`&key` → `&[u8; 32]`) mantiene
/// inalterados los call sites.
pub fn get_key(
    crypto_state: &State<'_, CryptoState>,
) -> Result<Zeroizing<[u8; 32]>, CommandError> {
    let guard = crypto_state
        .0
        .lock()
        .map_err(|_| CommandError::internal("Lock poisoned"))?;
    guard
        .as_ref()
        .map(|k| Zeroizing::new(**k))
        .ok_or_else(|| CommandError::new(ERR_VAULT_LOCKED))
}

/// Helper: Extraer la data key del DataKey state.
/// Igual que `get_key`: la copia que se devuelve se zeroizea al soltarse.
pub fn get_data_key(data_key_state: &DataKey) -> Result<Zeroizing<[u8; 32]>, CommandError> {
    let guard = data_key_state
        .0
        .lock()
        .map_err(|_| CommandError::internal("Lock poisoned"))?;
    guard
        .as_ref()
        .map(|k| Zeroizing::new(**k))
        .ok_or_else(|| CommandError::new(ERR_VAULT_LOCKED))
}
