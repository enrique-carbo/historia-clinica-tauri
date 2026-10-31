use std::path::Path;

use tauri::{AppHandle, Manager, State};

use super::{record_audit, require_admin, with_conn};
use crate::db;
use crate::error::{
    CommandError, ERR_BACKUP_SIGNATURE_INVALID, ERR_BACKUP_UNVERIFIED, ERR_DB_NOT_INITIALIZED,
    ERR_SIGNING_KEY_MISSING, ERR_STORAGE_PATH,
};
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

fn app_data_dir(app: &AppHandle) -> Result<std::path::PathBuf, CommandError> {
    app.path()
        .app_local_data_dir()
        .map_err(|_| CommandError::new(ERR_STORAGE_PATH))
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
) -> Result<BackupInfo, CommandError> {
    let actor = require_admin(&session)?;
    let app_dir = app_data_dir(&app)?;

    let result = {
        let sign_guard = signing_state
            .0
            .lock()
            .map_err(|_| CommandError::internal("Lock poisoned"))?;
        let private_key_bytes = sign_guard
            .as_ref()
            .ok_or_else(|| CommandError::new(ERR_SIGNING_KEY_MISSING))?;

        let guard = db_state
            .0
            .lock()
            .map_err(|_| CommandError::internal("Lock poisoned"))?;
        let conn = guard
            .as_ref()
            .ok_or_else(|| CommandError::new(ERR_DB_NOT_INITIALIZED))?;

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
            .map_err(|_| CommandError::internal("Lock poisoned"))?;
        let conn = guard
            .as_ref()
            .ok_or_else(|| CommandError::new(ERR_DB_NOT_INITIALIZED))?;
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

/// Último respaldo registrado en `audit_log`.
///
/// `audit_log` es la fuente local de verdad (el archivo de respaldo puede
/// estar en cualquier carpeta elegida por el usuario, así que no se puede
/// inferir del filesystem). Tras una restauración, el "último" es el que
/// quedó dentro del respaldo restaurado — que es lo correcto: no hay
/// respaldo posterior a ese.
#[derive(Debug, serde::Serialize)]
pub struct BackupStatus {
    /// RFC3339 del último `backup_created`, o `None` si nunca hubo.
    pub last_backup_at: Option<String>,
    pub backup_count: i64,
}

/// Estado del respaldo para el recordatorio de la UI. Solo administradores:
/// expone metadatos de auditoría (fechas), nunca contenido.
#[tauri::command]
pub fn backup_status(
    session: State<'_, SessionState>,
    db_state: State<'_, DbState>,
) -> Result<BackupStatus, CommandError> {
    require_admin(&session)?;
    with_conn(&db_state, |conn| {
        conn.query_row(
            "SELECT MAX(created_at), COUNT(*) FROM audit_log WHERE action = 'backup_created'",
            [],
            |r| Ok((r.get::<_, Option<String>>(0)?, r.get::<_, i64>(1)?)),
        )
        .map(|(last_backup_at, backup_count)| BackupStatus {
            last_backup_at,
            backup_count,
        })
        .map_err(|e| CommandError::internal(e.to_string()))
    })
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
) -> Result<BackupValidation, CommandError> {
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
        backup::SignatureStatus::Forged(w) => {
            Err(CommandError::new(ERR_BACKUP_SIGNATURE_INVALID).with_param("reason", w))
        }
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
) -> Result<BackupValidation, CommandError> {
    require_admin(&session)?;
    let guard = db_state
        .0
        .lock()
        .map_err(|_| CommandError::internal("Lock poisoned"))?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| CommandError::new(ERR_DB_NOT_INITIALIZED))?;
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
) -> Result<(), CommandError> {
    let actor = require_admin(&session)?;
    let app_dir = app_data_dir(&app)?;
    let src = Path::new(&request.src_path);

    // 1. Validación completa (lectura) — nada destructivo todavía.
    //    Una firma inválida falla acá (sin posibilidad de forzar).
    let validation = {
        let guard = db_state
            .0
            .lock()
            .map_err(|_| CommandError::internal("Lock poisoned"))?;
        let conn = guard
            .as_ref()
            .ok_or_else(|| CommandError::new(ERR_DB_NOT_INITIALIZED))?;
        validate_with_signature(conn, src)?
    };

    if validation.signature_status != "verified" && !request.force_unverified {
        let reason = validation
            .signature_warning
            .as_deref()
            .unwrap_or("firma no verificada");
        return Err(CommandError::new(ERR_BACKUP_UNVERIFIED).with_param("reason", reason));
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
            .map_err(|_| CommandError::internal("Lock poisoned"))?;
        let conn = guard
            .as_ref()
            .ok_or_else(|| CommandError::new(ERR_DB_NOT_INITIALIZED))?;
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
        .map_err(|_| CommandError::internal("Lock poisoned"))? = None;
    *signing_state
        .0
        .lock()
        .map_err(|_| CommandError::internal("Lock poisoned"))? = None;
    *data_key_state
        .0
        .lock()
        .map_err(|_| CommandError::internal("Lock poisoned"))? = None;
    *session
        .0
        .lock()
        .map_err(|_| CommandError::internal("Lock poisoned"))? = None;

    // 4. Cerrar la conexión (SQLite no puede quedar abierto sobre archivos reemplazados)
    let old_conn = db_state
        .0
        .lock()
        .map_err(|_| CommandError::internal("Lock poisoned"))?
        .take()
        .ok_or_else(|| CommandError::new(ERR_DB_NOT_INITIALIZED))?;
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
                    .map_err(|_| CommandError::internal("Lock poisoned"))? = Some(conn);
            }
            Err(CommandError::internal(format!(
                "La restauración falló: {}",
                e
            )))
        }
        Ok(_) => {
            let conn = reopened.map_err(|e| {
                CommandError::internal(format!(
                    "Respaldo restaurado, pero no se pudo reabrir la base: {}",
                    e
                ))
            })?;

            *db_state
                .0
                .lock()
                .map_err(|_| CommandError::internal("Lock poisoned"))? = Some(conn);

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::database::init_db;
    use crate::error::ERR_ADMIN_REQUIRED;
    use crate::types::SessionUser;
    use std::sync::Mutex;
    use tauri::Manager;

    /// App mock con DB temporal y una sesión del rol dado. El usuario actor
    /// existe porque `audit_log.actor_user_id` tiene FK a `users`.
    fn setup(role: &str) -> (tauri::App<tauri::test::MockRuntime>, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let conn = init_db(dir.path().to_path_buf()).expect("init_db");
        conn.execute(
            "INSERT INTO users (id, username, password_hash, role, created_at)
             VALUES ('actor-1', 'actor.test', 'hash', ?1, ?2)",
            rusqlite::params![role, chrono::Utc::now().to_rfc3339()],
        )
        .expect("insert actor");

        let app = tauri::test::mock_app();
        app.manage(DbState(Mutex::new(Some(conn))));
        app.manage(SessionState(Mutex::new(Some(SessionUser {
            user_id: "actor-1".into(),
            username: "actor".into(),
            role: role.into(),
        }))));
        (app, dir)
    }

    fn status(
        app: &tauri::App<tauri::test::MockRuntime>,
    ) -> Result<BackupStatus, CommandError> {
        backup_status(
            app.state::<SessionState>(),
            app.state::<DbState>(),
        )
    }

    fn audit_backup(app: &tauri::App<tauri::test::MockRuntime>, created_at: &str) {
        let db = app.state::<DbState>();
        let guard = db.0.lock().unwrap();
        record_audit(
            guard.as_ref().unwrap(),
            "actor-1",
            "backup_created",
            None,
            "respaldo de prueba",
        )
        .expect("audit");
        // `record_audit` usa NOW(); para el test necesitamos fechas fijas.
        guard
            .as_ref()
            .unwrap()
            .execute(
                "UPDATE audit_log SET created_at = ?1 WHERE action = 'backup_created'",
                rusqlite::params![created_at],
            )
            .expect("fix created_at");
    }

    #[test]
    fn sin_respaldos_no_hay_fecha_ni_conteo() {
        let (app, _dir) = setup("administrador");
        let s = status(&app).expect("status");
        assert_eq!(s.last_backup_at, None);
        assert_eq!(s.backup_count, 0);
    }

    #[test]
    fn toma_el_respaldo_mas_reciente_y_cuenta_los_previos() {
        let (app, _dir) = setup("administrador");
        audit_backup(&app, "2026-10-01T10:00:00+00:00");
        audit_backup(&app, "2026-10-20T10:00:00+00:00");

        let s = status(&app).expect("status");
        assert_eq!(s.backup_count, 2);
        assert_eq!(
            s.last_backup_at.as_deref(),
            Some("2026-10-20T10:00:00+00:00")
        );
    }

    #[test]
    fn solo_administradores_ven_el_estado() {
        let (app, _dir) = setup("profesional");
        let err = status(&app).expect_err("debe rechazar");
        assert_eq!(err.code, ERR_ADMIN_REQUIRED, "err: {}", err);
    }
}
