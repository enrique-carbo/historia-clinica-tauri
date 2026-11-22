//! Contrato de módulos (`docs/module-contract.md` §3–4, §7).
//!
//! Ruta 1: módulos compilados, `PluginHost` in-process. El borde ya es
//! serializable (`ModuleRequest`/`ModuleResponse`/DTOs) para que Ruta 2 sea
//! un swap de transporte. El actor **siempre** sale de la sesión: los
//! módulos nunca reciben `user_id` del payload.

use crate::core::dto::{Actor, ActorRole, AuditEvent, FiliationDto, ProfessionalDto};
use crate::error::{
    CommandError, ERR_MODULE_INVALID_REQUEST, ERR_MODULE_SCHEMA_TOO_NEW, ERR_MODULE_UNSUPPORTED,
};
use async_trait::async_trait;
use rusqlite::{Connection, Transaction};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use zeroize::Zeroizing;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModuleRequest {
    /// Versión del protocolo. Hoy 1; se incrementa solo si cambia la forma
    /// de `ModuleRequest`/`ModuleResponse`/`ModuleContext`.
    pub protocol_version: u16,
    /// Acción solicitada. String libre que el módulo parsea.
    pub action: String,
    /// Entidad sobre la que opera, si aplica (referencia blanda).
    pub entity_id: Option<i64>,
    /// Payload específico de la acción. El módulo valida.
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModuleResponse {
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleScope {
    /// Activación a nivel instalación: todo el staff accede.
    Installation,
    /// Activación a nivel instalación + acceso explícito por profesional.
    InstallationAndProfessional,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleDescriptor {
    pub id: String,
    pub name: String,
    pub version: String,
    pub scope: ModuleScope,
    /// Roles que pasan el gate. Fail-closed: lista vacía = nadie.
    pub allowed_roles: Vec<ActorRole>,
}

/// Estado de un módulo compilado para la UI de administración.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModuleStatusDto {
    pub id: String,
    pub name: String,
    pub version: String,
    pub scope: ModuleScope,
    pub allowed_roles: Vec<ActorRole>,
    /// `active` | `inactive` | `uninstalled` (fila de `modules` o implícito).
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleError {
    /// Acción/método todavía no implementado (aditivo, §4).
    Unsupported,
    /// El payload no respeta el contrato de la acción.
    InvalidRequest(String),
    /// La DB del módulo tiene una versión posterior a la del binario.
    SchemaTooNew,
    /// Error proveniente de `ModuleContext` / `core::api` (`ERR_VAULT_LOCKED`,
    /// `ERR_ENTITY_NOT_FOUND`, `ERR_ENTITY_ACCESS_DENIED`, etc.): se preserva
    /// intacto al volver al frontend (§12).
    Core(CommandError),
    /// Fallo interno del módulo (DB propia, parseo, etc.).
    Internal(String),
}

impl std::fmt::Display for ModuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => write!(f, "acción no soportada"),
            Self::InvalidRequest(detail) => write!(f, "solicitud inválida: {}", detail),
            Self::SchemaTooNew => write!(f, "schema del módulo más nuevo que el binario"),
            Self::Core(err) => write!(f, "{}", err),
            Self::Internal(detail) => write!(f, "error interno del módulo: {}", detail),
        }
    }
}

impl std::error::Error for ModuleError {}

impl From<CommandError> for ModuleError {
    fn from(err: CommandError) -> Self {
        Self::Core(err)
    }
}

impl From<ModuleError> for CommandError {
    fn from(err: ModuleError) -> Self {
        match err {
            ModuleError::Unsupported => CommandError::new(ERR_MODULE_UNSUPPORTED),
            ModuleError::InvalidRequest(detail) => {
                CommandError::new(ERR_MODULE_INVALID_REQUEST).with_param("detail", detail)
            }
            ModuleError::SchemaTooNew => CommandError::new(ERR_MODULE_SCHEMA_TOO_NEW),
            ModuleError::Core(cmd_err) => cmd_err,
            ModuleError::Internal(detail) => CommandError::internal(detail),
        }
    }
}

/// Handle opaco a la DB propia del módulo (`fichio-<module_id>.db`, §7).
///
/// La conexión vive detrás de un `Mutex` y solo se expone mediante closures
/// síncronos (`with_conn` / `with_tx`): el `MutexGuard` se suelta antes de
/// volver a código `async`, evitando que un módulo deje el lock tomado a
/// través de un `.await`.
pub struct ModuleDb {
    conn: Mutex<Connection>,
}

impl std::fmt::Debug for ModuleDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModuleDb").finish_non_exhaustive()
    }
}

impl ModuleDb {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }

    pub fn with_conn<F, R>(&self, callback: F) -> Result<R, ModuleError>
    where
        F: FnOnce(&Connection) -> Result<R, ModuleError>,
    {
        let guard = self
            .conn
            .lock()
            .map_err(|_| ModuleError::Internal("Lock poisoned".to_string()))?;
        callback(&guard)
    }

    pub fn with_tx<F, R>(&self, callback: F) -> Result<R, ModuleError>
    where
        F: FnOnce(&Transaction<'_>) -> Result<R, ModuleError>,
    {
        let mut guard = self
            .conn
            .lock()
            .map_err(|_| ModuleError::Internal("Lock poisoned".to_string()))?;
        let tx = guard
            .transaction()
            .map_err(|e| ModuleError::Internal(e.to_string()))?;
        let out = callback(&tx)?;
        tx.commit()
            .map_err(|e| ModuleError::Internal(e.to_string()))?;
        Ok(out)
    }
}

/// Acceso acotado del módulo al Core. El host lo implementa in-process
/// (Ruta 1) o por proxy serializado (Ruta 2).
#[async_trait]
pub trait ModuleContext: Send + Sync {
    /// Actor derivado de la sesión. Nunca del payload.
    fn actor(&self) -> &Actor;

    /// Filiación proyectada. Actor `paciente`: solo su propia entity.
    async fn fetch_filiation(&self, entity_id: i64) -> Result<FiliationDto, CommandError>;

    async fn entity_exists(&self, entity_id: i64) -> Result<bool, CommandError>;

    async fn fetch_professional(&self, user_id: &str) -> Result<ProfessionalDto, CommandError>;

    /// Auditoría con enum cerrado, sin PHI.
    async fn audit(&self, event: AuditEvent) -> Result<(), CommandError>;

    /// Crypto opt-in: el módulo nunca ve la DataKey.
    async fn wrap_with_datakey(&self, plaintext: &[u8]) -> Result<Vec<u8>, CommandError>;
    async fn unwrap_with_datakey(
        &self,
        ciphertext: &[u8],
    ) -> Result<Zeroizing<Vec<u8>>, CommandError>;
}

/// Un módulo compilado. No hay métodos por acción: agregar una acción no
/// cambia el trait (lo que sobrevive a Ruta 2).
#[async_trait]
pub trait Module: Send + Sync + 'static {
    fn descriptor(&self) -> ModuleDescriptor;

    /// Migración de la DB **propia** del módulo. El host la llama en la
    /// activación y en cada arranque con el módulo activo. Idempotente.
    /// Recibe `&Connection` para poder correr dentro de la transacción que
    /// abre el host (`Transaction` no implementa `DerefMut`).
    fn migrate(&self, conn: &Connection) -> Result<(), ModuleError>;

    /// Único punto de entrada. Recibe `db` (la DB propia del módulo) y `ctx`
    /// (acceso mediado al Core).
    async fn handle(
        &self,
        req: ModuleRequest,
        db: &ModuleDb,
        ctx: &dyn ModuleContext,
    ) -> Result<ModuleResponse, ModuleError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{ERR_ENTITY_ACCESS_DENIED, ERR_VAULT_LOCKED};
    use futures_executor::block_on;

    struct DummyContext {
        actor: Actor,
    }

    #[async_trait]
    impl ModuleContext for DummyContext {
        fn actor(&self) -> &Actor {
            &self.actor
        }

        async fn fetch_filiation(&self, entity_id: i64) -> Result<FiliationDto, CommandError> {
            Ok(FiliationDto {
                entity_id,
                full_name: "Ana Pérez".to_string(),
                document: "30111222".to_string(),
                fields: Vec::new(),
            })
        }

        async fn entity_exists(&self, _entity_id: i64) -> Result<bool, CommandError> {
            Ok(true)
        }

        async fn fetch_professional(
            &self,
            user_id: &str,
        ) -> Result<ProfessionalDto, CommandError> {
            Ok(ProfessionalDto {
                user_id: user_id.to_string(),
                full_name: "Dra. Test".to_string(),
                profession: "Médica".to_string(),
                specialty: "Clínica".to_string(),
                licenses: Vec::new(),
                public_key_hex: "pk".to_string(),
            })
        }

        async fn audit(&self, _event: AuditEvent) -> Result<(), CommandError> {
            Ok(())
        }

        async fn wrap_with_datakey(&self, plaintext: &[u8]) -> Result<Vec<u8>, CommandError> {
            Ok(plaintext.to_vec())
        }

        async fn unwrap_with_datakey(
            &self,
            ciphertext: &[u8],
        ) -> Result<Zeroizing<Vec<u8>>, CommandError> {
            Ok(Zeroizing::new(ciphertext.to_vec()))
        }
    }

    struct DummyModule;

    #[async_trait]
    impl Module for DummyModule {
        fn descriptor(&self) -> ModuleDescriptor {
            ModuleDescriptor {
                id: "dummy".to_string(),
                name: "Módulo de prueba".to_string(),
                version: "0.1.0".to_string(),
                scope: ModuleScope::Installation,
                allowed_roles: vec![ActorRole::Profesional],
            }
        }

        fn migrate(&self, conn: &Connection) -> Result<(), ModuleError> {
            conn.execute_batch("CREATE TABLE IF NOT EXISTS items (id TEXT PRIMARY KEY)")
                .map_err(|e| ModuleError::Internal(e.to_string()))
        }

        async fn handle(
            &self,
            req: ModuleRequest,
            db: &ModuleDb,
            ctx: &dyn ModuleContext,
        ) -> Result<ModuleResponse, ModuleError> {
            db.with_tx(|tx| {
                tx.execute("INSERT INTO items (id) VALUES (?1)", [&req.action])
                    .map_err(|e| ModuleError::Internal(e.to_string()))?;
                Ok(())
            })?;

            let count: i64 = db.with_conn(|conn| {
                conn.query_row("SELECT COUNT(*) FROM items", [], |r| r.get(0))
                    .map_err(|e| ModuleError::Internal(e.to_string()))
            })?;

            let filiation = ctx.fetch_filiation(req.entity_id.unwrap_or(1)).await?;

            Ok(ModuleResponse {
                payload: serde_json::json!({
                    "actor": ctx.actor().user_id,
                    "action": req.action,
                    "patient": filiation.full_name,
                    "count": count,
                }),
            })
        }
    }

    #[test]
    fn module_contract_is_object_safe_and_dispatches_with_module_db() {
        let module: Box<dyn Module> = Box::new(DummyModule);

        let desc = module.descriptor();
        assert_eq!(desc.id, "dummy");
        assert_eq!(desc.allowed_roles, vec![ActorRole::Profesional]);

        let conn = Connection::open_in_memory().expect("in-memory");
        module.migrate(&conn).expect("migrate");
        module.migrate(&conn).expect("migrate es idempotente");
        let db = ModuleDb::new(conn);

        let ctx = DummyContext {
            actor: Actor {
                user_id: "user-1".to_string(),
                role: ActorRole::Profesional,
            },
        };
        let req = ModuleRequest {
            protocol_version: 1,
            action: "ping".to_string(),
            entity_id: Some(7),
            payload: serde_json::json!({}),
        };

        let res = block_on(module.handle(req, &db, &ctx)).expect("handle");
        assert_eq!(res.payload["actor"], "user-1");
        assert_eq!(res.payload["action"], "ping");
        assert_eq!(res.payload["patient"], "Ana Pérez");
        assert_eq!(res.payload["count"], 1);
    }

    #[test]
    fn module_error_maps_to_stable_command_codes_and_preserves_core_errors() {
        let unsupported: CommandError = ModuleError::Unsupported.into();
        assert_eq!(unsupported.code, ERR_MODULE_UNSUPPORTED);

        let invalid: CommandError = ModuleError::InvalidRequest("falta campo".into()).into();
        assert_eq!(invalid.code, ERR_MODULE_INVALID_REQUEST);
        assert_eq!(
            invalid.params.get("detail").map(String::as_str),
            Some("falta campo")
        );

        let too_new: CommandError = ModuleError::SchemaTooNew.into();
        assert_eq!(too_new.code, ERR_MODULE_SCHEMA_TOO_NEW);

        // Un error del Core (ej. bóveda cerrada o acceso denegado) que sube
        // con `?` dentro de `handle` conserva su código exacto al volver.
        let core_locked: ModuleError = CommandError::new(ERR_VAULT_LOCKED).into();
        let back: CommandError = core_locked.into();
        assert_eq!(back.code, ERR_VAULT_LOCKED);

        let core_denied: ModuleError = CommandError::new(ERR_ENTITY_ACCESS_DENIED).into();
        let back: CommandError = core_denied.into();
        assert_eq!(back.code, ERR_ENTITY_ACCESS_DENIED);
    }

    #[test]
    fn request_and_descriptor_roundtrip_through_json() {
        let req = ModuleRequest {
            protocol_version: 1,
            action: "template.create".to_string(),
            entity_id: Some(3),
            payload: serde_json::json!({"name": "Rx tórax"}),
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: ModuleRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back, req);

        let desc = DummyModule.descriptor();
        let json = serde_json::to_string(&desc).unwrap();
        let back: ModuleDescriptor = serde_json::from_str(&json).unwrap();
        assert_eq!(back, desc);
    }
}
