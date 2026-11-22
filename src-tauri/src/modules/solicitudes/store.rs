//! Persistencia del módulo de solicitudes (`fichio-solicitudes.db`).
//!
//! Solo conoce las tablas del módulo: `templates` (catálogo sin PHI) y
//! `solicitudes` (metadatos + ítems/diagnóstico/notas cifrados).

use super::models::{SolicitudRecord, SolicitudSummary, TemplateRecord};
use crate::core::module::{ModuleDb, ModuleError};
use rusqlite::{params, OptionalExtension};

fn internal(error: rusqlite::Error) -> ModuleError {
    ModuleError::Internal(error.to_string())
}

fn parse_items(json: &str) -> Result<Vec<String>, ModuleError> {
    serde_json::from_str(json).map_err(|e| ModuleError::Internal(e.to_string()))
}

pub fn insert_template(
    db: &ModuleDb,
    id: &str,
    owner: &str,
    name: &str,
    items: &[String],
    now: &str,
) -> Result<(), ModuleError> {
    let items_json =
        serde_json::to_string(items).map_err(|e| ModuleError::Internal(e.to_string()))?;
    db.with_tx(|tx| {
        tx.execute(
            "INSERT INTO templates
                (id, owner_user_id, name, items_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![id, owner, name, items_json, now],
        )
        .map_err(internal)?;
        Ok(())
    })
}

pub fn update_template(
    db: &ModuleDb,
    id: &str,
    owner: &str,
    name: &str,
    items: &[String],
    now: &str,
) -> Result<bool, ModuleError> {
    let items_json =
        serde_json::to_string(items).map_err(|e| ModuleError::Internal(e.to_string()))?;
    db.with_tx(|tx| {
        let changed = tx
            .execute(
                "UPDATE templates
                 SET name = ?1, items_json = ?2, updated_at = ?3
                 WHERE id = ?4 AND owner_user_id = ?5",
                params![name, items_json, now, id, owner],
            )
            .map_err(internal)?;
        Ok(changed > 0)
    })
}

pub fn delete_template(
    db: &ModuleDb,
    id: &str,
    owner: &str,
) -> Result<bool, ModuleError> {
    db.with_tx(|tx| {
        let changed = tx
            .execute(
                "DELETE FROM templates WHERE id = ?1 AND owner_user_id = ?2",
                params![id, owner],
            )
            .map_err(internal)?;
        Ok(changed > 0)
    })
}

pub fn list_templates(
    db: &ModuleDb,
    owner: &str,
) -> Result<Vec<TemplateRecord>, ModuleError> {
    db.with_conn(|conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, name, items_json, created_at, updated_at
                 FROM templates
                 WHERE owner_user_id = ?1
                 ORDER BY updated_at DESC",
            )
            .map_err(internal)?;

        let rows = stmt
            .query_map(params![owner], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(internal)?;

        let mut out = Vec::new();
        for row in rows {
            let (id, name, items_json, created_at, updated_at) = row.map_err(internal)?;
            out.push(TemplateRecord {
                id,
                name,
                items: parse_items(&items_json)?,
                created_at,
                updated_at,
            });
        }
        Ok(out)
    })
}

pub fn load_template(
    db: &ModuleDb,
    id: &str,
    owner: &str,
) -> Result<Option<TemplateRecord>, ModuleError> {
    let row = db.with_conn(|conn| {
        conn.query_row(
            "SELECT id, name, items_json, created_at, updated_at
             FROM templates
             WHERE id = ?1 AND owner_user_id = ?2",
            params![id, owner],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional()
        .map_err(internal)
    })?;

    match row {
        None => Ok(None),
        Some((id, name, items_json, created_at, updated_at)) => Ok(Some(TemplateRecord {
            id,
            name,
            items: parse_items(&items_json)?,
            created_at,
            updated_at,
        })),
    }
}

pub struct NewSolicitud<'a> {
    pub id: &'a str,
    pub entity_id: i64,
    pub owner: &'a str,
    pub study_name: &'a str,
    pub template_id: Option<&'a str>,
    pub items_enc: &'a [u8],
    pub diagnostico_enc: Option<&'a [u8]>,
    pub notas_enc: Option<&'a [u8]>,
    pub now: &'a str,
}

pub fn insert_solicitud(
    db: &ModuleDb,
    new: NewSolicitud<'_>,
) -> Result<(), ModuleError> {
    db.with_tx(|tx| {
        tx.execute(
            "INSERT INTO solicitudes
                (id, entity_id, created_by_user_id, study_name, template_id,
                 items_enc, diagnostico_enc, notas_enc, created_at, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'issued')",
            params![
                new.id,
                new.entity_id,
                new.owner,
                new.study_name,
                new.template_id,
                new.items_enc,
                new.diagnostico_enc,
                new.notas_enc,
                new.now
            ],
        )
        .map_err(internal)?;
        Ok(())
    })
}

pub fn list_solicitudes(
    db: &ModuleDb,
    owner: &str,
    entity_id: Option<i64>,
) -> Result<Vec<SolicitudSummary>, ModuleError> {
    db.with_conn(|conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, entity_id, study_name, template_id, status, created_at
                 FROM solicitudes
                 WHERE created_by_user_id = ?1
                   AND (?2 IS NULL OR entity_id = ?2)
                 ORDER BY created_at DESC",
            )
            .map_err(internal)?;

        let rows = stmt
            .query_map(params![owner, entity_id], |row| {
                Ok(SolicitudSummary {
                    id: row.get(0)?,
                    entity_id: row.get(1)?,
                    study_name: row.get(2)?,
                    template_id: row.get(3)?,
                    status: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .map_err(internal)?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(internal)?);
        }
        Ok(out)
    })
}

pub fn load_solicitud(
    db: &ModuleDb,
    id: &str,
    owner: &str,
) -> Result<Option<SolicitudRecord>, ModuleError> {
    db.with_conn(|conn| {
        conn.query_row(
            "SELECT id, entity_id, study_name, template_id, status, created_at,
                    items_enc, diagnostico_enc, notas_enc
             FROM solicitudes
             WHERE id = ?1 AND created_by_user_id = ?2",
            params![id, owner],
            |row| {
                Ok(SolicitudRecord {
                    summary: SolicitudSummary {
                        id: row.get(0)?,
                        entity_id: row.get(1)?,
                        study_name: row.get(2)?,
                        template_id: row.get(3)?,
                        status: row.get(4)?,
                        created_at: row.get(5)?,
                    },
                    items_enc: row.get(6)?,
                    diagnostico_enc: row.get(7)?,
                    notas_enc: row.get(8)?,
                })
            },
        )
        .optional()
        .map_err(internal)
    })
}
