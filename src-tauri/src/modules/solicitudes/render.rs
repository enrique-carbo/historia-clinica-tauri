//! Render puro de una solicitud (sin I/O, testeable).
//!
//! Devuelve Markdown (preview/descarga) y HTML (impresión / guardar como PDF
//! desde el WebView). El PDF nativo queda para cuando haya una librería
//! elegida; el documento y los datos no cambian.

use super::models::TemplateDefinition;
use crate::core::dto::{FiliationDto, ProfessionalDto};
use std::collections::BTreeMap;

struct DocumentView {
    study_name: String,
    header: Vec<(String, String)>,
    data: Vec<(String, String)>,
}

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

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn build_view(
    filiation: &FiliationDto,
    professional: &ProfessionalDto,
    study_name: &str,
    values: &BTreeMap<String, String>,
    template: Option<&TemplateDefinition>,
    created_at: &str,
) -> DocumentView {
    let mut header = Vec::new();
    header.push(("Fecha".to_string(), created_at.to_string()));
    header.push((
        "Profesional".to_string(),
        format!("{} ({})", professional.full_name, professional.profession),
    ));
    if !professional.licenses.is_empty() {
        let licenses = professional
            .licenses
            .iter()
            .map(|license| format!("{} ({})", license.number, license.jurisdiction))
            .collect::<Vec<_>>()
            .join(", ");
        header.push(("Matrículas".to_string(), licenses));
    }
    header.push(("Paciente".to_string(), filiation.full_name.clone()));
    if !filiation.document.is_empty() {
        header.push(("Documento".to_string(), filiation.document.clone()));
    }
    for field in &filiation.fields {
        if matches!(field.key.as_str(), "nombre" | "apellido" | "dni")
            || field.value.trim().is_empty()
        {
            continue;
        }
        header.push((field.label.clone(), field.value.clone()));
    }

    let data: Vec<(String, String)> = match template {
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
                    && !definition.fields.iter().any(|field| field.name == *key)
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

    DocumentView {
        study_name: study_name.to_string(),
        header,
        data,
    }
}

pub fn render_solicitud(
    filiation: &FiliationDto,
    professional: &ProfessionalDto,
    study_name: &str,
    values: &BTreeMap<String, String>,
    template: Option<&TemplateDefinition>,
    created_at: &str,
) -> String {
    let view = build_view(filiation, professional, study_name, values, template, created_at);

    let mut out = String::new();
    out.push_str(&format!("# Solicitud de estudio: {}\n\n", view.study_name));
    for (label, value) in &view.header {
        out.push_str(&format!("- **{}:** {}\n", label, one_line(value)));
    }

    out.push_str("\n## Datos del estudio\n\n");
    if view.data.is_empty() {
        out.push_str("- (sin datos)\n");
    } else {
        for (label, value) in &view.data {
            out.push_str(&format!("- **{}:** {}\n", label, one_line(value)));
        }
    }

    out
}

pub fn render_solicitud_html(
    filiation: &FiliationDto,
    professional: &ProfessionalDto,
    study_name: &str,
    values: &BTreeMap<String, String>,
    template: Option<&TemplateDefinition>,
    created_at: &str,
) -> String {
    let view = build_view(filiation, professional, study_name, values, template, created_at);

    let mut html = String::new();
    html.push_str("<!doctype html>\n<html lang=\"es\">\n<head>\n");
    html.push_str("<meta charset=\"utf-8\">\n");
    html.push_str(&format!(
        "<title>Solicitud de estudio: {}</title>\n",
        escape_html(&view.study_name)
    ));
    html.push_str(
        r#"<style>
            :root { color-scheme: light; }
            body { font-family: system-ui, -apple-system, "Segoe UI", sans-serif;
                   color: #18181b; margin: 2rem; line-height: 1.45; }
            h1 { font-size: 1.25rem; margin: 0 0 1rem; }
            h2 { font-size: 1rem; margin: 1.5rem 0 .5rem; padding-bottom: .25rem;
                 border-bottom: 1px solid #d4d4d8; }
            dl { display: grid; grid-template-columns: max-content 1fr;
                 gap: .3rem 1rem; margin: 0; }
            dt { font-weight: 600; color: #52525b; }
            dd { margin: 0; white-space: pre-wrap; }
            .muted { color: #71717a; }
            @media print { body { margin: 1cm; } .no-print { display: none; } }
        </style>
        </head>
        <body>
        "#,
    );
    html.push_str(&format!(
        "<h1>Solicitud de estudio: {}</h1>\n<dl>\n",
        escape_html(&view.study_name)
    ));
    for (label, value) in &view.header {
        html.push_str(&format!(
            "<dt>{}</dt><dd>{}</dd>\n",
            escape_html(label),
            escape_html(&one_line(value))
        ));
    }
    html.push_str("</dl>\n<h2>Datos del estudio</h2>\n");
    if view.data.is_empty() {
        html.push_str("<p class=\"muted\">(sin datos)</p>\n");
    } else {
        html.push_str("<dl>\n");
        for (label, value) in &view.data {
            html.push_str(&format!(
                "<dt>{}</dt><dd>{}</dd>\n",
                escape_html(label),
                escape_html(&one_line(value))
            ));
        }
        html.push_str("</dl>\n");
    }
    html.push_str("</body>\n</html>\n");
    html
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::dto::{FiliationFieldDto, License};

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

    fn template() -> TemplateDefinition {
        TemplateDefinition {
            fields: vec![
                crate::modules::solicitudes::models::TemplateField {
                    name: "estudio".to_string(),
                    label: "Estudio".to_string(),
                    field_type: "text".to_string(),
                    required: true,
                    options: Vec::new(),
                },
                crate::modules::solicitudes::models::TemplateField {
                    name: "indicacion".to_string(),
                    label: "Indicación".to_string(),
                    field_type: "textarea".to_string(),
                    required: false,
                    options: Vec::new(),
                },
            ],
        }
    }

    #[test]
    fn render_uses_template_labels_and_order() {
        let template = template();
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
        let estudio = markdown.find("**Estudio:**").expect("estudio");
        let indicacion = markdown.find("**Indicación:**").expect("indicacion");
        assert!(estudio < indicacion);
        assert!(markdown.contains("Dolor torácico persistente"));
    }

    #[test]
    fn render_without_template_humanizes_keys() {
        let mut values = BTreeMap::new();
        values.insert("estudio_solicitado".to_string(), "Cultivo".to_string());

        let markdown = render_solicitud(
            &filiation(),
            &professional(),
            "Cultivo",
            &values,
            None,
            "2026-11-22T10:00:00Z",
        );

        assert!(markdown.contains("**Estudio Solicitado:** Cultivo"));
    }

    #[test]
    fn render_keeps_values_missing_from_edited_template() {
        let template = TemplateDefinition {
            fields: vec![crate::modules::solicitudes::models::TemplateField {
                name: "estudio".to_string(),
                label: "Estudio".to_string(),
                field_type: "text".to_string(),
                required: true,
                options: Vec::new(),
            }],
        };
        let mut values = BTreeMap::new();
        values.insert("estudio".to_string(), "Rx tórax".to_string());
        values.insert("indicacion".to_string(), "control".to_string());

        let markdown = render_solicitud(
            &filiation(),
            &professional(),
            "Rx tórax",
            &values,
            Some(&template),
            "2026-11-22T10:00:00Z",
        );
        // El template ya no tiene el campo, así que el label se humaniza
        // desde la clave (sin tilde); el valor no se pierde.
        assert!(markdown.contains("**Indicacion:** control"));
    }

    #[test]
    fn render_includes_extra_filiation_fields_but_not_duplicates() {
        let mut fil = filiation();
        fil.fields = vec![
            FiliationFieldDto {
                key: "nombre".to_string(),
                label: "Nombre".to_string(),
                value: "Juan".to_string(),
            },
            FiliationFieldDto {
                key: "seguro_salud".to_string(),
                label: "Seguro Salud".to_string(),
                value: "OSDE".to_string(),
            },
        ];

        let markdown = render_solicitud(
            &fil,
            &professional(),
            "Cultivo",
            &BTreeMap::new(),
            None,
            "2026-11-22T10:00:00Z",
        );
        assert!(markdown.contains("**Seguro Salud:** OSDE"));
        assert!(!markdown.contains("**Nombre:** Juan"));
    }

    #[test]
    fn render_html_escapes_values_and_has_print_styles() {
        let mut fil = filiation();
        fil.full_name = "<b>Juan</b> & Cía".to_string();
        let mut values = BTreeMap::new();
        values.insert("detalle".to_string(), "a < b & c".to_string());

        let html = render_solicitud_html(
            &fil,
            &professional(),
            "Rx <tórax>",
            &values,
            None,
            "2026-11-22T10:00:00Z",
        );

        assert!(html.contains("<!doctype html>"));
        assert!(html.contains("@media print"));
        assert!(html.contains("Rx &lt;tórax&gt;"));
        assert!(html.contains("&lt;b&gt;Juan&lt;/b&gt; &amp; Cía"));
        assert!(html.contains("a &lt; b &amp; c"));
        assert!(!html.contains("<b>Juan</b>"));
    }
}
