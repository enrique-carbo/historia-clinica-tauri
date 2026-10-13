use std::path::Path;

use tauri::{AppHandle, Manager, State};

use super::{record_audit, require_admin};
use crate::db;
use crate::security::backup;
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

/// Genera un respaldo completo (DB + wraps + vaults) en
/// `dest_parent/respaldo-{timestamp}/`. Requiere administrador y la llave
/// de firma en RAM (bóveda abierta): el manifest se firma con ella.
#[tauri::command]
pub fn create_backup(
    app: AppHandle,
    db_state: State<'_, DbState>,
    session: State<'_, SessionState>,
    signing_state: State<'_, SigningState>,
    dest_parent: String,
) -> Result<BackupInfo, String> {
    let actor = require_admin(&session)?;
    let app_dir = app_data_dir(&app)?;

    let result = {
        let sign_guard = signing_state
            .0
            .lock()
            .map_err(|_| "Lock poisoned".to_string())?;
        let private_key_bytes = sign_guard
            .as_ref()
            .ok_or("No hay llave de firma en RAM. ¿Bóveda cerrada?".to_string())?;

        let guard = db_state
            .0
            .lock()
            .map_err(|_| "Lock poisoned".to_string())?;
        let conn = guard
            .as_ref()
            .ok_or("Base de datos no inicializada".to_string())?;

        backup::create_backup(
            conn,
            &app_dir,
            Path::new(&dest_parent),
            &actor.user_id,
            private_key_bytes,
        )?
    };

    let info = BackupInfo::from(result);
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
            "backup_created",
            None,
            &format!("{} ({} archivos)", info.path, info.file_count),
        )?;
    }

    Ok(info)
}

/// Estado de la firma del manifest devuelto por `validate_backup`.
#[derive(serde::Serialize)]
pub struct BackupValidation {
    pub manifest: backup::Manifest,
    /// `"verified"` | `"unverifiable"`
    pub signature_status: String,
    /// Razón si `signature_status` es `"unverifiable"` (restore requiere
    /// `force_unverified`).
    pub signature_warning: Option<String>,
}

/// Validación completa de un respaldo: hashes + esquema + firma. Una
/// firma inválida contra una llave local conocida es hard error (manifest
/// alterado); lo no verificable queda como warning para el caller.
fn validate_with_signature(
    conn: &rusqlite::Connection,
    src: &Path,
) -> Result<BackupValidation, String> {
    let manifest = backup::validate_backup(src)?;
    match backup::verify_manifest_signature(&manifest, conn) {
        backup::SignatureStatus::Verified => Ok(BackupValidation {
            manifest,
            signature_status: "verified".to_string(),
            signature_warning: None,
        }),
        backup::SignatureStatus::Unverifiable(w) => Ok(BackupValidation {
            manifest,
            signature_status: "unverifiable".to_string(),
            signature_warning: Some(w),
        }),
        backup::SignatureStatus::Forged(w) => Err(format!(
            "Firma del manifest inválida: {}. El respaldo fue alterado o no corresponde a esta instalación; no se puede restaurar.",
            w
        )),
    }
}

/// Valida un respaldo (integridad de hashes + compatibilidad de esquema +
/// firma del manifest) y devuelve su estado. Solo lectura — se llama antes
/// de restaurar.
#[tauri::command]
pub fn validate_backup(
    session: State<'_, SessionState>,
    db_state: State<'_, DbState>,
    src_path: String,
) -> Result<BackupValidation, String> {
    require_admin(&session)?;
    let guard = db_state
        .0
        .lock()
        .map_err(|_| "Lock poisoned".to_string())?;
    let conn = guard
        .as_ref()
        .ok_or("Base de datos no inicializada".to_string())?;
    validate_with_signature(conn, Path::new(&src_path))
}

/// Parámetros de `restore_backup` (agrupados para no pasar el límite de
/// argumentos del command). El frontend manda camelCase.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreRequest {
    pub src_path: String,
    /// Confirmación explícita del admin para restaurar un respaldo cuya
    /// firma no es verificable por esta instalación (legado o firmante
    /// desconocido). No salta una firma inválida: esa falla siempre.
    pub force_unverified: bool,
}

/// Restaura un respaldo sobre el directorio de datos de la instalación.
///
/// Orden: valida todo ANTES de tocar nada → audita → cierra llaves, sesión
/// y conexión → reemplaza archivos → reabre la DB (migraciones). Si falla
/// la copia, intenta reabrir la base vieja.
///
/// La firma del manifest debe estar verificada contra esta instalación;
/// `force_unverified` solo salta esa garantía cuando el manifest no es
/// verificable (legado sin firma o firmante ajeno), nunca cuando la
/// firma es inválida.
#[tauri::command]
pub fn restore_backup(
    app: AppHandle,
    db_state: State<'_, DbState>,
    crypto_state: State<'_, CryptoState>,
    signing_state: State<'_, SigningState>,
    data_key_state: State<'_, DataKey>,
    session: State<'_, SessionState>,
    request: RestoreRequest,
) -> Result<(), String> {
    let actor = require_admin(&session)?;
    let app_dir = app_data_dir(&app)?;
    let src = Path::new(&request.src_path);

    // 1. Validación completa (lectura) — nada destructivo todavía.
    //    Una firma inválida falla acá (sin posibilidad de forzar).
    let validation = {
        let guard = db_state
            .0
            .lock()
            .map_err(|_| "Lock poisoned".to_string())?;
        let conn = guard
            .as_ref()
            .ok_or("Base de datos no inicializada".to_string())?;
        validate_with_signature(conn, src)?
    };

    if validation.signature_status != "verified" && !request.force_unverified {
        let reason = validation
            .signature_warning
            .as_deref()
            .unwrap_or("firma no verificada");
        return Err(format!(
            "El respaldo no tiene una firma verificable por esta instalación ({}). Verificá el origen y confirmá la restauración de todos modos.",
            reason
        ));
    }

    let manifest = validation.manifest;
    let verified_note = if validation.signature_status == "verified" {
        "firma verificada"
    } else {
        "firma no verificable, restauración forzada"
    };

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
            &format!(
                "{} ({} archivos, {})",
                src.display(),
                manifest.files.len(),
                verified_note
            ),
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

            *db_state
                .0
                .lock()
                .map_err(|_| "Lock poisoned".to_string())? = Some(conn);

            Ok(())
        }
    }
}
