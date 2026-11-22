//! Render puro de una solicitud (sin I/O, testeable).
//!
//! Corte 5: devuelve Markdown. El PDF/impresión se resuelve en el corte 7
//! (misma función, otro formateador) sin tocar la persistencia.

use super::models::TemplateDefinition;
use crate::core::dto::{FiliationDto, ProfessionalDto};
use std::collections::BTreeMap;

fn one_line(value: &str) -> String {
    value.replace(['\r', '\n'], " ").trim().to_string()
}

fn humanize(key: &str) -> String {
    key.split(['_', '-'])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn render_solicitud(
    filiation: &FiliationDto,
    professional: &ProfessionalDto,
    study_name: &str,
    values: &BTreeMap<String, String>,
    template: Option<&TemplateDefinition>,
    created_at: &str,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Solicitud de estudio: {}\n\n", study_name));
    out.push_str(&format!("- **Fecha:** {}\n", created_at));
    out.push_str(&format!(
        "- **Profesional:** {} ({})\n",
        professional.full_name, professional.profession
    ));
    if !professional.licenses.is_empty() {
        let licenses = professional
            .licenses
            .iter()
            .map(|license| format!("{} ({})", license.number, license.jurisdiction))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("- **Matrículas:** {}\n", licenses));
    }
    out.push_str(&format!("- **Paciente:** {}\n", filiation.full_name));
    if !filiation.document.is_empty() {
        out.push_str(&format!("- **Documento:** {}\n", filiation.document));
    }
    for field in &filiation.fields {
        if matches!(field.key.as_str(), "nombre" | "apellido" | "dni")
            || field.value.trim().is_empty()
        {
            continue;
        }
        out.push_str(&format!(
            "- **{}:** {}\n",
            field.label,
            one_line(&field.value)
        ));
    }

    out.push_str("\n## Datos del estudio\n\n");

    let pairs: Vec<(String, String)> = match template {
        Some(definition) => {
            let mut ordered: Vec<(String, String)> = definition
                .ordered_values(values)
                .into_iter()
                .map(|(label, value)| (label.to_string(), value.to_string()))
                .collect();
            // Regla de oro: si el template fue editado después de emitir la
            // solicitud y algún campo guardado en `values` ya no figura en
            // `definition.fields`, se conserva al final sin perder datos.
            for (key, value) in values {
                if !value.trim().is_empty()
                    && !definition.fields.iter().any(|f| f.name == *key)
                {
                    ordered.push((humanize(key), value.clone()));
                }
            }
            ordered
        }
        None => values
            .iter()
            .filter(|(_, value)| !value.trim().is_empty())
            .map(|(key, value)| (humanize(key), value.clone()))
            .collect(),
    };

    if pairs.is_empty() {
        out.push_str("- (sin datos)\n");
    } else {
        for (label, value) in pairs {
            out.push_str(&format!("- **{}:** {}\n", label, one_line(&value)));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::dto::License;

    fn filiation() -> FiliationDto {
        FiliationDto {
            entity_id: 1,
            full_name: "Juan Pérez".to_string(),
            document: "20111222".to_string(),
            fields: Vec::new(),
        }
    }

    fn professional() -> ProfessionalDto {
        ProfessionalDto {
            user_id: "prof-1".to_string(),
            full_name: "Dra. Ana".to_string(),
            profession: "Médica".to_string(),
            specialty: "Clínica".to_string(),
            licenses: vec![License {
                number: "MP-1".to_string(),
                jurisdiction: "Córdoba".to_string(),
            }],
            public_key_hex: "pk".to_string(),
        }
    }

    #[test]
    fn render_uses_template_labels_and_order() {
        let template = TemplateDefinition {
            fields: vec![
                super::super::models::TemplateField {
                    name: "estudio".to_string(),
                    label: "Estudio".to_string(),
                    field_type: "text".to_string(),
                    required: true,
                    options: Vec::new(),
                },
                super::super::models::TemplateField {
                    name: "indicacion".to_string(),
                    label: "Indicación".to_string(),
                    field_type: "textarea".to_string(),
                    required: false,
                    options: Vec::new(),
                },
            ],
        };
        let mut values = BTreeMap::new();
        values.insert("indicacion".to_string(), "Dolor torácico\npersistente".to_string());
        values.insert("estudio".to_string(), "Rx tórax".to_string());

        let markdown = render_solicitud(
            &filiation(),
            &professional(),
            "Rx tórax",
            &values,
            Some(&template),
            "2026-11-22T10:00:00Z",
        );

        assert!(markdown.contains("# Solicitud de estudio: Rx tórax"));
        assert!(markdown.contains("**Paciente:** Juan Pérez"));
        assert!(markdown.contains("**Matrículas:** MP-1 (Córdoba)"));
        // Orden del template: Estudio antes que Indicación.
        let estudio = markdown.find("**Estudio:**").expect("estudio");
        let indicacion = markdown.find("**Indicación:**").expect("indicacion");
        assert!(estudio < indicacion);
        // Saltos de línea colapsados en la viñeta.
        assert!(markdown.contains("Dolor torácico persistente"));
    }

    #[test]
    fn render_without_template_humanizes_keys() {
        let mut values = BTreeMap::new();
        values.insert("estudio_solicitado".to_string(), "Cultivo".to_string());

        let mut fil = filiation();
        fil.fields = vec![
            crate::core::dto::FiliationFieldDto {
                key: "nombre".to_string(),
                label: "Nombre".to_string(),
                value: "Juan".to_string(),
            },
            crate::core::dto::FiliationFieldDto {
                key: "dni".to_string(),
                label: "DNI".to_string(),
                value: "20111222".to_string(),
            },
            crate::core::dto::FiliationFieldDto {
                key: "seguro_salud".to_string(),
                label: "Seguro Salud".to_string(),
                value: "OSDE".to_string(),
            },
        ];

        let markdown = render_solicitud(
            &fil,
            &professional(),
            "Cultivo",
            &values,
            None,
            "2026-11-22T10:00:00Z",
        );

        assert!(markdown.contains("**Estudio Solicitado:** Cultivo"));
        assert!(markdown.contains("**Seguro Salud:** OSDE"));
        assert!(!markdown.contains("**Nombre:** Juan"));
        assert!(!markdown.contains("**DNI:** 20111222"));
    }
}
