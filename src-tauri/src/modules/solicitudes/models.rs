//! Tipos y validación del módulo de solicitudes.
//!
//! `TemplateDefinition` es la misma *shape* que los entry templates del Core
//! (`type`: text/textarea/select/date), pero vive en la DB del módulo porque
//! la escribe cada profesional. No lleva datos de paciente.

use crate::core::module::ModuleError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TemplateField {
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct TemplateDefinition {
    pub fields: Vec<TemplateField>,
}

impl TemplateDefinition {
    /// Valida la forma del template (no el contenido de una solicitud).
    pub fn validate(&self) -> Result<(), ModuleError> {
        if self.fields.is_empty() {
            return Err(ModuleError::InvalidRequest(
                "el template no tiene campos".to_string(),
            ));
        }

        let mut seen = HashSet::new();
        for field in &self.fields {
            if field.name.trim().is_empty() {
                return Err(ModuleError::InvalidRequest(
                    "un campo del template no tiene 'name'".to_string(),
                ));
            }
            if field.label.trim().is_empty() {
                return Err(ModuleError::InvalidRequest(format!(
                    "el campo '{}' no tiene label",
                    field.name
                )));
            }
            if !matches!(
                field.field_type.as_str(),
                "text" | "textarea" | "select" | "date"
            ) {
                return Err(ModuleError::InvalidRequest(format!(
                    "tipo inválido en '{}': {}",
                    field.name, field.field_type
                )));
            }
            if field.field_type == "select" && field.options.is_empty() {
                return Err(ModuleError::InvalidRequest(format!(
                    "el select '{}' no tiene options",
                    field.name
                )));
            }
            if !seen.insert(field.name.as_str()) {
                return Err(ModuleError::InvalidRequest(format!(
                    "campo duplicado en el template: '{}'",
                    field.name
                )));
            }
        }
        Ok(())
    }

    /// Valida los valores de una solicitud contra este template: rechaza
    /// campos desconocidos y required faltantes.
    pub fn validate_values(
        &self,
        values: &BTreeMap<String, String>,
    ) -> Result<(), ModuleError> {
        let known: HashSet<&str> = self.fields.iter().map(|f| f.name.as_str()).collect();
        for key in values.keys() {
            if !known.contains(key.as_str()) {
                return Err(ModuleError::InvalidRequest(format!(
                    "campo desconocido en la solicitud: '{}'",
                    key
                )));
            }
        }

        for field in &self.fields {
            let raw_value = values.get(&field.name).map(String::as_str).unwrap_or("");
            let trimmed = raw_value.trim();
            if field.required && trimmed.is_empty() {
                return Err(ModuleError::InvalidRequest(format!(
                    "falta el campo requerido '{}'",
                    field.label
                )));
            }
            if field.field_type == "select"
                && !trimmed.is_empty()
                && !field.options.iter().any(|opt| opt == raw_value)
            {
                return Err(ModuleError::InvalidRequest(format!(
                    "valor inválido para '{}': '{}'",
                    field.label, raw_value
                )));
            }
        }
        Ok(())
    }

    /// Pares `(label, value)` en el orden del template, omitiendo vacíos.
    pub fn ordered_values<'a>(
        &'a self,
        values: &'a BTreeMap<String, String>,
    ) -> Vec<(&'a str, &'a str)> {
        self.fields
            .iter()
            .filter_map(|field| {
                let value = values.get(&field.name)?;
                if value.trim().is_empty() {
                    return None;
                }
                Some((field.label.as_str(), value.as_str()))
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TemplateRecord {
    pub id: String,
    pub name: String,
    /// Se aplana en el JSON como `fields: [...]` para que el frontend no
    /// tenga que anidar `fields.fields`.
    #[serde(flatten)]
    pub definition: TemplateDefinition,
    pub created_at: String,
    pub updated_at: String,
}

/// Metadatos de una solicitud: nunca incluye los valores descifrados.
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
    /// `nonce ‖ ciphertext` (DataKey), JSON de `{ campo: valor }`.
    pub values_enc: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(name: &str, label: &str, kind: &str, required: bool) -> TemplateField {
        TemplateField {
            name: name.to_string(),
            label: label.to_string(),
            field_type: kind.to_string(),
            required,
            options: Vec::new(),
        }
    }

    #[test]
    fn template_validation_rejects_empty_duplicate_and_bad_select() {
        assert!(TemplateDefinition { fields: vec![] }.validate().is_err());

        let duplicate = TemplateDefinition {
            fields: vec![
                field("estudio", "Estudio", "text", true),
                field("estudio", "Otro", "text", false),
            ],
        };
        assert!(duplicate.validate().is_err());

        let bad_select = TemplateDefinition {
            fields: vec![field("prioridad", "Prioridad", "select", true)],
        };
        assert!(bad_select.validate().is_err());

        let unknown_type = TemplateDefinition {
            fields: vec![field("x", "X", "radio", false)],
        };
        assert!(unknown_type.validate().is_err());
    }

    #[test]
    fn values_validation_enforces_required_and_known_fields() {
        let template = TemplateDefinition {
            fields: vec![
                field("estudio", "Estudio", "text", true),
                field("indicacion", "Indicación", "textarea", false),
                TemplateField {
                    name: "prioridad".to_string(),
                    label: "Prioridad".to_string(),
                    field_type: "select".to_string(),
                    required: true,
                    options: vec!["Rutina".to_string(), "Urgente".to_string()],
                },
            ],
        };

        let mut values = BTreeMap::new();
        values.insert("estudio".to_string(), "Rx tórax".to_string());
        values.insert("prioridad".to_string(), "Rutina".to_string());
        template.validate_values(&values).expect("válido");

        // Valor fuera de `options` en un select → rechazado.
        values.insert("prioridad".to_string(), "Inmediata".to_string());
        assert!(template.validate_values(&values).is_err());
        values.insert("prioridad".to_string(), "Rutina".to_string());

        values.insert("otro".to_string(), "x".to_string());
        assert!(template.validate_values(&values).is_err());

        values.remove("otro");
        values.remove("estudio");
        assert!(template.validate_values(&values).is_err());
    }
}
