use crate::crypto;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportPatient {
    pub data: HashMap<String, String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportProfessional {
    pub full_name: String,
    pub license_number: String,
    pub specialty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportEntry {
    pub category: String,
    pub title: String,
    pub status: String,
    pub timestamp: String,
    pub author_name: String,
    pub payload: HashMap<String, String>,
    pub signature: String,
    pub is_verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportSnapshot {
    pub patient: ExportPatient,
    pub professional: ExportProfessional,
    pub entries: Vec<ExportEntry>,
    pub generated_at: String,
}

const FIELD_ORDER: &[(&str, &[&str])] = &[
    (
        "SOAP_NOTE",
        &["subjetivo", "objetivo", "evaluacion", "plan"],
    ),
    (
        "ALLERGY",
        &["sustancia", "tipo_reaccion", "descripcion"],
    ),
    (
        "MEDICATION",
        &[
            "medicamento",
            "dosis",
            "frecuencia",
            "via_administracion",
            "indicacion",
        ],
    ),
    (
        "CONDITION",
        &["diagnostico", "codigo_cie10", "estado", "observaciones"],
    ),
];

const FIELD_LABELS: &[(&str, &str)] = &[
    ("subjetivo", "Subjetivo"),
    ("objetivo", "Objetivo"),
    ("evaluacion", "Evaluación"),
    ("plan", "Plan"),
    ("sustancia", "Sustancia"),
    ("tipo_reaccion", "Tipo de Reacción"),
    ("descripcion", "Descripción"),
    ("medicamento", "Medicamento"),
    ("dosis", "Dosis"),
    ("frecuencia", "Frecuencia"),
    ("via_administracion", "Vía de Administración"),
    ("indicacion", "Indicación"),
    ("diagnostico", "Diagnóstico"),
    ("codigo_cie10", "Código CIE-10"),
    ("estado", "Estado"),
    ("observaciones", "Observaciones"),
];

const CATEGORY_LABELS: &[(&str, &str)] = &[
    ("SOAP_NOTE", "Nota SOAP"),
    ("ALLERGY", "Alergia"),
    ("MEDICATION", "Medicación"),
    ("CONDITION", "Condición / Diagnóstico"),
];

const STATUS_LABELS: &[(&str, &str)] = &[
    ("ACTIVE", "Activo"),
    ("RESOLVED", "Resuelto"),
    ("COMPLETED", "Completado"),
];

const PATIENT_LABELS: &[(&str, &str)] = &[
    ("nombre", "Nombre"),
    ("apellido", "Apellido"),
    ("dni", "DNI / Documento"),
    ("fecha_nacimiento", "Fecha de Nacimiento"),
    ("telefono", "Teléfono"),
    ("email", "Correo Electrónico"),
    ("direccion", "Dirección"),
];

fn label_of<'a>(list: &[(&'a str, &'a str)], key: &'a str) -> &'a str {
    list.iter()
        .find(|(k, _)| *k == key)
        .map(|(_, label)| *label)
        .unwrap_or(key)
}

fn humanize(key: &str) -> String {
    key.split(|c| c == '_' || c == '-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    first.to_uppercase().collect::<String>() + chars.as_str()
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn ordered_payload(category: &str, payload: &HashMap<String, String>) -> Vec<(String, String)> {
    let empty: &[&str] = &[];
    let order = FIELD_ORDER
        .iter()
        .find(|(c, _)| *c == category)
        .map(|(_, fields)| *fields)
        .unwrap_or(empty);

    let mut keys: Vec<&String> = payload.keys().collect();
    keys.sort_by(|a, b| {
        let ia = order.iter().position(|f| f == a);
        let ib = order.iter().position(|f| f == b);
        match (ia, ib) {
            (Some(x), Some(y)) => x.cmp(&y),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => a.cmp(b),
        }
    });

    keys.into_iter()
        .map(|k| (k.clone(), payload[k].clone()))
        .collect()
}

fn age_from_birth(birth: &str) -> Option<i64> {
    use chrono::Datelike;
    let date = chrono::NaiveDate::parse_from_str(birth.trim(), "%Y-%m-%d").ok()?;
    let today = chrono::Local::now().date_naive();
    if date > today {
        return None;
    }
    let mut age = today.year() as i64 - date.year() as i64;
    if (today.month(), today.day()) < (date.month(), date.day()) {
        age -= 1;
    }
    Some(age)
}

fn format_timestamp(ts: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(ts)
        .map(|dt| {
            dt.with_timezone(&chrono::Local)
                .format("%d/%m/%Y %H:%M")
                .to_string()
        })
        .unwrap_or_else(|_| ts.to_string())
}

fn escape_cell(value: &str) -> String {
    value
        .replace('|', "\\|")
        .replace('\r', "")
        .replace('\n', " ")
}

fn full_patient_name(snapshot: &ExportSnapshot) -> String {
    let nombre = snapshot.patient.data.get("nombre").map(String::as_str).unwrap_or("");
    let apellido = snapshot.patient.data.get("apellido").map(String::as_str).unwrap_or("");
    format!("{} {}", nombre.trim(), apellido.trim())
        .trim()
        .to_string()
}

pub fn render_markdown(snapshot: &ExportSnapshot) -> String {
    let mut out = String::new();

    let patient_name = full_patient_name(snapshot);
    let heading = if patient_name.is_empty() {
        "Paciente".to_string()
    } else {
        patient_name
    };

    out.push_str(&format!("# Historia Clínica — {}\n\n", heading));
    out.push_str(&format!(
        "> Documento generado el {} · Simplex Health Core\n",
        snapshot.generated_at
    ));
    out.push_str(&format!(
        "> Entries incluidas: {}\n\n",
        snapshot.entries.len()
    ));

    out.push_str("## Datos del paciente\n\n");
    out.push_str("| Campo | Valor |\n|---|---|\n");
    for (key, label) in PATIENT_LABELS {
        let value = snapshot
            .patient
            .data
            .get(*key)
            .map(String::as_str)
            .unwrap_or("");
        if value.trim().is_empty() {
            continue;
        }
        let mut display = value.to_string();
        if *key == "fecha_nacimiento" {
            if let Some(age) = age_from_birth(value) {
                display = format!("{} ({} años)", value, age);
            }
        }
        out.push_str(&format!("| {} | {} |\n", label, escape_cell(&display)));
    }
    out.push('\n');

    out.push_str("## Profesional\n\n");
    out.push_str("| Campo | Valor |\n|---|---|\n");
    let professional_rows: [(&str, &str); 3] = [
        ("Nombre", &snapshot.professional.full_name),
        ("Matrícula", &snapshot.professional.license_number),
        ("Especialidad", &snapshot.professional.specialty),
    ];
    for (label, value) in professional_rows {
        if value.trim().is_empty() {
            continue;
        }
        out.push_str(&format!("| {} | {} |\n", label, escape_cell(value)));
    }
    out.push('\n');

    out.push_str("## Resumen\n\n");
    out.push_str(&format!(
        "- **Total de entries:** {}\n",
        snapshot.entries.len()
    ));
    for (category, label) in CATEGORY_LABELS {
        let count = snapshot
            .entries
            .iter()
            .filter(|e| e.category == *category)
            .count();
        out.push_str(&format!("- {}: {}\n", label, count));
    }
    out.push_str("\n---\n\n## Evolución clínica\n");

    let mut entries = snapshot.entries.clone();
    entries.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

    for (index, entry) in entries.iter().enumerate() {
        out.push_str(&format!(
            "\n### {}. {} — {}\n\n",
            index + 1,
            format_timestamp(&entry.timestamp),
            entry.title.replace('\n', " ")
        ));
        out.push_str(&format!(
            "- **Categoría:** {}\n",
            label_of(CATEGORY_LABELS, &entry.category)
        ));
        out.push_str(&format!(
            "- **Estado:** {}\n",
            label_of(STATUS_LABELS, &entry.status)
        ));
        out.push_str(&format!("- **Autor:** {}\n", entry.author_name));
        let verify = if entry.is_verified {
            "✓ verificada"
        } else {
            "✗ sin verificar"
        };
        let sig_len = entry.signature.len().min(16);
        out.push_str(&format!(
            "- **Firma:** {} `{}`\n\n",
            verify,
            &entry.signature[..sig_len]
        ));

        let mut has_content = false;
        for (key, value) in ordered_payload(&entry.category, &entry.payload) {
            if value.trim().is_empty() {
                continue;
            }
            has_content = true;
            let label = label_of(FIELD_LABELS, &key);
            let heading_label = if label == key { humanize(&key) } else { label.to_string() };
            out.push_str(&format!("#### {}\n\n{}\n\n", heading_label, value));
        }
        if !has_content {
            out.push_str("*(Sin contenido)*\n\n");
        }
    }

    let hash = crypto::hash_document(&out);
    out.push_str("---\n\n");
    out.push_str(&format!(
        "> **SHA-256 del documento:** `{}`\n",
        hex::encode(hash)
    ));
    out.push_str("> Este hash permite verificar la integridad del contenido exportado.\n\n");
    out.push_str(
        "*Documento confidencial que contiene información clínica protegida. Generado por Simplex Health Core.*\n",
    );

    out
}

fn sanitize_component(raw: &str) -> String {
    let mut out = String::new();
    for c in raw.trim().chars() {
        if c.is_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

pub fn export_filename(snapshot: &ExportSnapshot) -> String {
    let apellido = snapshot
        .patient
        .data
        .get("apellido")
        .map(|s| sanitize_component(s))
        .unwrap_or_default();
    let nombre = snapshot
        .patient
        .data
        .get("nombre")
        .map(|s| sanitize_component(s))
        .unwrap_or_default();

    let mut parts: Vec<String> = vec!["HistoriaClinica".to_string()];
    parts.extend(
        [apellido, nombre]
            .into_iter()
            .filter(|p| !p.is_empty()),
    );
    let base = parts.join("_");

    format!(
        "{}_{}.md",
        base,
        chrono::Local::now().format("%Y-%m-%d_%H%M%S")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_payload(category: &str) -> HashMap<String, String> {
        let mut payload = HashMap::new();
        match category {
            "SOAP_NOTE" => {
                payload.insert("plan".to_string(), "Enalapril 10mg cada 24h".to_string());
                payload.insert("subjetivo".to_string(), "Cefalea occipital".to_string());
                payload.insert("objetivo".to_string(), "TA 120/80".to_string());
                payload.insert("evaluacion".to_string(), "Hipertensión grado 1".to_string());
            }
            "ALLERGY" => {
                payload.insert("sustancia".to_string(), "Penicilina".to_string());
                payload.insert("tipo_reaccion".to_string(), "Moderada".to_string());
            }
            _ => {}
        }
        payload
    }

    fn sample_snapshot() -> ExportSnapshot {
        let mut data = HashMap::new();
        data.insert("nombre".to_string(), "María José".to_string());
        data.insert("apellido".to_string(), "García López".to_string());
        data.insert("dni".to_string(), "12.345.678".to_string());
        data.insert("fecha_nacimiento".to_string(), "1980-05-12".to_string());

        ExportSnapshot {
            patient: ExportPatient {
                data,
                created_at: "2026-01-01".to_string(),
            },
            professional: ExportProfessional {
                full_name: "Dra. Ana Test".to_string(),
                license_number: "MP-12345".to_string(),
                specialty: "Clínica Médica".to_string(),
            },
            entries: vec![
                ExportEntry {
                    category: "SOAP_NOTE".to_string(),
                    title: "Control".to_string(),
                    status: "ACTIVE".to_string(),
                    timestamp: "2026-01-02T10:00:00+00:00".to_string(),
                    author_name: "Dra. Ana Test".to_string(),
                    payload: sample_payload("SOAP_NOTE"),
                    signature: "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"
                        .to_string(),
                    is_verified: true,
                },
                ExportEntry {
                    category: "ALLERGY".to_string(),
                    title: "Alergia registrada".to_string(),
                    status: "ACTIVE".to_string(),
                    timestamp: "2026-01-01T09:00:00+00:00".to_string(),
                    author_name: "Dra. Ana Test".to_string(),
                    payload: sample_payload("ALLERGY"),
                    signature: "1111222233334444111122223333444411112222333344441111222233334444"
                        .to_string(),
                    is_verified: false,
                },
            ],
            generated_at: "30/09/2026 14:30".to_string(),
        }
    }

    #[test]
    fn renders_header_with_patient_and_professional() {
        let md = render_markdown(&sample_snapshot());
        assert!(md.contains("# Historia Clínica — María José García López"));
        assert!(md.contains("| DNI / Documento | 12.345.678 |"));
        assert!(md.contains("| Nombre | Dra. Ana Test |"));
        assert!(md.contains("| Matrícula | MP-12345 |"));
        assert!(md.contains("> Documento generado el 30/09/2026 14:30"));
    }

    #[test]
    fn renders_entries_in_chronological_order() {
        let md = render_markdown(&sample_snapshot());
        let allergy_pos = md.find("### 1.").expect("primera entry");
        let soap_pos = md.find("### 2.").expect("segunda entry");
        assert!(allergy_pos < soap_pos, "la entry más antigua debe ir primero");
        let slice = &md[allergy_pos..soap_pos];
        assert!(slice.contains("Alergia registrada"));
        assert!(slice.contains("✗ sin verificar"));
        let soap_slice = &md[soap_pos..];
        assert!(soap_slice.contains("✓ verificada"));
    }

    #[test]
    fn uses_canonical_field_order_and_labels() {
        let md = render_markdown(&sample_snapshot());
        let s = md.find("#### Subjetivo").expect("subjetivo");
        let o = md.find("#### Objetivo").expect("objetivo");
        let e = md.find("#### Evaluación").expect("evaluacion");
        let p = md.find("#### Plan").expect("plan");
        assert!(s < o && o < e && e < p, "orden canónico SOAP respetado");
        assert!(md.contains("#### Sustancia"));
        assert!(md.contains("#### Tipo de Reacción"));
    }

    #[test]
    fn footer_contains_sha256_and_warning() {
        let md = render_markdown(&sample_snapshot());
        assert!(md.contains("> **SHA-256 del documento:** `"));
        assert!(md.contains("Documento confidencial"));
    }

    #[test]
    fn filename_is_sanitized() {
        let mut snapshot = sample_snapshot();
        snapshot
            .patient
            .data
            .insert("apellido".to_string(), "  García/López  ".to_string());
        snapshot
            .patient
            .data
            .insert("nombre".to_string(), "María José".to_string());
        let name = export_filename(&snapshot);
        assert!(name.starts_with("HistoriaClinica_García_López_María_José_"));
        assert!(name.ends_with(".md"));
        assert!(!name.contains('/'));
    }

    #[test]
    fn filename_falls_back_when_patient_has_no_name() {
        let snapshot = ExportSnapshot {
            patient: ExportPatient {
                data: HashMap::new(),
                created_at: String::new(),
            },
            professional: ExportProfessional {
                full_name: String::new(),
                license_number: String::new(),
                specialty: String::new(),
            },
            entries: vec![],
            generated_at: String::new(),
        };
        let name = export_filename(&snapshot);
        assert!(name.starts_with("HistoriaClinica_"));
        assert!(name.ends_with(".md"));
    }
}
