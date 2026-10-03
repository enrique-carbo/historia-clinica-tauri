use crate::export::render::{
    export_filename, render_markdown, ExportEntry, ExportPatient, ExportProfessional, ExportSnapshot,
};
use crate::types::{DataKey, DbState};
use serde::Serialize;
use std::path::Path;
use tauri::{AppHandle, Manager, State};

use super::{get_entity, get_entries_by_subject, get_my_profile};

#[derive(Debug, Serialize)]
pub struct ExportResult {
    pub path: String,
    pub filename: String,
    pub entries_count: usize,
}

/// Arma el snapshot completo (paciente + perfil + TODAS las entries) ya
/// descifrado, listo para renderizar. Falla si la bóveda está cerrada.
#[tauri::command]
pub fn get_export_snapshot(
    subject_id: i64,
    professional_user_id: String,
    db_state: State<'_, DbState>,
    data_key_state: State<'_, DataKey>,
) -> Result<ExportSnapshot, String> {
    let entity = get_entity(subject_id, db_state.clone(), data_key_state.clone())?;

    let entries = get_entries_by_subject(
        subject_id,
        i64::MAX,
        0,
        db_state.clone(),
        data_key_state.clone(),
    )?;

    let professional = match get_my_profile(professional_user_id, db_state, data_key_state) {
        Ok(profile) => ExportProfessional {
            full_name: profile.full_name,
            license_number: profile.license_number,
            specialty: profile.specialty,
        },
        Err(e) => {
            eprintln!("[Export] Perfil profesional no disponible: {}", e);
            ExportProfessional {
                full_name: String::new(),
                license_number: String::new(),
                specialty: String::new(),
            }
        }
    };

    Ok(ExportSnapshot {
        patient: ExportPatient {
            data: entity.data,
            created_at: entity.created_at,
        },
        professional,
        entries: entries
            .into_iter()
            .map(|e| ExportEntry {
                category: e.category,
                title: e.title,
                status: e.status,
                timestamp: e.timestamp,
                author_name: e.author_name,
                payload: e.payload,
                signature: e.signature,
                is_verified: e.is_verified,
            })
            .collect(),
        generated_at: chrono::Local::now().format("%d/%m/%Y %H:%M").to_string(),
    })
}

/// Renderiza el .md y lo escribe en <app_local_data_dir>/exports/.
/// Nombre único con timestamp: sin diálogos ni conflictos de sobrescritura.
#[tauri::command]
pub fn export_history(
    app: AppHandle,
    snapshot: ExportSnapshot,
) -> Result<ExportResult, String> {
    let base_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|_| "No se pudo determinar la ruta de almacenamiento".to_string())?;

    write_export(&base_dir.join("exports"), &snapshot)
}

fn write_export(dir: &Path, snapshot: &ExportSnapshot) -> Result<ExportResult, String> {
    if snapshot.entries.is_empty() {
        return Err("El paciente no tiene entries para exportar.".to_string());
    }

    let markdown = render_markdown(snapshot);
    let filename = export_filename(snapshot);

    std::fs::create_dir_all(dir)
        .map_err(|e| format!("Error creando carpeta de exports: {}", e))?;

    let mut path = dir.join(&filename);
    if path.exists() {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("export").to_string();
        for i in 1..100 {
            let candidate = dir.join(format!("{} ({}).md", stem, i));
            if !candidate.exists() {
                path = candidate;
                break;
            }
        }
    }

    std::fs::write(&path, &markdown)
        .map_err(|e| format!("Error escribiendo el archivo: {}", e))?;

    println!("✅ [Export] Historia clínica exportada: {:?}", path);

    Ok(ExportResult {
        path: path.to_string_lossy().into_owned(),
        filename: path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&filename)
            .to_string(),
        entries_count: snapshot.entries.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::crypto;
    use crate::db::database::init_db;
    use crate::types::SigningState;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use tauri::Manager;

    const DATA_KEY: [u8; 32] = [7u8; 32];

    struct TestEnv {
        app: tauri::App<tauri::test::MockRuntime>,
        author_id: String,
        entity_id: i64,
        _dir: tempfile::TempDir,
    }

    fn setup(data_key_present: bool) -> TestEnv {
        let dir = tempfile::tempdir().expect("tempdir");
        let conn = init_db(dir.path().to_path_buf()).expect("init_db");

        let (signing_key, verifying_key) = crypto::generate_keypair();
        let private_bytes = signing_key.to_bytes();
        let public_hex = hex::encode(verifying_key.to_bytes());

        let author_id = "author-export-1".to_string();
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO users (id, username, password_hash, role, created_at)
             VALUES (?1, ?2, 'hash_test', 'medico', ?3)",
            rusqlite::params![author_id, "dra.export", now],
        )
        .expect("insert user");

        let name_enc = crypto::encrypt_text("Dra. Export Test", &DATA_KEY).expect("enc");
        let lic_enc = crypto::encrypt_text("MP-99999", &DATA_KEY).expect("enc");
        let spec_enc = crypto::encrypt_text("Pediatría", &DATA_KEY).expect("enc");
        conn.execute(
            "INSERT INTO professional_profiles
                (user_id, full_name_ciphertext, full_name_nonce,
                 license_number_ciphertext, license_number_nonce,
                 specialty_ciphertext, specialty_nonce,
                 public_key, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                author_id,
                name_enc.ciphertext,
                name_enc.nonce,
                lic_enc.ciphertext,
                lic_enc.nonce,
                spec_enc.ciphertext,
                spec_enc.nonce,
                public_hex,
                now
            ],
        )
        .expect("insert profile");

        let mut patient_fields: HashMap<String, String> = HashMap::new();
        for (field, value) in [
            ("nombre", "Juan"),
            ("apellido", "Pérez"),
            ("dni", "30111222"),
            ("fecha_nacimiento", "1990-03-15"),
        ] {
            let enc = crypto::encrypt_text(value, &DATA_KEY).expect("enc");
            patient_fields.insert(field.to_string(), format!("{}:{}", enc.ciphertext, enc.nonce));
        }
        let blob = serde_json::to_vec(&patient_fields).expect("blob");
        conn.execute(
            "INSERT INTO entities (entity_type, created_by_user_id, enc_data_blob, created_at)
             VALUES ('paciente', ?1, ?2, ?3)",
            rusqlite::params![author_id, blob, now],
        )
        .expect("insert entity");
        let entity_id = conn.last_insert_rowid();

        let app = tauri::test::mock_app();
        app.manage(DbState(Mutex::new(Some(conn))));
        app.manage(DataKey(Mutex::new(if data_key_present {
            Some(DATA_KEY)
        } else {
            None
        })));
        app.manage(SigningState(Mutex::new(Some(private_bytes))));

        TestEnv {
            app,
            author_id,
            entity_id,
            _dir: dir,
        }
    }

    fn snapshot_of(env: &TestEnv) -> Result<ExportSnapshot, String> {
        get_export_snapshot(
            env.entity_id,
            env.author_id.clone(),
            env.app.state::<DbState>(),
            env.app.state::<DataKey>(),
        )
    }

    #[test]
    fn snapshot_decrypts_patient_and_profile() {
        let env = setup(true);
        let snapshot = snapshot_of(&env).expect("snapshot");

        assert_eq!(snapshot.patient.data.get("nombre").map(String::as_str), Some("Juan"));
        assert_eq!(
            snapshot.patient.data.get("apellido").map(String::as_str),
            Some("Pérez")
        );
        assert_eq!(snapshot.professional.full_name, "Dra. Export Test");
        assert_eq!(snapshot.professional.license_number, "MP-99999");
        assert_eq!(snapshot.professional.specialty, "Pediatría");
        assert!(!snapshot.generated_at.is_empty());
    }

    #[test]
    fn snapshot_fails_when_vault_locked() {
        let env = setup(false);
        let err = snapshot_of(&env).expect_err("debe fallar sin data key");
        assert!(err.contains("Data key"), "error claro de bóveda: {}", err);
    }

    #[test]
    fn write_export_generates_markdown_file() {
        let env = setup(true);
        let mut snapshot = snapshot_of(&env).expect("snapshot");
        snapshot.entries.push(ExportEntry {
            category: "SOAP_NOTE".to_string(),
            title: "Primera consulta".to_string(),
            status: "ACTIVE".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            author_name: "Dra. Export Test".to_string(),
            payload: {
                let mut p = HashMap::new();
                p.insert("subjetivo".to_string(), "Malestar general".to_string());
                p
            },
            signature: "a".repeat(64),
            is_verified: true,
        });

        let dir = tempfile::tempdir().expect("tempdir");
        let result = write_export(dir.path(), &snapshot).expect("write_export");

        assert!(result.filename.ends_with(".md"));
        assert!(result.filename.starts_with("HistoriaClinica_Pérez_Juan_"));
        assert_eq!(result.entries_count, 1);
        assert!(Path::new(&result.path).exists());

        let content = std::fs::read_to_string(&result.path).expect("read");
        assert!(content.contains("# Historia Clínica — Juan Pérez"));
        assert!(content.contains("Primera consulta"));
        assert!(content.contains("SHA-256"));
    }

    #[test]
    fn write_export_rejects_empty_history() {
        let env = setup(true);
        let snapshot = snapshot_of(&env).expect("snapshot");
        assert!(snapshot.entries.is_empty());

        let dir = tempfile::tempdir().expect("tempdir");
        let err = write_export(dir.path(), &snapshot).expect_err("debe rechazar 0 entries");
        assert!(err.contains("no tiene entries"));
    }
}
