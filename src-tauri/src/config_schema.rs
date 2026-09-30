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

// ========== ENTITY SCHEMA METHODS ==========

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
