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
    #[serde(default)]
    pub encrypted: bool,
    #[serde(rename = "blind_index")]
    pub blind_index: Option<bool>,
    pub options: Option<Vec<String>>,
    /// Subcampos cuando `field_type == "group"`. El storage sigue plano: cada
    /// subcampo se guarda con la clave `{group.name}_{sub.name}` (prefijo).
    pub fields: Option<Vec<FieldSchema>>,
}

// ========== ENTITY SCHEMA METHODS ==========

/// Único punto de carga del schema embebido en el binario.
/// Fuente: `profiles/{FICHIO_PROFILE}/entity_schema.json` (med|vet, default
/// med), copiado a OUT_DIR por `build.rs`. El frontend consume los mismos
/// archivos vía `src/utils/schema.ts` (selección por `VITE_FICHIO_PROFILE`).
pub const SCHEMA_JSON: &str = include_str!(concat!(env!("OUT_DIR"), "/entity_schema.json"));

pub fn from_embedded() -> Result<SchemaConfig, String> {
    serde_json::from_str(SCHEMA_JSON).map_err(|e| format!("Schema inválido: {}", e))
}

/// Clave de storage de un campo: los subcampos de un group se guardan
/// aplanados con prefijo `{group}_{campo}` (misma convención en el frontend).
fn storage_key(prefix: Option<&str>, name: &str) -> String {
    match prefix {
        Some(p) => format!("{}_{}", p, name),
        None => name.to_string(),
    }
}

fn is_non_empty(data: &HashMap<String, String>, key: &str) -> bool {
    data.get(key).map(|s| !s.trim().is_empty()).unwrap_or(false)
}

fn collect_field_labels(
    fields: &[FieldSchema],
    prefix: Option<&str>,
    out: &mut Vec<(String, String)>,
) {
    for field in fields {
        let key = storage_key(prefix, &field.name);
        match &field.fields {
            Some(subfields) => collect_field_labels(subfields, Some(&key), out),
            None => out.push((key, field.label.clone())),
        }
    }
}

fn validate_fields(
    fields: &[FieldSchema],
    prefix: Option<&str>,
    group_active: bool,
    data: &HashMap<String, String>,
) -> Result<(), String> {
    for field in fields {
        let key = storage_key(prefix, &field.name);
        if let Some(subfields) = &field.fields {
            // Grupo: sus required internos aplican solo si el grupo tiene
            // algún dato ("nada o todo") o si el grupo mismo es required.
            let has_data = subfields.iter().any(|sub| match &sub.fields {
                Some(_) => false,
                None => is_non_empty(data, &storage_key(Some(&key), &sub.name)),
            });
            validate_fields(subfields, Some(&key), field.required || has_data, data)?;
        } else if field.required && (prefix.is_none() || group_active) && !is_non_empty(data, &key) {
            return Err(format!("Campo requerido faltante: {}", field.label));
        }
    }
    Ok(())
}

impl SchemaConfig {
    /// Labels (name, label) de los campos de una entidad, en orden del schema.
    /// Los groups se aplanan con prefijo (ver `storage_key`). Es la fuente de
    /// verdad para formularios, export y vistas.
    pub fn field_labels(&self, entity_type: &str) -> Result<Vec<(String, String)>, String> {
        let schema = self
            .entities
            .get(entity_type)
            .ok_or_else(|| format!("Tipo de entidad desconocido: {}", entity_type))?;
        let mut out = Vec::new();
        collect_field_labels(&schema.fields, None, &mut out);
        Ok(out)
    }

    pub fn validate_entity_data(
        &self,
        entity_type: &str,
        data: &HashMap<String, String>,
    ) -> Result<(), String> {
        let schema = self
            .entities
            .get(entity_type)
            .ok_or_else(|| format!("Tipo de entidad desconocido: {}", entity_type))?;
        validate_fields(&schema.fields, None, false, data)
    }

    pub fn get_blind_index_field(&self, entity_type: &str) -> Result<String, String> {
        let schema = self
            .entities
            .get(entity_type)
            .ok_or_else(|| format!("Tipo de entidad desconocido: {}", entity_type))?;
        Ok(schema.blind_index_field.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixture propia: estos tests prueban el MECANISMO (prefijo de groups +
    /// required condicional), no el contenido de entity_schema.json — editar
    /// labels, campos o required del schema real no debe romperlos.
    fn fixture_schema() -> SchemaConfig {
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
                            {"name": "telefono", "label": "Teléfono", "type": "tel",
                             "required": false, "encrypted": true},
                            {"name": "responsable", "label": "Responsable - Tutor",
                             "type": "group", "required": false,
                             "fields": [
                                {"name": "nombre_completo", "label": "Nombre y Apellido Tutor",
                                 "type": "text", "required": true, "encrypted": true},
                                {"name": "telefono", "label": "Teléfono de Contacto",
                                 "type": "tel", "required": true, "encrypted": true}
                             ]}
                        ]
                    }
                }
            }"#,
        )
        .expect("fixture de schema válida")
    }

    fn base_patient_data() -> HashMap<String, String> {
        let mut data = HashMap::new();
        data.insert("nombre".to_string(), "Ana".to_string());
        data
    }

    #[test]
    fn embedded_schema_parses_and_has_unique_keys() {
        // Guardia de contenido: el JSON real debe parsear y el default_entity
        // debe existir con labels sin claves duplicadas (los prefijos de
        // groups evitan colisionar con campos top-level).
        let s = from_embedded().expect("entity_schema.json embebido válido");
        let labels = s.field_labels(&s.default_entity).expect("default_entity existe");
        assert!(!labels.is_empty(), "sin campos para default_entity");
        let unique: std::collections::HashSet<_> = labels.iter().map(|(k, _)| k).collect();
        assert_eq!(unique.len(), labels.len(), "claves duplicadas en field_labels");
    }

    #[test]
    fn field_labels_flattens_groups_with_prefix() {
        let labels = fixture_schema().field_labels("patient").expect("labels");
        assert!(labels.contains(&("nombre".to_string(), "Nombre".to_string())));
        assert!(labels.contains(&(
            "responsable_nombre_completo".to_string(),
            "Nombre y Apellido Tutor".to_string()
        )));
        assert!(labels.contains(&(
            "responsable_telefono".to_string(),
            "Teléfono de Contacto".to_string()
        )));
        // El prefijo desambigua: el teléfono del paciente NO lleva prefijo.
        assert!(labels.contains(&("telefono".to_string(), "Teléfono".to_string())));
        // Orden: el grupo va al final, en la posición del schema.
        let keys: Vec<&str> = labels.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(
            keys,
            vec!["nombre", "telefono", "responsable_nombre_completo", "responsable_telefono"]
        );
    }

    #[test]
    fn group_required_applies_only_when_partially_filled() {
        let s = fixture_schema();
        let mut data = base_patient_data();

        // Grupo vacío con required:false → válido.
        s.validate_entity_data("patient", &data).expect("grupo vacío");

        // Grupo parcial → el subcampo required faltante debe rechazar.
        data.insert(
            "responsable_nombre_completo".to_string(),
            "Juan Tutor".to_string(),
        );
        let err = s
            .validate_entity_data("patient", &data)
            .expect_err("grupo parcial debe fallar");
        assert!(err.contains("Teléfono de Contacto"), "mensaje: {}", err);

        // Grupo completo → válido.
        data.insert("responsable_telefono".to_string(), "555-1234".to_string());
        s.validate_entity_data("patient", &data)
            .expect("grupo completo");
    }

    #[test]
    fn top_level_required_still_enforced() {
        let s = fixture_schema();
        let mut data = base_patient_data();
        data.remove("nombre");
        let err = s
            .validate_entity_data("patient", &data)
            .expect_err("nombre es required");
        assert!(err.contains("Nombre"), "mensaje: {}", err);
    }
}
