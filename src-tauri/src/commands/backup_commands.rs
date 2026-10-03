use std::path::Path;

use tauri::{AppHandle, Manager, State};

use super::{record_audit, require_admin};
use crate::db;
use crate::security::{backup, crypto};
use crate::types::{CryptoState, DataKey, DbState, SessionState, SigningState};

/// Resumen devuelto al frontend tras generar un respaldo.
#[derive(serde::Serialize)]
pub struct BackupInfo {
    pub path: String,
    pub app_version: String,
    pub schema_version: u32,
    pub created_at: String,
    pub file_count: usize,
    pub total_bytes: u64,
}

impl From<(std::path::PathBuf, backup::Manifest)> for BackupInfo {
    fn from((path, m): (std::path::PathBuf, backup::Manifest)) -> Self {
        BackupInfo {
            path: path.to_string_lossy().into_owned(),
            app_version: m.app_version,
            schema_version: m.schema_version,
            created_at: m.created_at,
            file_count: m.files.len(),
            total_bytes: m.files.iter().map(|f| f.bytes).sum(),
        }
    }
}

fn app_data_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_local_data_dir()
        .map_err(|_| "No se pudo determinar la ruta de almacenamiento".to_string())
}

/// Genera un respaldo completo (DB + wraps + vaults + sal) en
/// `dest_parent/respaldo-{timestamp}/`. Requiere administrador.
#[tauri::command]
pub fn create_backup(
    app: AppHandle,
    db_state: State<'_, DbState>,
    session: State<'_, SessionState>,
    dest_parent: String,
) -> Result<BackupInfo, String> {
    let actor = require_admin(&session)?;
    let app_dir = app_data_dir(&app)?;

    let guard = db_state
        .0
        .lock()
        .map_err(|_| "Lock poisoned".to_string())?;
    let conn = guard
        .as_ref()
        .ok_or("Base de datos no inicializada".to_string())?;

    let result = backup::create_backup(conn, &app_dir, Path::new(&dest_parent))?;

    let info = BackupInfo::from(result);
    record_audit(
        conn,
        &actor.user_id,
        "backup_created",
        None,
        &format!("{} ({} archivos)", info.path, info.file_count),
    )?;

    Ok(info)
}

/// Valida un respaldo (integridad de hashes + compatibilidad de esquema)
/// y devuelve su manifest. Solo lectura — se llama antes de restaurar.
#[tauri::command]
pub fn validate_backup(
    session: State<'_, SessionState>,
    src_path: String,
) -> Result<backup::Manifest, String> {
    require_admin(&session)?;
    backup::validate_backup(Path::new(&src_path))
}

/// Restaura un respaldo sobre el directorio de datos de la instalación.
///
/// Orden: valida todo ANTES de tocar nada → audita → cierra llaves, sesión
/// y conexión → reemplaza archivos → reabre la DB (migraciones) → recarga
/// la sal de blind index. Si falla la copia, intenta reabrir la base vieja.
#[tauri::command]
pub fn restore_backup(
    app: AppHandle,
    db_state: State<'_, DbState>,
    crypto_state: State<'_, CryptoState>,
    signing_state: State<'_, SigningState>,
    data_key_state: State<'_, DataKey>,
    session: State<'_, SessionState>,
    src_path: String,
) -> Result<(), String> {
    let actor = require_admin(&session)?;
    let app_dir = app_data_dir(&app)?;
    let src = Path::new(&src_path);

    // 1. Validación completa (lectura) — nada destructivo todavía
    let manifest = backup::validate_backup(src)?;

    // 2. Auditoría con la conexión actual
    {
        let guard = db_state
            .0
            .lock()
            .map_err(|_| "Lock poisoned".to_string())?;
        let conn = guard
            .as_ref()
            .ok_or("Base de datos no inicializada".to_string())?;
        record_audit(
            conn,
            &actor.user_id,
            "backup_restored",
            None,
            &format!("{} ({} archivos)", src.display(), manifest.files.len()),
        )?;
    }

    // 3. Cerrar llaves y sesión: los datos que se van a reemplazar ya no
    //    deben estar descifrados en RAM
    *crypto_state
        .0
        .lock()
        .map_err(|_| "Lock poisoned".to_string())? = None;
    *signing_state
        .0
        .lock()
        .map_err(|_| "Lock poisoned".to_string())? = None;
    *data_key_state
        .0
        .lock()
        .map_err(|_| "Lock poisoned".to_string())? = None;
    *session
        .0
        .lock()
        .map_err(|_| "Lock poisoned".to_string())? = None;

    // 4. Cerrar la conexión (SQLite no puede quedar abierto sobre archivos reemplazados)
    let old_conn = db_state
        .0
        .lock()
        .map_err(|_| "Lock poisoned".to_string())?
        .take()
        .ok_or("Base de datos no inicializada".to_string())?;
    drop(old_conn);

    // 5. Restaurar
    let restore_result = backup::restore_backup(src, &app_dir);

    // 6. Reabrir siempre (aunque la copia haya fallado, para no dejar la app sin base)
    let reopened = db::database::init_db(app_dir.clone());

    match restore_result {
        Err(e) => {
            if let Ok(conn) = reopened {
                *db_state
                    .0
                    .lock()
                    .map_err(|_| "Lock poisoned".to_string())? = Some(conn);
            }
            Err(format!("La restauración falló: {}", e))
        }
        Ok(_) => {
            let conn = reopened
                .map_err(|e| format!("Respaldo restaurado, pero no se pudo reabrir la base: {}", e))?;

            // La sal de blind index puede haber cambiado (restauración
            // desde otra instalación): recargarla desde el archivo restaurado.
            let salt_path = app_dir.join(".installation_salt");
            if salt_path.exists() {
                let salt = std::fs::read(&salt_path)
                    .map_err(|e| format!("No se pudo leer la sal de instalación: {}", e))?;
                crypto::init_blind_index_salt(salt)?;
            }

            *db_state
                .0
                .lock()
                .map_err(|_| "Lock poisoned".to_string())? = Some(conn);

            Ok(())
        }
    }
}
