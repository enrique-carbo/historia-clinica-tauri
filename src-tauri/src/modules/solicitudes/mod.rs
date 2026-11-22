//! Módulo de solicitud de estudios complementarios (`docs/MODULES/module-solicitudes-v1.md`).
//!
//! Ruta 1: módulo interno compilado (feature `module-solicitudes`). Datos
//! operativos en `fichio-solicitudes.db`; filiación/profesional solo por
//! `ModuleContext`; el resultado clínico sigue en la SOAP del Core.
//!
//! Modelo v2: template = título + lista de estudios (checklist); solicitud =
//! ítems tildados + diagnóstico + notas (PHI cifrada con la DataKey).

mod models;
mod render;
mod store;

use crate::core::dto::{ActorRole, AuditEvent};
use crate::core::module::{
    Module, ModuleContext, ModuleDb, ModuleDescriptor, ModuleError, ModuleRequest,
    ModuleResponse, ModuleScope,
};
use async_trait::async_trait;
use models::{ensure_items_belong, normalize_optional_text};
use render::{render_solicitud, render_solicitud_html};
use rusqlite::Connection;
use serde_json::{json, Value};
use uuid::Uuid;

pub struct SolicitudesModule;

/// Schema propio del módulo (`PRAGMA user_version` en `fichio-solicitudes.db`).
/// v1 fue el modelo pre-release de campos tipados; v2 es checklist + diagnóstico.
const SCHEMA_VERSION: i64 = 2;

const SCHEMA_V2: &str = r#"
    CREATE TABLE IF NOT EXISTS templates (
        id            TEXT PRIMARY KEY,
        owner_user_id TEXT NOT NULL,
        name          TEXT NOT NULL,
        items_json    TEXT NOT NULL,
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
        items_enc          BLOB NOT NULL,
        diagnostico_enc    BLOB,
        notas_enc          BLOB,
        created_at         TEXT NOT NULL,
        status             TEXT NOT NULL DEFAULT 'issued'
            CHECK (status IN ('draft', 'issued'))
    );
    CREATE INDEX IF NOT EXISTS idx_solicitudes_owner
        ON solicitudes(created_by_user_id);
    CREATE INDEX IF NOT EXISTS idx_solicitudes_entity
        ON solicitudes(entity_id);
"#;

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Reset explícito de las versiones pre-release (v0/v1): esquema de campos
/// tipados, incompatible con el checklist. Es el **único** caso destructivo y
/// está acotado por versión; se elimina al estabilizar la v2.
fn reset_pre_release_v0_v1(conn: &Connection) -> Result<(), ModuleError> {
    conn.execute_batch("DROP TABLE IF EXISTS solicitudes; DROP TABLE IF EXISTS templates;")
        .map_err(|e| ModuleError::Internal(e.to_string()))
}

fn apply_schema_v2(conn: &Connection) -> Result<(), ModuleError> {
    conn.execute_batch(SCHEMA_V2)
        .map_err(|e| ModuleError::Internal(e.to_string()))
}

fn require_str<'a>(payload: &'a Value, key: &str) -> Result<&'a str, ModuleError> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ModuleError::InvalidRequest(format!("falta '{}'", key)))
}

fn parse_items(payload: &Value) -> Result<Vec<String>, ModuleError> {
    let raw = payload
        .get("items")
        .ok_or_else(|| ModuleError::InvalidRequest("falta 'items'".to_string()))?;
    let items: Vec<String> = serde_json::from_value(raw.clone()).map_err(|_| {
        ModuleError::InvalidRequest("'items' debe ser una lista de textos".to_string())
    })?;
    models::normalize_items(items)
}

fn to_payload<T: serde::Serialize>(value: T) -> Result<Value, ModuleError> {
    serde_json::to_value(value).map_err(|e| ModuleError::Internal(e.to_string()))
}

async fn decrypt_optional_text(
    ctx: &dyn ModuleContext,
    encoded: Option<&[u8]>,
) -> Result<Option<String>, ModuleError> {
    match encoded {
        None => Ok(None),
        Some(bytes) => {
            let plaintext = ctx.unwrap_with_datakey(bytes).await?;
            String::from_utf8(plaintext.to_vec())
                .map(Some)
                .map_err(|e| ModuleError::Internal(e.to_string()))
        }
    }
}

/// Codificación canónica del contenido auditado: longitud-prefijo por campo,
/// sin depender de JSON ni del orden de claves. La usa el fingerprint HMAC
/// (`ctx.fingerprint_with_datakey`) para que el mismo contenido produzca
/// siempre los mismos bytes.
fn canonical_solicitud(
    solicitud_id: &str,
    entity_id: i64,
    study_name: &str,
    items: &[String],
    diagnostico: Option<&str>,
    notas: Option<&str>,
) -> Vec<u8> {
    fn part(out: &mut Vec<u8>, bytes: &[u8]) {
        out.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
        out.extend_from_slice(bytes);
    }

    let mut out = Vec::new();
    part(&mut out, solicitud_id.as_bytes());
    part(&mut out, &entity_id.to_be_bytes());
    part(&mut out, study_name.as_bytes());
    part(&mut out, &(items.len() as u64).to_be_bytes());
    for item in items {
        part(&mut out, item.as_bytes());
    }
    for optional in [diagnostico, notas] {
        match optional {
            Some(text) => {
                out.push(1);
                part(&mut out, text.as_bytes());
            }
            None => out.push(0),
        }
    }
    out
}

async fn template_create(
    req: &ModuleRequest,
    db: &ModuleDb,
    ctx: &dyn ModuleContext,
) -> Result<ModuleResponse, ModuleError> {
    let name = require_str(&req.payload, "name")?;
    let items = parse_items(&req.payload)?;

    let id = Uuid::new_v4().to_string();
    store::insert_template(db, &id, &ctx.actor().user_id, name, &items, &now())?;
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
    let items = parse_items(&req.payload)?;

    let updated = store::update_template(
        db,
        id,
        &ctx.actor().user_id,
        name,
        &items,
        &now(),
    )?;
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

    let mut items = parse_items(&req.payload)?;
    let diagnostico = normalize_optional_text(
        req.payload.get("diagnostico").and_then(Value::as_str),
    );
    let notas = normalize_optional_text(req.payload.get("notas").and_then(Value::as_str));

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
        ensure_items_belong(&template.items, &items)?;
        // Preservar el orden canónico del template, sin importar en qué
        // orden el profesional tildó los checkboxes.
        items = template
            .items
            .iter()
            .filter(|candidate| items.contains(candidate))
            .cloned()
            .collect();
    }

    if study_name.is_empty() {
        return Err(ModuleError::InvalidRequest(
            "falta 'study_name' o un template".to_string(),
        ));
    }

    let items_json = serde_json::to_vec(&items)
        .map_err(|e| ModuleError::Internal(e.to_string()))?;
    let items_enc = ctx.wrap_with_datakey(&items_json).await?;
    let diagnostico_enc = match &diagnostico {
        Some(text) => Some(ctx.wrap_with_datakey(text.as_bytes()).await?),
        None => None,
    };
    let notas_enc = match &notas {
        Some(text) => Some(ctx.wrap_with_datakey(text.as_bytes()).await?),
        None => None,
    };

    let id = Uuid::new_v4().to_string();
    let fingerprint = ctx
        .fingerprint_with_datakey(
            "solicitud.issued/v1",
            &canonical_solicitud(
                &id,
                entity_id,
                &study_name,
                &items,
                diagnostico.as_deref(),
                notas.as_deref(),
            ),
        )
        .await?;

    let created_at = now();
    store::insert_solicitud(
        db,
        store::NewSolicitud {
            id: &id,
            entity_id,
            owner: &ctx.actor().user_id,
            study_name: &study_name,
            template_id: template_id.as_deref(),
            items_enc: &items_enc,
            diagnostico_enc: diagnostico_enc.as_deref(),
            notas_enc: notas_enc.as_deref(),
            now: &created_at,
        },
    )?;

    ctx.audit(AuditEvent::SolicitudIssued {
        subject_id: entity_id,
        solicitud_id: id.clone(),
        fingerprint,
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

async fn solicitud_get(
    req: &ModuleRequest,
    db: &ModuleDb,
    ctx: &dyn ModuleContext,
) -> Result<ModuleResponse, ModuleError> {
    let id = require_str(&req.payload, "id")?;
    let record = store::load_solicitud(db, id, &ctx.actor().user_id)?.ok_or_else(|| {
        ModuleError::InvalidRequest("solicitud no encontrada".to_string())
    })?;

    let items_plain = ctx.unwrap_with_datakey(&record.items_enc).await?;
    let items: Vec<String> =
        serde_json::from_slice(&items_plain).map_err(|e| ModuleError::Internal(e.to_string()))?;
    let diagnostico = decrypt_optional_text(ctx, record.diagnostico_enc.as_deref()).await?;
    let notas = decrypt_optional_text(ctx, record.notas_enc.as_deref()).await?;

    Ok(ModuleResponse {
        payload: json!({
            "solicitud": record.summary,
            "items": items,
            "diagnostico": diagnostico,
            "notas": notas,
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

    let items_plain = ctx.unwrap_with_datakey(&record.items_enc).await?;
    let items: Vec<String> =
        serde_json::from_slice(&items_plain).map_err(|e| ModuleError::Internal(e.to_string()))?;
    let diagnostico = decrypt_optional_text(ctx, record.diagnostico_enc.as_deref()).await?;
    let notas = decrypt_optional_text(ctx, record.notas_enc.as_deref()).await?;

    let filiation = ctx.fetch_filiation(record.summary.entity_id).await?;
    let professional = ctx.fetch_professional(&ctx.actor().user_id).await?;

    let markdown = render_solicitud(
        &filiation,
        &professional,
        &record.summary.study_name,
        &items,
        diagnostico.as_deref(),
        notas.as_deref(),
        &record.summary.created_at,
    );
    let html = render_solicitud_html(
        &filiation,
        &professional,
        &record.summary.study_name,
        &items,
        diagnostico.as_deref(),
        notas.as_deref(),
        &record.summary.created_at,
    );

    Ok(ModuleResponse {
        payload: json!({ "markdown": markdown, "html": html }),
    })
}

#[async_trait]
impl Module for SolicitudesModule {
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            id: "solicitudes".to_string(),
            name: "Solicitud de estudios complementarios".to_string(),
            version: "0.2.0".to_string(),
            scope: ModuleScope::Installation,
            allowed_roles: vec![ActorRole::Profesional],
        }
    }

    fn migrate(&self, conn: &Connection) -> Result<(), ModuleError> {
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|e| ModuleError::Internal(e.to_string()))?;

        if version > SCHEMA_VERSION {
            return Err(ModuleError::SchemaTooNew);
        }
        if version == SCHEMA_VERSION {
            // Fast path: nunca se corre DDL sobre un schema al día (y menos
            // un DROP). Si la DB está corrupta, que falle al usarla.
            return Ok(());
        }

        // Solo v0/v1 son incompatibles: el modelo pre-release de campos
        // tipados no se puede convertir al checklist. El reset está acotado
        // a esas versiones exactas; al agregar v3, los datos v2 se migran de
        // forma aditiva (ALTER/backfill), nunca con un DROP.
        if version < 2 {
            reset_pre_release_v0_v1(conn)?;
            apply_schema_v2(conn)?;
        }

        // Futuro:
        // if version < 3 { apply_schema_v3(conn)?; }

        conn.execute_batch(&format!("PRAGMA user_version = {}", SCHEMA_VERSION))
            .map_err(|e| ModuleError::Internal(e.to_string()))?;
        Ok(())
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

        async fn fingerprint_with_datakey(
            &self,
            domain: &str,
            data: &[u8],
        ) -> Result<String, CommandError> {
            Ok(crypto::audit_fingerprint(&KEY, domain, data))
        }
    }

    fn setup_db() -> ModuleDb {
        let conn = Connection::open_in_memory().expect("in-memory");
        SolicitudesModule.migrate(&conn).expect("migrate");
        ModuleDb::new(conn)
    }

    fn items() -> Value {
        json!(["Hemograma completo", "Glucemia", "Colesterol total"])
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
        let payload = json!({ "name": name, "items": items() });
        let response = run(db, ctx, "template.create", payload, None).expect("template.create");
        response["id"].as_str().unwrap().to_string()
    }

    fn raw_blob(db: &ModuleDb, column: &str, id: &str) -> Option<Vec<u8>> {
        db.with_conn(|conn| {
            conn.query_row(
                &format!("SELECT {} FROM solicitudes WHERE id = ?1", column),
                params![id],
                |row| row.get::<_, Option<Vec<u8>>>(0),
            )
            .map_err(|e| ModuleError::Internal(e.to_string()))
        })
        .expect("raw blob")
    }

    #[test]
    fn template_crud_is_owner_scoped_and_stores_items() {
        let db = setup_db();
        let owner = FakeCtx::new("user-a");
        let other = FakeCtx::new("user-b");
        let id = create_template(&db, &owner, "Laboratorio general");

        let list = run(&db, &owner, "template.list", json!({}), None).unwrap();
        assert_eq!(list[0]["name"].as_str(), Some("Laboratorio general"));
        assert_eq!(list[0]["items"][0].as_str(), Some("Hemograma completo"));
        assert!(run(&db, &other, "template.list", json!({}), None)
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty());

        let update = json!({
            "id": id,
            "name": "Laboratorio general v2",
            "items": ["Hemograma completo", "TSH"]
        });
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
    fn template_requires_name_and_at_least_one_item() {
        let db = setup_db();
        let ctx = FakeCtx::new("user-a");

        let empty_items = json!({"name": "X", "items": []});
        assert!(matches!(
            run(&db, &ctx, "template.create", empty_items, None),
            Err(ModuleError::InvalidRequest(_))
        ));

        let not_array = json!({"name": "X", "items": "Hemograma"});
        assert!(matches!(
            run(&db, &ctx, "template.create", not_array, None),
            Err(ModuleError::InvalidRequest(_))
        ));
    }

    #[test]
    fn solicitud_create_encrypts_items_diagnostico_and_audits() {
        let db = setup_db();
        let ctx = FakeCtx::new("user-a");
        let template_id = create_template(&db, &ctx, "Laboratorio general");

        let created = run(
            &db,
            &ctx,
            "solicitud.create",
            json!({
                "template_id": template_id,
                // Tildados en orden inverso al del template: debe guardarse en
                // el orden canónico del template ("Hemograma completo", "Glucemia").
                "items": ["Glucemia", "Hemograma completo"],
                "diagnostico": "control anemia",
                "notas": "paciente en ayunas"
            }),
            Some(1),
        )
        .expect("solicitud.create");
        let id = created["id"].as_str().unwrap().to_string();

        // Los blobs no contienen el texto plano (PHI cifrada).
        for (column, needle) in [
            ("items_enc", b"Hemograma".as_slice()),
            ("diagnostico_enc", b"control anemia".as_slice()),
            ("notas_enc", b"paciente en ayunas".as_slice()),
        ] {
            let blob = raw_blob(&db, column, &id).expect("blob presente");
            assert!(
                !blob.windows(needle.len()).any(|window| window == needle),
                "{} no debe estar en claro",
                column
            );
        }

        // Audit emitido (sin PHI): fingerprint HMAC, no un SHA-256 adivinable.
        let events = ctx.events.lock().unwrap();
        let fingerprint = events
            .iter()
            .find_map(|event| match event {
                AuditEvent::SolicitudIssued { fingerprint, .. } => Some(fingerprint.clone()),
                _ => None,
            })
            .expect("solicitud_issued");
        assert!(fingerprint.starts_with("hmac-sha256:"));
        assert!(!fingerprint.contains("Hemograma"));
        assert!(!fingerprint.contains("control anemia"));

        // List devuelve solo metadatos y copia el nombre del template.
        let list = run(&db, &ctx, "solicitud.list", json!({}), Some(1)).unwrap();
        assert_eq!(list[0]["study_name"].as_str(), Some("Laboratorio general"));
        assert!(list[0].get("items").is_none());

        // Get descifra ítems (en el orden del template) + diagnóstico + notas.
        let got = run(&db, &ctx, "solicitud.get", json!({"id": id}), None).unwrap();
        assert_eq!(got["items"][0].as_str(), Some("Hemograma completo"));
        assert_eq!(got["items"][1].as_str(), Some("Glucemia"));
        assert_eq!(got["diagnostico"].as_str(), Some("control anemia"));
        assert_eq!(got["notas"].as_str(), Some("paciente en ayunas"));
    }

    #[test]
    fn solicitud_render_includes_checklist_diagnostico_and_notas() {
        let db = setup_db();
        let ctx = FakeCtx::new("user-a");
        create_template(&db, &ctx, "Laboratorio general");
        let created = run(
            &db,
            &ctx,
            "solicitud.create",
            json!({
                "study_name": "Laboratorio general",
                "items": ["Hemograma completo", "Glucemia"],
                "diagnostico": "control anemia",
                "notas": "ayuno"
            }),
            Some(1),
        )
        .unwrap();
        let id = created["id"].as_str().unwrap();

        let rendered = run(&db, &ctx, "solicitud.render", json!({"id": id}), None).unwrap();
        let markdown = rendered["markdown"].as_str().unwrap();
        assert!(markdown.contains("# Solicitud de estudio: Laboratorio general"));
        assert!(markdown.contains("**Paciente:** Juan Pérez"));
        assert!(markdown.contains("**Profesional:** Dra. Ana (Médica)"));
        assert!(markdown.contains("## Diagnóstico / indicación"));
        assert!(markdown.contains("control anemia"));
        assert!(markdown.contains("- Hemograma completo"));
        assert!(markdown.contains("- Glucemia"));
        assert!(markdown.contains("## Notas"));
        assert!(markdown.contains("ayuno"));
        assert!(rendered["html"]
            .as_str()
            .unwrap()
            .contains("<!doctype html>"));
    }

    #[test]
    fn solicitud_requires_entity_items_and_template_membership() {
        let db = setup_db();
        let ctx = FakeCtx::new("user-a");
        let template_id = create_template(&db, &ctx, "Laboratorio general");

        // Sin entity_id.
        assert!(matches!(
            run(
                &db,
                &ctx,
                "solicitud.create",
                json!({"study_name": "X", "items": ["Hemograma completo"]}),
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
                json!({"study_name": "X", "items": ["Hemograma completo"]}),
                Some(1)
            ),
            Err(ModuleError::InvalidRequest(_))
        ));

        // Sin ítems.
        assert!(matches!(
            run(
                &db,
                &ctx,
                "solicitud.create",
                json!({"template_id": template_id, "items": []}),
                Some(1)
            ),
            Err(ModuleError::InvalidRequest(_))
        ));

        // Ítem que no pertenece al template.
        assert!(matches!(
            run(
                &db,
                &ctx,
                "solicitud.create",
                json!({"template_id": template_id, "items": ["Orina completa"]}),
                Some(1)
            ),
            Err(ModuleError::InvalidRequest(_))
        ));
    }

    #[test]
    fn manual_solicitud_without_template_accepts_free_items() {
        let db = setup_db();
        let ctx = FakeCtx::new("user-a");

        let created = run(
            &db,
            &ctx,
            "solicitud.create",
            json!({
                "study_name": "Cultivo",
                "items": ["Material: orina", "Siembra directa"],
                "notas": "muestra de la mañana"
            }),
            Some(1),
        )
        .unwrap();
        let id = created["id"].as_str().unwrap();

        let got = run(&db, &ctx, "solicitud.get", json!({"id": id}), None).unwrap();
        assert_eq!(got["items"][0].as_str(), Some("Material: orina"));
        assert_eq!(got["notas"].as_str(), Some("muestra de la mañana"));
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
            json!({"study_name": "X", "items": ["Hemograma completo"]}),
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

    #[test]
    fn canonical_solicitud_is_unambiguous() {
        let items_ab_c = vec!["ab".to_string(), "c".to_string()];
        let items_a_bc = vec!["a".to_string(), "bc".to_string()];
        assert_ne!(
            canonical_solicitud("id", 1, "t", &items_ab_c, None, None),
            canonical_solicitud("id", 1, "t", &items_a_bc, None, None),
            "el largo-prefijo evita colisiones entre listas"
        );

        // Diagnóstico y notas ocupan posiciones distintas: no son intercambiables.
        assert_ne!(
            canonical_solicitud("id", 1, "t", &items_ab_c, Some("dx"), None),
            canonical_solicitud("id", 1, "t", &items_ab_c, None, Some("dx")),
        );
    }

    #[test]
    fn migrate_resets_legacy_v0_schema_and_sets_version() {
        let conn = Connection::open_in_memory().expect("in-memory");
        conn.execute_batch(
            "CREATE TABLE templates (id TEXT); CREATE TABLE solicitudes (id TEXT);",
        )
        .unwrap();
        // user_version = 0 (modelo pre-release viejo)
        SolicitudesModule.migrate(&conn).expect("migrate v2");

        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        let columns: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('templates') WHERE name = 'items_json'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(columns, 1);
    }

    #[test]
    fn migrate_v1_resets_only_the_incompatible_pre_release_schema() {
        let conn = Connection::open_in_memory().expect("in-memory");
        conn.execute_batch(
            "CREATE TABLE templates (id TEXT PRIMARY KEY, fields_json TEXT);
             CREATE TABLE solicitudes (id TEXT PRIMARY KEY, values_enc BLOB);
             INSERT INTO templates (id, fields_json) VALUES ('old', '[]');
             PRAGMA user_version = 1;",
        )
        .unwrap();

        SolicitudesModule.migrate(&conn).expect("migrate v1 -> v2");

        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM templates", [], |row| row.get(0))
            .unwrap();
        assert_eq!(rows, 0, "v1 no es convertible: se resetea");
        let has_items: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('templates') WHERE name = 'items_json'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(has_items, 1);
        let has_fields: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('templates') WHERE name = 'fields_json'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(has_fields, 0);
    }

    #[test]
    fn migrate_current_version_is_noop_and_preserves_data() {
        let conn = Connection::open_in_memory().expect("in-memory");
        SolicitudesModule.migrate(&conn).expect("migrate inicial");
        conn.execute(
            "INSERT INTO templates
                (id, owner_user_id, name, items_json, created_at, updated_at)
             VALUES ('t1', 'u1', 'Laboratorio general', '[\"Hemograma\"]', 'now', 'now')",
            [],
        )
        .unwrap();

        // Correr migrate de nuevo es idempotente: no hay DDL ni pérdida de filas.
        SolicitudesModule.migrate(&conn).expect("migrate idempotente");

        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM templates", [], |row| row.get(0))
            .unwrap();
        assert_eq!(rows, 1);
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
    }

    #[test]
    fn migrate_rejects_a_newer_schema_version() {
        let conn = Connection::open_in_memory().expect("in-memory");
        conn.execute_batch(&format!("PRAGMA user_version = {}", SCHEMA_VERSION + 1))
            .unwrap();
        assert_eq!(
            SolicitudesModule.migrate(&conn).unwrap_err(),
            ModuleError::SchemaTooNew
        );
    }
}
