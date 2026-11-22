//! Módulo de solicitud de estudios complementarios (`docs/MODULES/module-solicitudes-v1.md`).
//!
//! Ruta 1: módulo interno compilado (feature `module-solicitudes`). Datos
//! operativos en `fichio-solicitudes.db`; filiación/profesional solo por
//! `ModuleContext`; el resultado clínico sigue en la SOAP del Core.

mod models;
mod render;
mod store;

use crate::core::dto::{ActorRole, AuditEvent};
use crate::core::module::{
    Module, ModuleContext, ModuleDb, ModuleDescriptor, ModuleError, ModuleRequest,
    ModuleResponse, ModuleScope,
};
use async_trait::async_trait;
use models::TemplateDefinition;
use render::render_solicitud;
use rusqlite::Connection;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use uuid::Uuid;

pub struct SolicitudesModule;

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn require_str<'a>(payload: &'a Value, key: &str) -> Result<&'a str, ModuleError> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ModuleError::InvalidRequest(format!("falta '{}'", key)))
}

fn parse_fields(payload: &Value) -> Result<TemplateDefinition, ModuleError> {
    let raw = payload
        .get("fields")
        .ok_or_else(|| ModuleError::InvalidRequest("falta 'fields'".to_string()))?;
    let fields: Vec<models::TemplateField> = serde_json::from_value(raw.clone())
        .map_err(|e| ModuleError::InvalidRequest(format!("'fields' inválido: {}", e)))?;
    Ok(TemplateDefinition { fields })
}

fn parse_values(payload: &Value) -> Result<BTreeMap<String, String>, ModuleError> {
    let raw = payload.get("values").cloned().unwrap_or_else(|| json!({}));
    serde_json::from_value(raw).map_err(|_| {
        ModuleError::InvalidRequest("'values' debe ser un objeto de strings".to_string())
    })
}

fn to_payload<T: serde::Serialize>(value: T) -> Result<Value, ModuleError> {
    serde_json::to_value(value).map_err(|e| ModuleError::Internal(e.to_string()))
}

async fn template_create(
    req: &ModuleRequest,
    db: &ModuleDb,
    ctx: &dyn ModuleContext,
) -> Result<ModuleResponse, ModuleError> {
    let name = require_str(&req.payload, "name")?;
    let definition = parse_fields(&req.payload)?;
    definition.validate()?;

    let id = Uuid::new_v4().to_string();
    store::insert_template(db, &id, &ctx.actor().user_id, name, &definition, &now())?;
    ctx.audit(AuditEvent::SolicitudTemplateSaved {
        template_id: id.clone(),
    })
    .await?;

    Ok(ModuleResponse {
        payload: json!({ "id": id }),
    })
}

async fn template_update(
    req: &ModuleRequest,
    db: &ModuleDb,
    ctx: &dyn ModuleContext,
) -> Result<ModuleResponse, ModuleError> {
    let id = require_str(&req.payload, "id")?;
    let name = require_str(&req.payload, "name")?;
    let definition = parse_fields(&req.payload)?;
    definition.validate()?;

    let updated =
        store::update_template(db, id, &ctx.actor().user_id, name, &definition, &now())?;
    if !updated {
        return Err(ModuleError::InvalidRequest(
            "template no encontrado".to_string(),
        ));
    }
    ctx.audit(AuditEvent::SolicitudTemplateSaved {
        template_id: id.to_string(),
    })
    .await?;

    Ok(ModuleResponse {
        payload: json!({ "updated": true }),
    })
}

async fn template_delete(
    req: &ModuleRequest,
    db: &ModuleDb,
    ctx: &dyn ModuleContext,
) -> Result<ModuleResponse, ModuleError> {
    let id = require_str(&req.payload, "id")?;
    let deleted = store::delete_template(db, id, &ctx.actor().user_id)?;
    if !deleted {
        return Err(ModuleError::InvalidRequest(
            "template no encontrado".to_string(),
        ));
    }
    Ok(ModuleResponse {
        payload: json!({ "deleted": true }),
    })
}

async fn template_list(
    _req: &ModuleRequest,
    db: &ModuleDb,
    ctx: &dyn ModuleContext,
) -> Result<ModuleResponse, ModuleError> {
    let templates = store::list_templates(db, &ctx.actor().user_id)?;
    Ok(ModuleResponse {
        payload: to_payload(templates)?,
    })
}

async fn solicitud_create(
    req: &ModuleRequest,
    db: &ModuleDb,
    ctx: &dyn ModuleContext,
) -> Result<ModuleResponse, ModuleError> {
    let entity_id = req
        .entity_id
        .or_else(|| req.payload.get("entity_id").and_then(Value::as_i64))
        .ok_or_else(|| ModuleError::InvalidRequest("falta 'entity_id'".to_string()))?;

    if !ctx.entity_exists(entity_id).await? {
        return Err(ModuleError::InvalidRequest(
            "ficha no encontrada".to_string(),
        ));
    }

    let values = parse_values(&req.payload)?;
    let mut study_name = req
        .payload
        .get("study_name")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("")
        .to_string();
    let template_id = req
        .payload
        .get("template_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    if let Some(template_id) = &template_id {
        let template = store::load_template(db, template_id, &ctx.actor().user_id)?
            .ok_or_else(|| {
                ModuleError::InvalidRequest("template no encontrado".to_string())
            })?;
        if study_name.is_empty() {
            study_name = template.name.clone();
        }
        template.fields.validate_values(&values)?;
    }

    if study_name.is_empty() {
        return Err(ModuleError::InvalidRequest(
            "falta 'study_name' o un template".to_string(),
        ));
    }

    let plaintext = serde_json::to_vec(&values)
        .map_err(|e| ModuleError::Internal(e.to_string()))?;
    let values_enc = ctx.wrap_with_datakey(&plaintext).await?;
    let sha256 = hex::encode(Sha256::digest(&plaintext));

    let id = Uuid::new_v4().to_string();
    let created_at = now();
    store::insert_solicitud(
        db,
        store::NewSolicitud {
            id: &id,
            entity_id,
            owner: &ctx.actor().user_id,
            study_name: &study_name,
            template_id: template_id.as_deref(),
            values_enc: &values_enc,
            now: &created_at,
        },
    )?;

    ctx.audit(AuditEvent::SolicitudIssued {
        subject_id: entity_id,
        solicitud_id: id.clone(),
        sha256,
    })
    .await?;

    Ok(ModuleResponse {
        payload: json!({ "id": id }),
    })
}

async fn solicitud_list(
    req: &ModuleRequest,
    db: &ModuleDb,
    ctx: &dyn ModuleContext,
) -> Result<ModuleResponse, ModuleError> {
    let entity_id = req
        .entity_id
        .or_else(|| req.payload.get("entity_id").and_then(Value::as_i64));
    let solicitudes = store::list_solicitudes(db, &ctx.actor().user_id, entity_id)?;
    Ok(ModuleResponse {
        payload: to_payload(solicitudes)?,
    })
}

fn decode_values(plaintext: &[u8]) -> Result<BTreeMap<String, String>, ModuleError> {
    serde_json::from_slice(plaintext).map_err(|e| ModuleError::Internal(e.to_string()))
}

async fn solicitud_get(
    req: &ModuleRequest,
    db: &ModuleDb,
    ctx: &dyn ModuleContext,
) -> Result<ModuleResponse, ModuleError> {
    let id = require_str(&req.payload, "id")?;
    let record = store::load_solicitud(db, id, &ctx.actor().user_id)?.ok_or_else(|| {
        ModuleError::InvalidRequest("solicitud no encontrada".to_string())
    })?;

    let plaintext = ctx.unwrap_with_datakey(&record.values_enc).await?;
    let values = decode_values(&plaintext)?;

    Ok(ModuleResponse {
        payload: json!({
            "solicitud": record.summary,
            "values": values,
        }),
    })
}

async fn solicitud_render(
    req: &ModuleRequest,
    db: &ModuleDb,
    ctx: &dyn ModuleContext,
) -> Result<ModuleResponse, ModuleError> {
    let id = require_str(&req.payload, "id")?;
    let record = store::load_solicitud(db, id, &ctx.actor().user_id)?.ok_or_else(|| {
        ModuleError::InvalidRequest("solicitud no encontrada".to_string())
    })?;

    let plaintext = ctx.unwrap_with_datakey(&record.values_enc).await?;
    let values = decode_values(&plaintext)?;

    let template = match &record.summary.template_id {
        Some(template_id) => store::load_template(db, template_id, &ctx.actor().user_id)?
            .map(|template| template.fields),
        None => None,
    };

    let filiation = ctx.fetch_filiation(record.summary.entity_id).await?;
    let professional = ctx.fetch_professional(&ctx.actor().user_id).await?;
    let markdown = render_solicitud(
        &filiation,
        &professional,
        &record.summary.study_name,
        &values,
        template.as_ref(),
        &record.summary.created_at,
    );

    Ok(ModuleResponse {
        payload: json!({ "markdown": markdown }),
    })
}

#[async_trait]
impl Module for SolicitudesModule {
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            id: "solicitudes".to_string(),
            name: "Solicitud de estudios complementarios".to_string(),
            version: "0.1.0".to_string(),
            scope: ModuleScope::Installation,
            allowed_roles: vec![ActorRole::Profesional],
        }
    }

    fn migrate(&self, conn: &Connection) -> Result<(), ModuleError> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS templates (
                id            TEXT PRIMARY KEY,
                owner_user_id TEXT NOT NULL,
                name          TEXT NOT NULL,
                fields_json   TEXT NOT NULL,
                created_at    TEXT NOT NULL,
                updated_at    TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_templates_owner
                ON templates(owner_user_id);

            CREATE TABLE IF NOT EXISTS solicitudes (
                id                 TEXT PRIMARY KEY,
                entity_id          INTEGER NOT NULL,
                created_by_user_id TEXT NOT NULL,
                study_name         TEXT NOT NULL,
                template_id        TEXT,
                values_enc         BLOB NOT NULL,
                created_at         TEXT NOT NULL,
                status             TEXT NOT NULL DEFAULT 'issued'
                    CHECK (status IN ('draft', 'issued'))
            );
            CREATE INDEX IF NOT EXISTS idx_solicitudes_owner
                ON solicitudes(created_by_user_id);
            CREATE INDEX IF NOT EXISTS idx_solicitudes_entity
                ON solicitudes(entity_id);
            "#,
        )
        .map_err(|e| ModuleError::Internal(e.to_string()))
    }

    async fn handle(
        &self,
        req: ModuleRequest,
        db: &ModuleDb,
        ctx: &dyn ModuleContext,
    ) -> Result<ModuleResponse, ModuleError> {
        match req.action.as_str() {
            "template.create" => template_create(&req, db, ctx).await,
            "template.update" => template_update(&req, db, ctx).await,
            "template.delete" => template_delete(&req, db, ctx).await,
            "template.list" => template_list(&req, db, ctx).await,
            "solicitud.create" => solicitud_create(&req, db, ctx).await,
            "solicitud.list" => solicitud_list(&req, db, ctx).await,
            "solicitud.get" => solicitud_get(&req, db, ctx).await,
            "solicitud.render" => solicitud_render(&req, db, ctx).await,
            _ => Err(ModuleError::Unsupported),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::dto::{Actor, FiliationDto, License, ProfessionalDto};
    use crate::error::{CommandError, ERR_VAULT_LOCKED};
    use crate::security::crypto;
    use futures_executor::block_on;
    use rusqlite::params;
    use std::sync::Mutex;
    use zeroize::Zeroizing;

    const KEY: [u8; 32] = [3u8; 32];

    struct FakeCtx {
        actor: Actor,
        events: Mutex<Vec<AuditEvent>>,
        entity_exists: bool,
        wrap_error: Option<CommandError>,
    }

    impl FakeCtx {
        fn new(user_id: &str) -> Self {
            Self {
                actor: Actor {
                    user_id: user_id.to_string(),
                    role: ActorRole::Profesional,
                },
                events: Mutex::new(Vec::new()),
                entity_exists: true,
                wrap_error: None,
            }
        }
    }

    #[async_trait]
    impl ModuleContext for FakeCtx {
        fn actor(&self) -> &Actor {
            &self.actor
        }

        async fn fetch_filiation(
            &self,
            entity_id: i64,
        ) -> Result<FiliationDto, CommandError> {
            Ok(FiliationDto {
                entity_id,
                full_name: "Juan Pérez".to_string(),
                document: "20111222".to_string(),
                fields: Vec::new(),
            })
        }

        async fn entity_exists(&self, _entity_id: i64) -> Result<bool, CommandError> {
            Ok(self.entity_exists)
        }

        async fn fetch_professional(
            &self,
            user_id: &str,
        ) -> Result<ProfessionalDto, CommandError> {
            Ok(ProfessionalDto {
                user_id: user_id.to_string(),
                full_name: "Dra. Ana".to_string(),
                profession: "Médica".to_string(),
                specialty: "Clínica".to_string(),
                licenses: vec![License {
                    number: "MP-1".to_string(),
                    jurisdiction: "Córdoba".to_string(),
                }],
                public_key_hex: "pk".to_string(),
            })
        }

        async fn audit(&self, event: AuditEvent) -> Result<(), CommandError> {
            self.events.lock().unwrap().push(event);
            Ok(())
        }

        async fn wrap_with_datakey(
            &self,
            plaintext: &[u8],
        ) -> Result<Vec<u8>, CommandError> {
            if let Some(error) = &self.wrap_error {
                return Err(error.clone());
            }
            crypto::encrypt_bytes(plaintext, &KEY).map_err(CommandError::internal)
        }

        async fn unwrap_with_datakey(
            &self,
            ciphertext: &[u8],
        ) -> Result<Zeroizing<Vec<u8>>, CommandError> {
            crypto::decrypt_bytes(ciphertext, &KEY).map_err(CommandError::internal)
        }
    }

    fn setup_db() -> ModuleDb {
        let conn = Connection::open_in_memory().expect("in-memory");
        SolicitudesModule.migrate(&conn).expect("migrate");
        ModuleDb::new(conn)
    }

    fn fields() -> Value {
        json!([
            {"name": "estudio", "label": "Estudio", "type": "text", "required": true},
            {"name": "indicacion", "label": "Indicación", "type": "textarea", "required": false},
            {"name": "prioridad", "label": "Prioridad", "type": "select",
             "required": true, "options": ["Rutina", "Urgente"]}
        ])
    }

    fn run(
        db: &ModuleDb,
        ctx: &FakeCtx,
        action: &str,
        payload: Value,
        entity_id: Option<i64>,
    ) -> Result<Value, ModuleError> {
        let module = SolicitudesModule;
        let req = ModuleRequest {
            protocol_version: 1,
            action: action.to_string(),
            entity_id,
            payload,
        };
        block_on(module.handle(req, db, ctx)).map(|response| response.payload)
    }

    fn create_template(db: &ModuleDb, ctx: &FakeCtx, name: &str) -> String {
        let payload = json!({ "name": name, "fields": fields() });
        let response = run(db, ctx, "template.create", payload, None).expect("template.create");
        response["id"].as_str().unwrap().to_string()
    }

    #[test]
    fn template_crud_is_owner_scoped() {
        let db = setup_db();
        let owner = FakeCtx::new("user-a");
        let other = FakeCtx::new("user-b");
        let id = create_template(&db, &owner, "Rx tórax");

        assert_eq!(
            run(&db, &owner, "template.list", json!({}), None)
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(run(&db, &other, "template.list", json!({}), None)
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty());

        let update = json!({"id": id, "name": "Rx tórax 2", "fields": fields()});
        assert!(matches!(
            run(&db, &other, "template.update", update.clone(), None),
            Err(ModuleError::InvalidRequest(_))
        ));
        assert_eq!(
            run(&db, &owner, "template.update", update, None).unwrap()["updated"],
            true
        );

        assert_eq!(
            run(&db, &owner, "template.delete", json!({"id": id}), None).unwrap()["deleted"],
            true
        );
        assert!(run(&db, &owner, "template.list", json!({}), None)
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn template_validation_is_enforced_on_actions() {
        let db = setup_db();
        let ctx = FakeCtx::new("user-a");

        let empty = json!({"name": "X", "fields": []});
        assert!(matches!(
            run(&db, &ctx, "template.create", empty, None),
            Err(ModuleError::InvalidRequest(_))
        ));

        let bad_select = json!({"name": "X", "fields": [
            {"name": "prioridad", "label": "Prioridad", "type": "select", "required": true}
        ]});
        assert!(matches!(
            run(&db, &ctx, "template.create", bad_select, None),
            Err(ModuleError::InvalidRequest(_))
        ));
    }

    #[test]
    fn solicitud_create_encrypts_lists_metadata_and_audits() {
        let db = setup_db();
        let ctx = FakeCtx::new("user-a");
        let template_id = create_template(&db, &ctx, "Rx tórax");

        let created = run(
            &db,
            &ctx,
            "solicitud.create",
            json!({
                "template_id": template_id,
                "values": {
                    "estudio": "Rx tórax frente y perfil",
                    "indicacion": "secreto clinico",
                    "prioridad": "Urgente"
                }
            }),
            Some(1),
        )
        .expect("solicitud.create");
        let id = created["id"].as_str().unwrap().to_string();

        // El blob en la DB no contiene el texto plano (PHI cifrada).
        let raw: Vec<u8> = db
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT values_enc FROM solicitudes WHERE id = ?1",
                    params![id],
                    |row| row.get(0),
                )
                .map_err(|e| ModuleError::Internal(e.to_string()))
            })
            .unwrap();
        assert!(!raw
            .windows(b"secreto".len())
            .any(|window| window == b"secreto"));

        // Audit emitido (sin PHI).
        assert!(ctx
            .events
            .lock()
            .unwrap()
            .iter()
            .any(|event| matches!(event, AuditEvent::SolicitudIssued { .. })));

        // List devuelve solo metadatos y copia el nombre del template.
        let list = run(&db, &ctx, "solicitud.list", json!({}), Some(1)).unwrap();
        assert_eq!(list[0]["study_name"].as_str(), Some("Rx tórax"));
        assert!(list[0].get("values").is_none());

        // Get descifra.
        let got = run(&db, &ctx, "solicitud.get", json!({"id": id}), None).unwrap();
        assert_eq!(got["values"]["indicacion"].as_str(), Some("secreto clinico"));
    }

    #[test]
    fn solicitud_get_and_render_include_patient_professional_and_labels() {
        let db = setup_db();
        let ctx = FakeCtx::new("user-a");
        let template_id = create_template(&db, &ctx, "Rx tórax");
        let created = run(
            &db,
            &ctx,
            "solicitud.create",
            json!({
                "template_id": template_id,
                "values": {
                    "estudio": "frente y perfil",
                    "indicacion": "control evolutivo",
                    "prioridad": "Rutina"
                }
            }),
            Some(1),
        )
        .unwrap();
        let id = created["id"].as_str().unwrap();

        let rendered = run(&db, &ctx, "solicitud.render", json!({"id": id}), None).unwrap();
        let markdown = rendered["markdown"].as_str().unwrap();
        assert!(markdown.contains("# Solicitud de estudio: Rx tórax"));
        assert!(markdown.contains("**Paciente:** Juan Pérez"));
        assert!(markdown.contains("**Profesional:** Dra. Ana (Médica)"));
        assert!(markdown.contains("**Estudio:** frente y perfil"));
        assert!(markdown.contains("**Indicación:** control evolutivo"));
        assert!(markdown.contains("**Prioridad:** Rutina"));

        // Regla de oro: si el profesional edita el template después y quita
        // el campo `indicacion`, la solicitud ya emitida sigue mostrando su
        // valor sin perder datos.
        run(
            &db,
            &ctx,
            "template.update",
            json!({
                "id": template_id,
                "name": "Rx tórax v2",
                "fields": [
                    {"name": "estudio", "label": "Estudio", "type": "text", "required": true}
                ]
            }),
            None,
        )
        .unwrap();

        let rendered_after =
            run(&db, &ctx, "solicitud.render", json!({"id": id}), None).unwrap();
        let markdown_after = rendered_after["markdown"].as_str().unwrap();
        assert!(markdown_after.contains("**Estudio:** frente y perfil"));
        assert!(markdown_after.contains("control evolutivo"));
    }

    #[test]
    fn solicitud_requires_entity_and_required_values() {
        let db = setup_db();
        let ctx = FakeCtx::new("user-a");
        let template_id = create_template(&db, &ctx, "Rx tórax");

        // Sin entity_id.
        assert!(matches!(
            run(
                &db,
                &ctx,
                "solicitud.create",
                json!({"study_name": "X", "values": {}}),
                None
            ),
            Err(ModuleError::InvalidRequest(_))
        ));

        // Entity inexistente.
        let mut no_entity = FakeCtx::new("user-a");
        no_entity.entity_exists = false;
        assert!(matches!(
            run(
                &db,
                &no_entity,
                "solicitud.create",
                json!({"study_name": "X", "values": {}}),
                Some(1)
            ),
            Err(ModuleError::InvalidRequest(_))
        ));

        // Falta un required del template.
        assert!(matches!(
            run(
                &db,
                &ctx,
                "solicitud.create",
                json!({"template_id": template_id, "values": {"estudio": "X"}}),
                Some(1)
            ),
            Err(ModuleError::InvalidRequest(_))
        ));
    }

    #[test]
    fn manual_solicitud_without_template_works_and_humanizes_keys() {
        let db = setup_db();
        let ctx = FakeCtx::new("user-a");

        let created = run(
            &db,
            &ctx,
            "solicitud.create",
            json!({"study_name": "Cultivo", "values": {"material": "orina"}}),
            Some(1),
        )
        .unwrap();
        let id = created["id"].as_str().unwrap();

        let rendered = run(&db, &ctx, "solicitud.render", json!({"id": id}), None).unwrap();
        assert!(rendered["markdown"]
            .as_str()
            .unwrap()
            .contains("**Material:** orina"));
    }

    #[test]
    fn vault_locked_during_wrap_propagates_core_error() {
        let db = setup_db();
        let mut ctx = FakeCtx::new("user-a");
        ctx.wrap_error = Some(CommandError::new(ERR_VAULT_LOCKED));

        let err = run(
            &db,
            &ctx,
            "solicitud.create",
            json!({"study_name": "X", "values": {}}),
            Some(1),
        )
        .unwrap_err();
        let command_error: CommandError = err.into();
        assert_eq!(command_error.code, ERR_VAULT_LOCKED);
    }

    #[test]
    fn unknown_action_is_unsupported() {
        let db = setup_db();
        let ctx = FakeCtx::new("user-a");
        assert_eq!(
            run(&db, &ctx, "nope", json!({}), None).unwrap_err(),
            ModuleError::Unsupported
        );
    }
}
