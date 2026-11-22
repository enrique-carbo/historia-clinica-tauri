//! Tipos y validación del módulo de solicitudes.
//!
//! Un template es **título + lista ordenada de estudios** (checklist). La
//! solicitud guarda los ítems tildados + diagnóstico + notas; no hay campos
//! tipados ni formularios por estudio.

use crate::core::module::ModuleError;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TemplateRecord {
    pub id: String,
    pub name: String,
    /// Estudios del catálogo, en el orden en que se muestran/imprimen.
    pub items: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Metadatos de una solicitud: nunca incluye contenido clínico descifrado.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SolicitudSummary {
    pub id: String,
    pub entity_id: i64,
    pub study_name: String,
    pub template_id: Option<String>,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct SolicitudRecord {
    pub summary: SolicitudSummary,
    /// `nonce ‖ ciphertext` (DataKey), JSON `["Hemograma completo", ...]`.
    pub items_enc: Vec<u8>,
    /// Texto cifrado (PHI), opcional.
    pub diagnostico_enc: Option<Vec<u8>>,
    /// Texto cifrado (PHI), opcional.
    pub notas_enc: Option<Vec<u8>>,
}

/// Normaliza la lista de estudios: recorta, descarta vacíos, deduplica
/// preservando el orden y exige al menos uno.
pub fn normalize_items(items: Vec<String>) -> Result<Vec<String>, ModuleError> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for item in items {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        if seen.insert(trimmed.to_string()) {
            out.push(trimmed.to_string());
        }
    }
    if out.is_empty() {
        return Err(ModuleError::InvalidRequest(
            "seleccioná al menos un estudio".to_string(),
        ));
    }
    Ok(out)
}

/// Si la solicitud viene de un template, todo ítem tildado debe existir en
/// ese template (no se aceptan estudios inventados).
pub fn ensure_items_belong(
    template_items: &[String],
    selected: &[String],
) -> Result<(), ModuleError> {
    let known: HashSet<&str> = template_items.iter().map(String::as_str).collect();
    for item in selected {
        if !known.contains(item.as_str()) {
            return Err(ModuleError::InvalidRequest(format!(
                "el estudio '{}' no pertenece al template",
                item
            )));
        }
    }
    Ok(())
}

/// Texto libre opcional (diagnóstico/notas): trim; vacío → `None`.
pub fn normalize_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_items_trims_dedupes_and_requires_one() {
        let items = normalize_items(vec![
            " Hemograma completo ".to_string(),
            "".to_string(),
            "Glucemia".to_string(),
            "Hemograma completo".to_string(),
        ])
        .expect("items válidos");
        assert_eq!(items, vec!["Hemograma completo", "Glucemia"]);

        assert!(normalize_items(vec!["  ".to_string()]).is_err());
    }

    #[test]
    fn ensure_items_belong_rejects_unknown_items() {
        let template = vec!["Hemograma completo".to_string(), "Glucemia".to_string()];
        ensure_items_belong(&template, &["Glucemia".to_string()]).expect("pertenece");
        assert!(ensure_items_belong(&template, &["Orina completa".to_string()]).is_err());
    }

    #[test]
    fn optional_text_trims_and_drops_empty() {
        assert_eq!(normalize_optional_text(Some("  control  ")), Some("control".to_string()));
        assert_eq!(normalize_optional_text(Some("   ")), None);
        assert_eq!(normalize_optional_text(None), None);
    }
}
