//! Comandos Tauri del contrato de módulos (`docs/module-contract.md` §5).
//!
//! `module_execute` es el **único** entry point por acción: el frontend no
//! conoce commands tipados por módulo. Acá vive el `ModuleContext`
//! in-process (Ruta 1), que toma los locks de Tauri y delega en `core::api`.
//! Todo el trabajo de DB es síncrono y suelta el guard antes de cualquier
//! `.await` (el trait es async por Ruta 2).

use crate::core::dto::{Actor, ActorRole, AuditEvent, FiliationDto, ProfessionalDto};
use crate::core::host::PluginHost;
use crate::core::module::{ModuleContext, ModuleRequest, ModuleResponse, ModuleStatusDto};
use crate::db::config_schema::SchemaConfig;
use crate::error::{CommandError, ERR_MODULE_ROLE_DENIED};
use crate::types::{DataKey, DbState, SessionState, SessionUser};
use async_trait::async_trait;
use tauri::State;
use zeroize::Zeroizing;

use super::{get_data_key, require_admin, require_session, with_conn};

/// Actor de sesión → `Actor` del contrato. Rol desconocido = fail-closed.
fn actor_from_session(session: &SessionUser) -> Result<Actor, CommandError> {
    let role = ActorRole::from_role(&session.role)
        .ok_or_else(|| CommandError::new(ERR_MODULE_ROLE_DENIED))?;
    Ok(Actor {
        user_id: session.user_id.clone(),
        role,
    })
}

/// Implementación in-process de `ModuleContext` (Ruta 1).
///
/// Guarda los `State` de Tauri; cada método adquiere el lock, llama a
/// `core::api::*` de forma síncrona y lo suelta antes de devolver el future.
pub struct InProcessModuleContext<'a> {
    actor: Actor,
    session: SessionUser,
    db_state: State<'a, DbState>,
    data_key_state: State<'a, DataKey>,
    schema_state: State<'a, SchemaConfig>,
}

impl<'a> InProcessModuleContext<'a> {
    fn new(
        session: SessionUser,
        actor: Actor,
        db_state: State<'a, DbState>,
        data_key_state: State<'a, DataKey>,
        schema_state: State<'a, SchemaConfig>,
    ) -> Self {
        Self {
            actor,
            session,
            db_state,
            data_key_state,
            schema_state,
        }
    }
}

#[async_trait]
impl ModuleContext for InProcessModuleContext<'_> {
    fn actor(&self) -> &Actor {
        &self.actor
    }

    async fn fetch_filiation(
        &self,
        entity_id: i64,
    ) -> Result<FiliationDto, CommandError> {
        let data_key = get_data_key(&self.data_key_state)?;
        with_conn(&self.db_state, |conn| {
            crate::core::api::fetch_filiation(
                conn,
                &data_key,
                &self.schema_state,
                &self.session,
                entity_id,
            )
        })
    }

    async fn entity_exists(&self, entity_id: i64) -> Result<bool, CommandError> {
        with_conn(&self.db_state, |conn| {
            crate::core::api::entity_exists(conn, &self.session, entity_id)
        })
    }

    async fn fetch_professional(
        &self,
        user_id: &str,
    ) -> Result<ProfessionalDto, CommandError> {
        let data_key = get_data_key(&self.data_key_state)?;
        with_conn(&self.db_state, |conn| {
            crate::core::api::fetch_professional(conn, &data_key, user_id)
        })
    }

    async fn audit(&self, event: AuditEvent) -> Result<(), CommandError> {
        with_conn(&self.db_state, |conn| {
            crate::core::api::audit(conn, &self.actor.user_id, &event)
        })
    }

    async fn wrap_with_datakey(
        &self,
        plaintext: &[u8],
    ) -> Result<Vec<u8>, CommandError> {
        let data_key = get_data_key(&self.data_key_state)?;
        crate::core::api::wrap_with_datakey(&data_key, plaintext)
    }

    async fn unwrap_with_datakey(
        &self,
        ciphertext: &[u8],
    ) -> Result<Zeroizing<Vec<u8>>, CommandError> {
        let data_key = get_data_key(&self.data_key_state)?;
        crate::core::api::unwrap_with_datakey(&data_key, ciphertext)
    }
}

/// Único entry point de los módulos. El actor sale de la sesión, nunca del
/// payload. El gate corre síncrono y sin guard antes del `handle` async.
#[tauri::command]
pub async fn module_execute(
    module_id: String,
    req: ModuleRequest,
    session_state: State<'_, SessionState>,
    db_state: State<'_, DbState>,
    data_key_state: State<'_, DataKey>,
    schema_state: State<'_, SchemaConfig>,
    host: State<'_, PluginHost>,
) -> Result<ModuleResponse, CommandError> {
    let session = require_session(&session_state)?;
    let actor = actor_from_session(&session)?;

    let db = with_conn(&db_state, |conn| {
        host.resolve_dispatch(conn, &module_id, &req, &actor)
    })?;

    let ctx = InProcessModuleContext::new(
        session,
        actor,
        db_state.clone(),
        data_key_state.clone(),
        schema_state.clone(),
    );

    host.run(&module_id, &db, req, &ctx).await
}

/// Para que la UI decida si muestra el módulo (rol, scope y acceso).
#[tauri::command]
pub fn is_module_available(
    module_id: String,
    session_state: State<'_, SessionState>,
    db_state: State<'_, DbState>,
    host: State<'_, PluginHost>,
) -> Result<bool, CommandError> {
    let session = require_session(&session_state)?;
    let actor = actor_from_session(&session)?;
    with_conn(&db_state, |conn| {
        host.is_module_available(conn, &module_id, &actor)
    })
}

/// Panel de administración: módulos compilados y su estado.
#[tauri::command]
pub fn list_modules(
    session_state: State<'_, SessionState>,
    db_state: State<'_, DbState>,
    host: State<'_, PluginHost>,
) -> Result<Vec<ModuleStatusDto>, CommandError> {
    require_admin(&session_state)?;
    with_conn(&db_state, |conn| host.list_modules(conn))
}

#[tauri::command]
pub fn module_activate(
    module_id: String,
    session_state: State<'_, SessionState>,
    db_state: State<'_, DbState>,
    host: State<'_, PluginHost>,
) -> Result<(), CommandError> {
    let admin = require_admin(&session_state)?;
    with_conn(&db_state, |conn| {
        host.activate(conn, &module_id, &admin.user_id)
    })
}

#[tauri::command]
pub fn module_deactivate(
    module_id: String,
    session_state: State<'_, SessionState>,
    db_state: State<'_, DbState>,
    host: State<'_, PluginHost>,
) -> Result<(), CommandError> {
    let admin = require_admin(&session_state)?;
    with_conn(&db_state, |conn| {
        host.deactivate(conn, &module_id, &admin.user_id)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::module::{
        Module, ModuleDb, ModuleDescriptor, ModuleError, ModuleScope,
    };
    use crate::db::database::init_db;
    use crate::error::{ERR_MODULE_NOT_ACTIVE, ERR_VAULT_LOCKED};
    use crate::security::crypto;
    use futures_executor::block_on;
    use rusqlite::{params, Connection};
    use std::collections::HashMap;
    use std::sync::Mutex;
    use tauri::Manager;

    const DATA_KEY: [u8; 32] = [5u8; 32];

    struct E2eModule;

    #[async_trait]
    impl Module for E2eModule {
        fn descriptor(&self) -> ModuleDescriptor {
            ModuleDescriptor {
                id: "e2e".to_string(),
                name: "Módulo E2E".to_string(),
                version: "0.1.0".to_string(),
                scope: ModuleScope::Installation,
                allowed_roles: vec![ActorRole::Profesional],
            }
        }

        fn migrate(&self, conn: &Connection) -> Result<(), ModuleError> {
            conn.execute_batch("CREATE TABLE IF NOT EXISTS notes (id INTEGER PRIMARY KEY, enc BLOB)")
                .map_err(|e| ModuleError::Internal(e.to_string()))
        }

        async fn handle(
            &self,
            req: ModuleRequest,
            db: &ModuleDb,
            ctx: &dyn ModuleContext,
        ) -> Result<ModuleResponse, ModuleError> {
            let entity_id = req
                .entity_id
                .ok_or_else(|| ModuleError::InvalidRequest("entity_id requerido".into()))?;
            let filiation = ctx.fetch_filiation(entity_id).await?;
            let prof = ctx.fetch_professional(&ctx.actor().user_id).await?;
            let wrapped = ctx.wrap_with_datakey(b"secreto-modulo").await?;
            let unwrapped = ctx.unwrap_with_datakey(&wrapped).await?;
            assert_eq!(&*unwrapped, b"secreto-modulo");

            db.with_tx(|tx| {
                tx.execute("INSERT INTO notes (enc) VALUES (?1)", params![wrapped])
                    .map_err(|e| ModuleError::Internal(e.to_string()))?;
                Ok(())
            })?;

            ctx.audit(AuditEvent::SolicitudIssued {
                subject_id: entity_id,
                solicitud_id: "sol-e2e".to_string(),
                sha256: "deadbeef".to_string(),
            })
            .await?;

            Ok(ModuleResponse {
                payload: serde_json::json!({
                    "patient": filiation.full_name,
                    "professional": prof.full_name,
                }),
            })
        }
    }

    fn test_schema() -> SchemaConfig {
        serde_json::from_str(
            r#"{
                "version": 1,
                "default_entity": "patient",
                "entities": {
                    "patient": {
                        "label": "Paciente",
                        "label_plural": "Pacientes",
                        "blind_index_field": "dni",
                        "fields": [
                            {"name": "nombre", "label": "Nombre", "type": "text",
                             "required": true, "encrypted": true},
                            {"name": "apellido", "label": "Apellido", "type": "text",
                             "required": true, "encrypted": true},
                            {"name": "dni", "label": "DNI", "type": "text",
                             "required": true, "encrypted": false, "blind_index": true}
                        ]
                    }
                }
            }"#,
        )
        .expect("schema fixture")
    }

    #[test]
    fn e2e_activate_execute_with_real_context_propagate_vault_locked_and_deactivate() {
        let dir = tempfile::tempdir().expect("tempdir");
        let conn = init_db(dir.path().to_path_buf()).expect("init_db");

        conn.execute(
            "INSERT INTO users (id, username, password_hash, role, created_at)
             VALUES ('admin-1', 'admin', 'h', 'administrador', '2026-01-01'),
                    ('prof-1', 'dra', 'h', 'profesional', '2026-01-01')",
            [],
        )
        .expect("insert users");

        let enc_name = crypto::encrypt_text("Dra. Ana", &DATA_KEY).unwrap();
        let enc_prof = crypto::encrypt_text("Médica", &DATA_KEY).unwrap();
        conn.execute(
            "INSERT INTO professional_profiles
                (user_id, full_name_ciphertext, full_name_nonce,
                 specialty_ciphertext, specialty_nonce,
                 profession_ciphertext, profession_nonce,
                 public_key, updated_at)
             VALUES ('prof-1', ?1, ?2, '', '', ?3, ?4, 'pk-1', '2026-01-01')",
            params![
                enc_name.ciphertext,
                enc_name.nonce,
                enc_prof.ciphertext,
                enc_prof.nonce
            ],
        )
        .expect("insert profile");

        let mut fields = HashMap::new();
        for (k, v) in [("nombre", "Juan"), ("apellido", "Pérez"), ("dni", "20111222")] {
            let e = crypto::encrypt_text(v, &DATA_KEY).unwrap();
            fields.insert(k.to_string(), format!("{}:{}", e.ciphertext, e.nonce));
        }
        let blob = serde_json::to_vec(&fields).unwrap();
        conn.execute(
            "INSERT INTO entities
                (id, external_id, entity_type, created_by_user_id, blind_index, enc_data_blob, created_at)
             VALUES (1, 'ext-1', 'patient', 'prof-1', 'bi-1', ?1, '2026-01-01')",
            params![blob],
        )
        .expect("insert entity");

        let app = tauri::test::mock_app();
        app.manage(DbState(Mutex::new(Some(conn))));
        app.manage(DataKey(Mutex::new(Some(Zeroizing::new(DATA_KEY)))));
        app.manage(test_schema());
        app.manage(PluginHost::new(
            dir.path().to_path_buf(),
            vec![Box::new(E2eModule)],
        ));
        app.manage(SessionState(Mutex::new(Some(SessionUser {
            user_id: "admin-1".into(),
            username: "admin".into(),
            role: "administrador".into(),
        }))));

        // 1. Admin activa el módulo
        module_activate(
            "e2e".into(),
            app.state::<SessionState>(),
            app.state::<DbState>(),
            app.state::<PluginHost>(),
        )
        .expect("activate");

        let listed = list_modules(
            app.state::<SessionState>(),
            app.state::<DbState>(),
            app.state::<PluginHost>(),
        )
        .expect("list_modules");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].state, "active");

        // 2. Cambiamos sesión a profesional y ejecutamos end-to-end
        *app.state::<SessionState>().0.lock().unwrap() = Some(SessionUser {
            user_id: "prof-1".into(),
            username: "dra".into(),
            role: "profesional".into(),
        });

        assert!(is_module_available(
            "e2e".into(),
            app.state::<SessionState>(),
            app.state::<DbState>(),
            app.state::<PluginHost>(),
        )
        .expect("available"));

        let req = ModuleRequest {
            protocol_version: 1,
            action: "run".into(),
            entity_id: Some(1),
            payload: serde_json::json!({}),
        };

        let res = block_on(module_execute(
            "e2e".into(),
            req.clone(),
            app.state::<SessionState>(),
            app.state::<DbState>(),
            app.state::<DataKey>(),
            app.state::<SchemaConfig>(),
            app.state::<PluginHost>(),
        ))
        .expect("execute ok");

        assert_eq!(res.payload["patient"], "Juan Pérez");
        assert_eq!(res.payload["professional"], "Dra. Ana");

        // 3. Si la bóveda se bloquea (DataKey = None), propaga ERR_VAULT_LOCKED
        *app.state::<DataKey>().0.lock().unwrap() = None;
        let err = block_on(module_execute(
            "e2e".into(),
            req.clone(),
            app.state::<SessionState>(),
            app.state::<DbState>(),
            app.state::<DataKey>(),
            app.state::<SchemaConfig>(),
            app.state::<PluginHost>(),
        ))
        .expect_err("bóveda cerrada");
        assert_eq!(err.code, ERR_VAULT_LOCKED);

        // 4. Admin desactiva → module_execute devuelve ERR_MODULE_NOT_ACTIVE
        *app.state::<SessionState>().0.lock().unwrap() = Some(SessionUser {
            user_id: "admin-1".into(),
            username: "admin".into(),
            role: "administrador".into(),
        });
        module_deactivate(
            "e2e".into(),
            app.state::<SessionState>(),
            app.state::<DbState>(),
            app.state::<PluginHost>(),
        )
        .expect("deactivate");

        *app.state::<SessionState>().0.lock().unwrap() = Some(SessionUser {
            user_id: "prof-1".into(),
            username: "dra".into(),
            role: "profesional".into(),
        });
        let err = block_on(module_execute(
            "e2e".into(),
            req,
            app.state::<SessionState>(),
            app.state::<DbState>(),
            app.state::<DataKey>(),
            app.state::<SchemaConfig>(),
            app.state::<PluginHost>(),
        ))
        .expect_err("desactivado");
        assert_eq!(err.code, ERR_MODULE_NOT_ACTIVE);
    }
}
