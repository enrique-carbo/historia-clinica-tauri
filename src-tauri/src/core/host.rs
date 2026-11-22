//! `PluginHost` de la Ruta 1 (`docs/module-contract.md` §5–§7).
//!
//! Mantiene el registro de módulos compilados y las conexiones a sus DBs
//! propias (`fichio-<id>.db`). El gate corre acá, **antes** de llamar al
//! módulo: instalado → protocolo → activo → rol → acceso. El host es parte
//! del Core (no de un módulo), por eso sí conoce las tablas de gobierno
//! `modules` / `module_access` (v002). No conoce el schema de las DBs de los
//! módulos: solo llama a su `migrate()` y les pasa un `ModuleDb` opaco.

use crate::core::api::audit;
use crate::core::dto::{Actor, AuditEvent};
use crate::core::module::{
    Module, ModuleDb, ModuleDescriptor, ModuleRequest, ModuleResponse, ModuleScope,
    ModuleStatusDto,
};
use crate::error::{
    CommandError, ERR_MODULE_ACCESS_DENIED, ERR_MODULE_INVALID_REQUEST, ERR_MODULE_NOT_ACTIVE,
    ERR_MODULE_NOT_INSTALLED, ERR_MODULE_ROLE_DENIED,
};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Versión del protocolo que entiende este host. Si cambia la forma de
/// `ModuleRequest`/`ModuleResponse`/`ModuleContext`, se incrementa.
pub const MODULE_PROTOCOL_VERSION: u16 = 1;

pub struct PluginHost {
    app_data_dir: PathBuf,
    modules: HashMap<String, Box<dyn Module>>,
    dbs: Mutex<HashMap<String, Arc<ModuleDb>>>,
}

impl PluginHost {
    pub fn new(app_data_dir: PathBuf, modules: Vec<Box<dyn Module>>) -> Self {
        let modules = modules
            .into_iter()
            .map(|module| (module.descriptor().id.clone(), module))
            .collect();
        Self {
            app_data_dir,
            modules,
            dbs: Mutex::new(HashMap::new()),
        }
    }

    pub fn is_installed(&self, module_id: &str) -> bool {
        self.modules.contains_key(module_id)
    }

    pub fn descriptor(&self, module_id: &str) -> Option<ModuleDescriptor> {
        self.modules
            .get(module_id)
            .map(|module| module.descriptor())
    }

    fn module(&self, module_id: &str) -> Option<&dyn Module> {
        self.modules
            .get(module_id)
            .map(|module| module.as_ref())
    }

    /// Abre (lazy) la DB propia del módulo y corre `migrate()` en una
    /// transacción. Idempotente: si ya está abierta, devuelve el handle.
    pub fn open_db(&self, module_id: &str) -> Result<Arc<ModuleDb>, CommandError> {
        let module = self
            .modules
            .get(module_id)
            .map(|module| module.as_ref())
            .ok_or_else(|| CommandError::new(ERR_MODULE_NOT_INSTALLED))?;

        let mut dbs = self
            .dbs
            .lock()
            .map_err(|_| CommandError::internal("Lock poisoned"))?;
        if let Some(db) = dbs.get(module_id) {
            return Ok(db.clone());
        }

        let path = self
            .app_data_dir
            .join(format!("fichio-{}.db", module_id));
        let mut conn = Connection::open(&path).map_err(|e| {
            CommandError::internal(format!(
                "No se pudo abrir la DB del módulo {}: {}",
                module_id, e
            ))
        })?;
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;",
        )
        .map_err(|e| {
            CommandError::internal(format!("PRAGMAs de la DB del módulo {}: {}", module_id, e))
        })?;

        let tx = conn
            .transaction()
            .map_err(|e| CommandError::internal(e.to_string()))?;
        module.migrate(&tx)?;
        tx.commit()
            .map_err(|e| CommandError::internal(e.to_string()))?;

        let db = Arc::new(ModuleDb::new(conn));
        dbs.insert(module_id.to_string(), db.clone());
        Ok(db)
    }

    fn close_db(&self, module_id: &str) -> Result<(), CommandError> {
        let mut dbs = self
            .dbs
            .lock()
            .map_err(|_| CommandError::internal("Lock poisoned"))?;
        dbs.remove(module_id);
        Ok(())
    }

    /// Vuelca el WAL de todas las DBs de módulo abiertas antes de un respaldo.
    pub fn checkpoint_open_dbs(&self) -> Result<(), CommandError> {
        let dbs = self
            .dbs
            .lock()
            .map_err(|_| CommandError::internal("Lock poisoned"))?;
        for (id, db) in dbs.iter() {
            db.with_conn(|conn| {
                crate::security::backup::checkpoint(conn).map_err(|e| {
                    crate::core::module::ModuleError::Internal(format!(
                        "checkpoint de módulo {}: {}",
                        id, e
                    ))
                })
            })?;
        }
        Ok(())
    }

    /// Cierra todas las conexiones de módulos abiertas (usado por
    /// `restore_backup` antes de reemplazar archivos en disco).
    pub fn close_all_dbs(&self) -> Result<(), CommandError> {
        let mut dbs = self
            .dbs
            .lock()
            .map_err(|_| CommandError::internal("Lock poisoned"))?;
        dbs.clear();
        Ok(())
    }

    /// Validación del gate + apertura de la DB del módulo. **Síncrono** para
    /// que el `MutexGuard` de la DB del Core no cruce un `.await`.
    ///
    /// Orden estricto (`module-contract.md` §5): `descriptor` presente →
    /// `state == 'active'` → `role ∈ allowed_roles` → `module_access` (si
    /// aplica) → `protocol_version`.
    pub fn resolve_dispatch(
        &self,
        core_conn: &Connection,
        module_id: &str,
        req: &ModuleRequest,
        actor: &Actor,
    ) -> Result<Arc<ModuleDb>, CommandError> {
        let descriptor = self
            .descriptor(module_id)
            .ok_or_else(|| CommandError::new(ERR_MODULE_NOT_INSTALLED))?;

        if module_state(core_conn, module_id)?.as_deref() != Some("active") {
            return Err(CommandError::new(ERR_MODULE_NOT_ACTIVE));
        }

        if !descriptor.allowed_roles.contains(&actor.role) {
            return Err(CommandError::new(ERR_MODULE_ROLE_DENIED));
        }

        if descriptor.scope == ModuleScope::InstallationAndProfessional
            && !has_module_access(core_conn, module_id, &actor.user_id)?
        {
            return Err(CommandError::new(ERR_MODULE_ACCESS_DENIED));
        }

        if req.protocol_version != MODULE_PROTOCOL_VERSION {
            return Err(CommandError::new(ERR_MODULE_INVALID_REQUEST).with_param(
                "detail",
                format!(
                    "protocol_version {} no soportada (esperada {})",
                    req.protocol_version, MODULE_PROTOCOL_VERSION
                ),
            ));
        }

        self.open_db(module_id)
    }

    /// Ejecuta el módulo ya resuelto por el gate. Async porque el trait lo
    /// es; no toma la conexión del Core.
    pub async fn run(
        &self,
        module_id: &str,
        db: &ModuleDb,
        req: ModuleRequest,
        ctx: &dyn crate::core::module::ModuleContext,
    ) -> Result<ModuleResponse, CommandError> {
        let module = self
            .module(module_id)
            .ok_or_else(|| CommandError::new(ERR_MODULE_NOT_INSTALLED))?;
        module.handle(req, db, ctx).await.map_err(CommandError::from)
    }

    /// `true` si el módulo está instalado, activo y el actor puede usarlo.
    pub fn is_module_available(
        &self,
        core_conn: &Connection,
        module_id: &str,
        actor: &Actor,
    ) -> Result<bool, CommandError> {
        let Some(descriptor) = self.descriptor(module_id) else {
            return Ok(false);
        };
        if module_state(core_conn, module_id)?.as_deref() != Some("active") {
            return Ok(false);
        }
        if !descriptor.allowed_roles.contains(&actor.role) {
            return Ok(false);
        }
        if descriptor.scope == ModuleScope::InstallationAndProfessional
            && !has_module_access(core_conn, module_id, &actor.user_id)?
        {
            return Ok(false);
        }
        Ok(true)
    }

    pub fn activate(
        &self,
        core_conn: &Connection,
        module_id: &str,
        admin_id: &str,
    ) -> Result<(), CommandError> {
        if !self.is_installed(module_id) {
            return Err(CommandError::new(ERR_MODULE_NOT_INSTALLED));
        }

        // Migra antes de marcar activo: si falla, no se activa.
        self.open_db(module_id)?;

        core_conn
            .execute(
                "INSERT INTO modules (id, state, activated_at, activated_by)
                 VALUES (?1, 'active', datetime('now'), ?2)
                 ON CONFLICT(id) DO UPDATE SET
                     state = 'active',
                     activated_at = excluded.activated_at,
                     activated_by = excluded.activated_by",
                params![module_id, admin_id],
            )
            .map_err(|e| e.to_string())?;

        audit(
            core_conn,
            admin_id,
            &AuditEvent::ModuleActivated {
                module_id: module_id.to_string(),
            },
        )?;
        Ok(())
    }

    pub fn deactivate(
        &self,
        core_conn: &Connection,
        module_id: &str,
        admin_id: &str,
    ) -> Result<(), CommandError> {
        if !self.is_installed(module_id) {
            return Err(CommandError::new(ERR_MODULE_NOT_INSTALLED));
        }

        // Desactivar es "pausar y ocultar", no borrar: la DB queda en disco.
        core_conn
            .execute(
                "UPDATE modules SET state = 'inactive' WHERE id = ?1",
                params![module_id],
            )
            .map_err(|e| e.to_string())?;
        self.close_db(module_id)?;

        audit(
            core_conn,
            admin_id,
            &AuditEvent::ModuleDeactivated {
                module_id: module_id.to_string(),
            },
        )?;
        Ok(())
    }

    /// Lista los módulos compilados con su estado, ordenados por id.
    pub fn list_modules(
        &self,
        core_conn: &Connection,
    ) -> Result<Vec<ModuleStatusDto>, CommandError> {
        let mut out = Vec::with_capacity(self.modules.len());
        for module in self.modules.values() {
            let descriptor = module.descriptor();
            let state = module_state(core_conn, &descriptor.id)?
                .unwrap_or_else(|| "inactive".to_string());
            out.push(ModuleStatusDto {
                id: descriptor.id,
                name: descriptor.name,
                version: descriptor.version,
                scope: descriptor.scope,
                allowed_roles: descriptor.allowed_roles,
                state,
            });
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }
}

fn module_state(
    conn: &Connection,
    module_id: &str,
) -> Result<Option<String>, CommandError> {
    conn.query_row(
        "SELECT state FROM modules WHERE id = ?1",
        params![module_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| CommandError::internal(e.to_string()))
}

fn has_module_access(
    conn: &Connection,
    module_id: &str,
    user_id: &str,
) -> Result<bool, CommandError> {
    conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM module_access WHERE module_id = ?1 AND user_id = ?2
         )",
        params![module_id, user_id],
        |row| row.get(0),
    )
    .map_err(|e| CommandError::internal(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::dto::{ActorRole, FiliationDto, ProfessionalDto};
    use crate::core::module::{ModuleContext, ModuleError};
    use crate::error::ERR_MODULE_UNSUPPORTED;
    use async_trait::async_trait;
    use futures_executor::block_on;
    use zeroize::Zeroizing;

    struct TestModule {
        id: &'static str,
        scope: ModuleScope,
        allowed_roles: Vec<ActorRole>,
    }

    impl TestModule {
        fn boxed(id: &'static str, scope: ModuleScope) -> Box<dyn Module> {
            Box::new(Self {
                id,
                scope,
                allowed_roles: vec![ActorRole::Profesional],
            })
        }
    }

    #[async_trait]
    impl Module for TestModule {
        fn descriptor(&self) -> ModuleDescriptor {
            ModuleDescriptor {
                id: self.id.to_string(),
                name: format!("Módulo {}", self.id),
                version: "0.1.0".to_string(),
                scope: self.scope,
                allowed_roles: self.allowed_roles.clone(),
            }
        }

        fn migrate(&self, conn: &Connection) -> Result<(), ModuleError> {
            conn.execute_batch("CREATE TABLE IF NOT EXISTS items (id INTEGER PRIMARY KEY)")
                .map_err(|e| ModuleError::Internal(e.to_string()))
        }

        async fn handle(
            &self,
            req: ModuleRequest,
            db: &ModuleDb,
            ctx: &dyn ModuleContext,
        ) -> Result<ModuleResponse, ModuleError> {
            db.with_tx(|tx| {
                tx.execute("INSERT INTO items DEFAULT VALUES", [])
                    .map_err(|e| ModuleError::Internal(e.to_string()))?;
                Ok(())
            })?;
            Ok(ModuleResponse {
                payload: serde_json::json!({
                    "action": req.action,
                    "actor": ctx.actor().user_id,
                }),
            })
        }
    }

    struct TestContext {
        actor: Actor,
    }

    #[async_trait]
    impl ModuleContext for TestContext {
        fn actor(&self) -> &Actor {
            &self.actor
        }

        async fn fetch_filiation(&self, _entity_id: i64) -> Result<FiliationDto, CommandError> {
            Err(CommandError::new(ERR_MODULE_UNSUPPORTED))
        }

        async fn entity_exists(&self, _entity_id: i64) -> Result<bool, CommandError> {
            Ok(false)
        }

        async fn fetch_professional(
            &self,
            _user_id: &str,
        ) -> Result<ProfessionalDto, CommandError> {
            Err(CommandError::new(ERR_MODULE_UNSUPPORTED))
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

        async fn fingerprint_with_datakey(
            &self,
            domain: &str,
            _data: &[u8],
        ) -> Result<String, CommandError> {
            Ok(format!("hmac-sha256:test-{}", domain))
        }
    }

    fn core_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory core db");
        conn.execute_batch(
            r#"
            CREATE TABLE users (
                id TEXT PRIMARY KEY, username TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL, role TEXT NOT NULL,
                entity_id INTEGER, created_at TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1, last_login_at TEXT
            );
            CREATE TABLE modules (
                id TEXT PRIMARY KEY NOT NULL,
                state TEXT NOT NULL DEFAULT 'inactive'
                    CHECK(state IN ('inactive','active','uninstalled')),
                activated_at TEXT, activated_by TEXT
            );
            CREATE TABLE module_access (
                module_id TEXT NOT NULL, user_id TEXT NOT NULL,
                granted_at TEXT NOT NULL, granted_by TEXT NOT NULL,
                PRIMARY KEY (module_id, user_id)
            );
            CREATE TABLE audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                actor_user_id TEXT, action TEXT NOT NULL,
                target_user_id TEXT, detail TEXT, created_at TEXT NOT NULL
            );
            INSERT INTO users (id, username, password_hash, role, created_at)
            VALUES ('admin', 'admin', 'hash', 'administrador', '2026-01-01');
            "#,
        )
        .expect("ddl core");
        conn
    }

    fn host_with(modules: Vec<Box<dyn Module>>) -> (PluginHost, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        (
            PluginHost::new(dir.path().to_path_buf(), modules),
            dir,
        )
    }

    fn actor(role: ActorRole) -> Actor {
        Actor {
            user_id: "user-1".to_string(),
            role,
        }
    }

    fn request(protocol_version: u16) -> ModuleRequest {
        ModuleRequest {
            protocol_version,
            action: "ping".to_string(),
            entity_id: Some(1),
            payload: serde_json::json!({}),
        }
    }

    #[test]
    fn gate_rejects_not_installed_inactive_and_bad_protocol() {
        let conn = core_db();
        let (host, _dir) = host_with(vec![TestModule::boxed("test", ModuleScope::Installation)]);
        let professional = actor(ActorRole::Profesional);

        let err = host
            .resolve_dispatch(&conn, "nope", &request(1), &professional)
            .expect_err("no instalado");
        assert_eq!(err.code, ERR_MODULE_NOT_INSTALLED);

        let err = host
            .resolve_dispatch(&conn, "test", &request(2), &professional)
            .expect_err("inactivo incluso con protocolo inválido");
        assert_eq!(err.code, ERR_MODULE_NOT_ACTIVE);

        host.activate(&conn, "test", "admin").expect("activate");

        let err = host
            .resolve_dispatch(&conn, "test", &request(2), &actor(ActorRole::Asistente))
            .expect_err("rol denegado antes que protocolo");
        assert_eq!(err.code, ERR_MODULE_ROLE_DENIED);

        let err = host
            .resolve_dispatch(&conn, "test", &request(2), &professional)
            .expect_err("protocolo no soportado");
        assert_eq!(err.code, ERR_MODULE_INVALID_REQUEST);
    }

    #[test]
    fn gate_checks_role_and_dispatches_to_module() {
        let conn = core_db();
        let (host, _dir) = host_with(vec![TestModule::boxed("test", ModuleScope::Installation)]);
        let professional = actor(ActorRole::Profesional);

        host.activate(&conn, "test", "admin").expect("activate");

        let err = host
            .resolve_dispatch(&conn, "test", &request(1), &actor(ActorRole::Asistente))
            .expect_err("rol denegado");
        assert_eq!(err.code, ERR_MODULE_ROLE_DENIED);

        let db = host
            .resolve_dispatch(&conn, "test", &request(1), &professional)
            .expect("gate ok");
        let ctx = TestContext {
            actor: professional,
        };
        let response = block_on(host.run("test", &db, request(1), &ctx)).expect("run");
        assert_eq!(response.payload["action"], "ping");
        assert_eq!(response.payload["actor"], "user-1");

        // El módulo escribió en su propia DB (no en la del Core).
        let rows: i64 = db
            .with_conn(|conn| {
                conn.query_row("SELECT COUNT(*) FROM items", [], |row| row.get(0))
                    .map_err(|e| ModuleError::Internal(e.to_string()))
            })
            .expect("count");
        assert_eq!(rows, 1);
    }

    #[test]
    fn installation_and_professional_requires_explicit_access() {
        let conn = core_db();
        let (host, _dir) = host_with(vec![TestModule::boxed(
            "scoped",
            ModuleScope::InstallationAndProfessional,
        )]);
        let professional = actor(ActorRole::Profesional);
        host.activate(&conn, "scoped", "admin").expect("activate");

        let err = host
            .resolve_dispatch(&conn, "scoped", &request(1), &professional)
            .expect_err("sin acceso");
        assert_eq!(err.code, ERR_MODULE_ACCESS_DENIED);

        conn.execute(
            "INSERT INTO module_access (module_id, user_id, granted_at, granted_by)
             VALUES ('scoped', 'user-1', '2026-01-01', 'admin')",
            [],
        )
        .expect("grant");

        assert!(
            host.is_module_available(&conn, "scoped", &professional)
                .expect("available")
        );
        host.resolve_dispatch(&conn, "scoped", &request(1), &professional)
            .expect("con acceso");
    }

    #[test]
    fn deactivate_blocks_dispatch_and_audits() {
        let conn = core_db();
        let (host, _dir) = host_with(vec![TestModule::boxed("test", ModuleScope::Installation)]);
        let professional = actor(ActorRole::Profesional);

        host.activate(&conn, "test", "admin").expect("activate");
        let (action, detail): (String, String) = conn
            .query_row(
                "SELECT action, detail FROM audit_log ORDER BY id DESC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("audit");
        assert_eq!(action, "module_activated");
        assert_eq!(detail, "module_id=test");

        host.deactivate(&conn, "test", "admin").expect("deactivate");
        let (action, _): (String, String) = conn
            .query_row(
                "SELECT action, detail FROM audit_log ORDER BY id DESC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("audit");
        assert_eq!(action, "module_deactivated");

        let err = host
            .resolve_dispatch(&conn, "test", &request(1), &professional)
            .expect_err("desactivado");
        assert_eq!(err.code, ERR_MODULE_NOT_ACTIVE);

        assert!(
            !host
                .is_module_available(&conn, "test", &professional)
                .expect("available")
        );
    }

    #[test]
    fn list_modules_reports_state_sorted() {
        let conn = core_db();
        let (host, _dir) = host_with(vec![
            TestModule::boxed("zeta", ModuleScope::Installation),
            TestModule::boxed("alfa", ModuleScope::Installation),
        ]);

        host.activate(&conn, "alfa", "admin").expect("activate");
        let modules = host.list_modules(&conn).expect("list");

        assert_eq!(modules.len(), 2);
        assert_eq!(modules[0].id, "alfa");
        assert_eq!(modules[0].state, "active");
        assert_eq!(modules[1].id, "zeta");
        assert_eq!(modules[1].state, "inactive");
    }
}
