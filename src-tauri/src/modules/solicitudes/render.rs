//! Render puro de una solicitud (sin I/O, testeable).
//!
//! Documento: encabezado (paciente/profesional/fecha), diagnóstico/indicación,
//! checklist de estudios tildados y notas. Markdown para preview/descarga y
//! HTML para imprimir / guardar como PDF desde el WebView.

use crate::core::dto::{FiliationDto, ProfessionalDto};

struct DocumentView {
    study_name: String,
    header: Vec<(String, String)>,
    items: Vec<String>,
    diagnostico: Option<String>,
    notas: Option<String>,
}

fn one_line(value: &str) -> String {
    value.replace(['\r', '\n'], " ").trim().to_string()
}

/// RFC3339 de `created_at` → fecha/hora local legible (mismo formato que el
/// resto de la app). Si no parsea, se muestra tal cual.
fn format_timestamp(timestamp: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(timestamp)
        .map(|date| {
            date.with_timezone(&chrono::Local)
                .format("%d/%m/%Y %H:%M")
                .to_string()
        })
        .unwrap_or_else(|_| timestamp.to_string())
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
    items: &[String],
    diagnostico: Option<&str>,
    notas: Option<&str>,
    created_at: &str,
) -> DocumentView {
    let mut header = Vec::new();
    header.push(("Fecha".to_string(), format_timestamp(created_at)));
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

    DocumentView {
        study_name: study_name.to_string(),
        header,
        items: items.to_vec(),
        diagnostico: diagnostico.map(normalize_optional),
        notas: notas.map(normalize_optional),
    }
}

fn normalize_optional(value: &str) -> String {
    value.replace("\r\n", "\n").replace('\r', "\n").trim().to_string()
}

pub fn render_solicitud(
    filiation: &FiliationDto,
    professional: &ProfessionalDto,
    study_name: &str,
    items: &[String],
    diagnostico: Option<&str>,
    notas: Option<&str>,
    created_at: &str,
) -> String {
    let view = build_view(
        filiation,
        professional,
        study_name,
        items,
        diagnostico,
        notas,
        created_at,
    );

    let mut out = String::new();
    out.push_str(&format!("# Solicitud de estudio: {}\n\n", view.study_name));
    for (label, value) in &view.header {
        out.push_str(&format!("- **{}:** {}\n", label, one_line(value)));
    }

    out.push_str("\n## Diagnóstico / indicación\n\n");
    match view.diagnostico.as_deref().filter(|text| !text.is_empty()) {
        Some(diagnostico) => out.push_str(&format!("{}\n", diagnostico)),
        None => out.push_str("—\n"),
    }

    out.push_str("\n## Estudios solicitados\n\n");
    if view.items.is_empty() {
        out.push_str("- (sin estudios)\n");
    } else {
        for item in &view.items {
            out.push_str(&format!("- {}\n", one_line(item)));
        }
    }

    if let Some(notas) = view.notas.as_deref().filter(|text| !text.is_empty()) {
        out.push_str("\n## Notas\n\n");
        out.push_str(&format!("{}\n", notas));
    }

    out
}

pub fn render_solicitud_html(
    filiation: &FiliationDto,
    professional: &ProfessionalDto,
    study_name: &str,
    items: &[String],
    diagnostico: Option<&str>,
    notas: Option<&str>,
    created_at: &str,
) -> String {
    let view = build_view(
        filiation,
        professional,
        study_name,
        items,
        diagnostico,
        notas,
        created_at,
    );

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
            p { margin: .25rem 0; }
            strong { color: #52525b; }
            ul { margin: 0; padding-left: 1.25rem; }
            .muted { color: #71717a; }
            .block { white-space: pre-wrap; }
            @media print { body { margin: 1cm; } .no-print { display: none; } }
        </style>
        </head>
        <body>
        "#,
    );
    html.push_str(&format!(
        "<h1>Solicitud de estudio: {}</h1>\n",
        escape_html(&view.study_name)
    ));
    // `<p><strong>...</strong> valor</p>` en vez de `<dl>`: se lee bien aunque
    // el CSS no aplique (impresión/visores de HTML simples).
    for (label, value) in &view.header {
        html.push_str(&format!(
            "<p><strong>{}:</strong> {}</p>\n",
            escape_html(label),
            escape_html(&one_line(value))
        ));
    }

    html.push_str("<h2>Diagnóstico / indicación</h2>\n");
    match view.diagnostico.as_deref().filter(|text| !text.is_empty()) {
        Some(diagnostico) => html.push_str(&format!(
            "<p class=\"block\">{}</p>\n",
            escape_html(diagnostico)
        )),
        None => html.push_str("<p class=\"muted\">—</p>\n"),
    }

    html.push_str("<h2>Estudios solicitados</h2>\n");
    if view.items.is_empty() {
        html.push_str("<p class=\"muted\">(sin estudios)</p>\n");
    } else {
        html.push_str("<ul>\n");
        for item in &view.items {
            html.push_str(&format!("<li>{}</li>\n", escape_html(&one_line(item))));
        }
        html.push_str("</ul>\n");
    }

    if let Some(notas) = view.notas.as_deref().filter(|text| !text.is_empty()) {
        html.push_str("<h2>Notas</h2>\n");
        html.push_str(&format!(
            "<p class=\"block\">{}</p>\n",
            escape_html(notas)
        ));
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

    fn items() -> Vec<String> {
        vec![
            "Hemograma completo".to_string(),
            "Glucemia".to_string(),
        ]
    }

    #[test]
    fn render_lists_items_diagnostico_and_notas() {
        let markdown = render_solicitud(
            &filiation(),
            &professional(),
            "Laboratorio general",
            &items(),
            Some("Control anemia"),
            Some("Paciente en ayunas"),
            "2026-11-22T10:00:00Z",
        );

        assert!(markdown.contains("# Solicitud de estudio: Laboratorio general"));
        assert!(markdown.contains("**Paciente:** Juan Pérez"));
        assert!(markdown.contains("**Matrículas:** MP-1 (Córdoba)"));
        // La fecha se muestra local y legible, no en RFC3339 crudo.
        assert!(markdown.contains("**Fecha:**"));
        assert!(!markdown.contains("2026-11-22T10:00:00Z"));
        assert!(markdown.contains("## Diagnóstico / indicación"));
        assert!(markdown.contains("Control anemia"));
        assert!(markdown.contains("## Estudios solicitados"));
        let hemograma = markdown.find("- Hemograma completo").expect("hemograma");
        let glucemia = markdown.find("- Glucemia").expect("glucemia");
        assert!(hemograma < glucemia);
        assert!(markdown.contains("## Notas"));
        assert!(markdown.contains("Paciente en ayunas"));
    }

    #[test]
    fn render_preserves_line_breaks_in_diagnostico_and_notas() {
        let markdown = render_solicitud(
            &filiation(),
            &professional(),
            "Urocultivo",
            &["Antibiograma".to_string()],
            Some("Disuria\nfiebre"),
            Some("Ayuno\nPrimera orina"),
            "2026-11-22T10:00:00Z",
        );
        assert!(markdown.contains("Disuria\nfiebre"));
        assert!(markdown.contains("Ayuno\nPrimera orina"));

        let html = render_solicitud_html(
            &filiation(),
            &professional(),
            "Urocultivo",
            &["Antibiograma".to_string()],
            Some("Disuria\nfiebre"),
            Some("Ayuno\nPrimera orina"),
            "2026-11-22T10:00:00Z",
        );
        // El CSS usa pre-wrap: los saltos deben sobrevivir al render.
        assert!(html.contains("Disuria\nfiebre"));
        assert!(html.contains("Ayuno\nPrimera orina"));
    }

    #[test]
    fn render_without_diagnostico_or_notas_uses_placeholders() {
        let markdown = render_solicitud(
            &filiation(),
            &professional(),
            "Urocultivo",
            &["Recuento de colonias".to_string()],
            None,
            None,
            "2026-11-22T10:00:00Z",
        );

        assert!(markdown.contains("—"));
        assert!(markdown.contains("- Recuento de colonias"));
        assert!(!markdown.contains("## Notas"));
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
            &["Orina".to_string()],
            None,
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

        let html = render_solicitud_html(
            &fil,
            &professional(),
            "Rx <tórax>",
            &["a < b & c".to_string()],
            Some("dx < urgente"),
            None,
            "2026-11-22T10:00:00Z",
        );

        assert!(html.contains("<!doctype html>"));
        assert!(html.contains("@media print"));
        assert!(html.contains("Rx &lt;tórax&gt;"));
        assert!(html.contains("&lt;b&gt;Juan&lt;/b&gt; &amp; Cía"));
        assert!(html.contains("a &lt; b &amp; c"));
        assert!(html.contains("dx &lt; urgente"));
        assert!(!html.contains("<b>Juan</b>"));
        // Fecha legible y separada de la etiqueta aun sin CSS.
        assert!(html.contains("<strong>Fecha:</strong>"));
        assert!(!html.contains("2026-11-22T10:00:00Z"));
    }
}
