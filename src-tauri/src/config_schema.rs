use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct SchemaConfig {
    pub version: i32,
    #[serde(rename = "default_entity")]
    pub default_entity: String,
    pub entities: HashMap<String, EntitySchema>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct EntitySchema {
    pub label: String,
    #[serde(rename = "label_plural")]
    pub label_plural: String,
    pub icon: Option<String>,
    #[serde(rename = "blind_index_field")]
    pub blind_index_field: String,
    pub fields: Vec<FieldSchema>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct FieldSchema {
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub required: bool,
    pub encrypted: bool,
    #[serde(rename = "blind_index")]
    pub blind_index: Option<bool>,
    pub options: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct NoteTemplatesConfig {
    pub version: i32,
    pub templates: HashMap<String, NoteTemplate>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct NoteTemplate {
    pub label: String,
    pub description: String,
    pub icon: Option<String>,
    pub immutable: bool,
    pub requires_signature: bool,
    #[serde(rename = "signature_payload_order")]
    pub signature_payload_order: Vec<String>,
    pub fields: Vec<NoteFieldSchema>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct NoteFieldSchema {
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub required: bool,
    pub placeholder: Option<String>,
    pub rows: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct MedicalHistorySchemaConfig {
    pub version: i32,
    pub label: String,
    pub fields: Vec<FieldSchema>,
}

impl NoteTemplatesConfig {
    /// Valida que los campos requeridos del template estén presentes
    pub fn validate_note_fields(
        &self,
        template_id: &str,
        fields: &HashMap<String, String>,
    ) -> Result<(), String> {
        let template = self
            .templates
            .get(template_id)
            .ok_or_else(|| format!("Template desconocido: {}", template_id))?;

        for field in &template.fields {
            if field.required {
                let value = fields.get(&field.name).map(|s| s.trim()).unwrap_or("");
                if value.is_empty() {
                    return Err(format!("Campo requerido faltante: {}", field.label));
                }
            }
        }
        Ok(())
    }

    /// Construye el payload de firma según signature_payload_order
    pub fn build_signature_payload(
        &self,
        template_id: &str,
        fields: &HashMap<String, String>,
    ) -> Result<String, String> {
        let template = self
            .templates
            .get(template_id)
            .ok_or_else(|| format!("Template desconocido: {}", template_id))?;

        let mut parts = Vec::new();
        for field_name in &template.signature_payload_order {
            let value = fields
                .get(field_name)
                .ok_or_else(|| format!("Campo de firma faltante: {}", field_name))?;
            parts.push(value.as_str());
        }

        Ok(parts.join("|"))
    }
}

// ========== ENTITY SCHEMA METHODS (ya existentes) ==========

impl SchemaConfig {
    pub fn validate_entity_data(
        &self,
        entity_type: &str,
        data: &HashMap<String, String>,
    ) -> Result<(), String> {
        let schema = self
            .entities
            .get(entity_type)
            .ok_or_else(|| format!("Tipo de entidad desconocido: {}", entity_type))?;

        for field in &schema.fields {
            if field.required {
                let value = data.get(&field.name).map(|s| s.trim()).unwrap_or("");
                if value.is_empty() {
                    return Err(format!("Campo requerido faltante: {}", field.label));
                }
            }
        }
        Ok(())
    }

    pub fn get_blind_index_field(&self, entity_type: &str) -> Result<String, String> {
        let schema = self
            .entities
            .get(entity_type)
            .ok_or_else(|| format!("Tipo de entidad desconocido: {}", entity_type))?;
        Ok(schema.blind_index_field.clone())
    }
}

impl MedicalHistorySchemaConfig {
    /// Valida que los campos requeridos estén presentes
    pub fn validate(&self, data: &HashMap<String, String>) -> Result<(), String> {
        for field in &self.fields {
            if field.required {
                let value = data.get(&field.name).map(|s| s.trim()).unwrap_or("");
                if value.is_empty() {
                    return Err(format!("Campo requerido faltante: {}", field.label));
                }
            }
        }
        Ok(())
    }
}
