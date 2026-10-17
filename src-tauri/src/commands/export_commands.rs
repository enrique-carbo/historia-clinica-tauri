use crate::db::config_schema::SchemaConfig;
use crate::export::render::{
    append_signature_section, export_filename, render_markdown, ExportEntry, ExportPatient,
    ExportProfessional, ExportSnapshot,
};
use crate::security::crypto;
use crate::types::{DataKey, DbState, SessionState, SigningState};
use serde::Serialize;
use std::path::Path;
use tauri::{AppHandle, Manager, State};

use super::{
    get_entity, get_entries_by_subject, professional_profile_commands::load_profile,
    record_audit, require_session,
};

#[derive(Debug, Serialize)]
pub struct ExportResult {
    pub path: String,
    pub filename: String,
    pub entries_count: usize,
    /// SHA-256 del contenido firmado (sin el bloque de firma) — viaja al
    /// frontend para el banner y se registra en el audit_log.
    pub sha256: String,
}

/// Arma el snapshot completo (paciente + perfil + TODAS las entries) ya
/// descifrado, listo para renderizar. Solo corre en Rust — el snapshot
/// **nunca cruza el IPC**. Falla si la bóveda está cerrada.
/// El perfil profesional es el de la sesión activa — no se acepta un
/// `user_id` del frontend.
fn build_export_snapshot(
    subject_id: i64,
    session_state: State<'_, SessionState>,
    db_state: State<'_, DbState>,
    data_key_state: State<'_, DataKey>,
) -> Result<ExportSnapshot, String> {
    let session = require_session(&session_state)?;
    let entity = get_entity(subject_id, db_state.clone(), data_key_state.clone())?;

    let entries = get_entries_by_subject(
        subject_id,
        i64::MAX,
        0,
        db_state.clone(),
        data_key_state.clone(),
    )?;

    let professional = match load_profile(&session.user_id, &db_state, &data_key_state) {
        Ok(profile) => ExportProfessional {
            full_name: profile.full_name,
            profession: profile.profession,
            specialty: profile.specialty,
            licenses: profile.licenses,
            city: profile.city,
            country: profile.country,
        },
        Err(e) => {
            eprintln!("[Export] Perfil profesional no disponible: {}", e);
            ExportProfessional::default()
        }
    };

    Ok(ExportSnapshot {
        patient: ExportPatient {
            data: entity.data,
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
                author_profession: e.author_profession,
                payload: e.payload,
                signature: e.signature,
                is_verified: e.is_verified,
            })
            .collect(),
        generated_at: chrono::Local::now().format("%d/%m/%Y %H:%M").to_string(),
    })
}

/// Descifra y escribe el `.md` en <app_local_data_dir>/exports/ en un solo
/// command: el frontend manda únicamente el `subject_id` y recibe
/// `{ path, filename, entries_count }` — los datos descifrados no viajan
/// por el IPC. Nombre único con timestamp: sin diálogos ni conflictos.
#[tauri::command]
pub fn export_history<R: tauri::Runtime>(
    app: AppHandle<R>,
    subject_id: i64,
    session_state: State<'_, SessionState>,
    db_state: State<'_, DbState>,
    data_key_state: State<'_, DataKey>,
    schema_state: State<'_, SchemaConfig>,
    signing_state: State<'_, SigningState>,
) -> Result<ExportResult, String> {
    let session = require_session(&session_state)?;
    let snapshot =
        build_export_snapshot(subject_id, session_state, db_state.clone(), data_key_state)?;

    let sign_guard = signing_state.0.lock().map_err(|_| "Lock poisoned".to_string())?;
    let private_key_bytes = sign_guard
        .as_ref()
        .ok_or("Error de Seguridad: No hay llave de firma en RAM. ¿Bóveda cerrada?")?;

    let base_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|_| "No se pudo determinar la ruta de almacenamiento".to_string())?;

    let result = write_export(
        &base_dir.join("exports"),
        &snapshot,
        &schema_state,
        private_key_bytes,
    )?;

    // Auditoría solo en éxito: metadata pura (ids, hash) — nunca valores
    // ni identificables: el filename lleva apellido+nombre del paciente en
    // claro y está prohibido en audit_log; la correlación con el archivo
    // se hace por sha256 (contenido sin el bloque de firma).
    {
        let guard = db_state.0.lock().map_err(|_| "Lock poisoned".to_string())?;
        let conn = guard
            .as_ref()
            .ok_or("Base de datos no inicializada".to_string())?;
        record_audit(
            conn,
            &session.user_id,
            "export_emitted",
            None,
            &format!(
                "subject_id={} entries={} sha256={}",
                subject_id, result.entries_count, result.sha256
            ),
        )?;
    }

    Ok(result)
}

fn write_export(
    dir: &Path,
    snapshot: &ExportSnapshot,
    schema: &SchemaConfig,
    private_key_bytes: &[u8; 32],
) -> Result<ExportResult, String> {
    if snapshot.entries.is_empty() {
        return Err("El paciente no tiene entries para exportar.".to_string());
    }

    let markdown = render_markdown(snapshot, schema)?;
    // Firma del documento completo (hasta el footer de hash) con la clave
    // Ed25519 del profesional de la sesión.
    let doc_hash = crypto::hash_document(&markdown);
    let signature_hex = crypto::sign_hash(&doc_hash, private_key_bytes)?;
    let public_hex = crypto::public_key_from_private(private_key_bytes);
    let final_doc = append_signature_section(&markdown, &signature_hex, &public_hex);
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

    std::fs::write(&path, &final_doc)
        .map_err(|e| format!("Error escribiendo el archivo: {}", e))?;

    println!("✅ [Export] Historia clínica exportada (firmada): {:?}", path);

    Ok(ExportResult {
        path: path.to_string_lossy().into_owned(),
        filename: path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&filename)
            .to_string(),
        entries_count: snapshot.entries.len(),
        sha256: hex::encode(doc_hash),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::crypto;
    use crate::db::config_schema::from_embedded;
    use crate::db::database::init_db;
    use crate::types::{SessionState, SessionUser, SigningState};
    use super::super::professional_profile_commands::License;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use tauri::Manager;
    use zeroize::Zeroizing;

    const DATA_KEY: [u8; 32] = [7u8; 32];

    struct TestEnv {
        app: tauri::App<tauri::test::MockRuntime>,
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
             VALUES (?1, ?2, 'hash_test', 'profesional', ?3)",
            rusqlite::params![author_id, "dra.export", now],
        )
        .expect("insert user");

        let name_enc = crypto::encrypt_text("Dra. Export Test", &DATA_KEY).expect("enc");
        let spec_enc = crypto::encrypt_text("Pediatría", &DATA_KEY).expect("enc");
        let prof_enc = crypto::encrypt_text("Médica", &DATA_KEY).expect("enc");
        let city_enc = crypto::encrypt_text("Córdoba", &DATA_KEY).expect("enc");
        let country_enc = crypto::encrypt_text("Argentina", &DATA_KEY).expect("enc");
        let licenses_json = serde_json::to_string(&[License {
            number: "MP-99999".to_string(),
            jurisdiction: "Córdoba".to_string(),
        }])
        .expect("licenses json");
        let lic_enc = crypto::encrypt_text(&licenses_json, &DATA_KEY).expect("enc");
        conn.execute(
            "INSERT INTO professional_profiles
                (user_id, full_name_ciphertext, full_name_nonce,
                 specialty_ciphertext, specialty_nonce,
                 profession_ciphertext, profession_nonce,
                 licenses_ciphertext, licenses_nonce,
                 city_ciphertext, city_nonce,
                 country_ciphertext, country_nonce,
                 public_key, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            rusqlite::params![
                author_id,
                name_enc.ciphertext,
                name_enc.nonce,
                spec_enc.ciphertext,
                spec_enc.nonce,
                prof_enc.ciphertext,
                prof_enc.nonce,
                lic_enc.ciphertext,
                lic_enc.nonce,
                city_enc.ciphertext,
                city_enc.nonce,
                country_enc.ciphertext,
                country_enc.nonce,
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
            Some(Zeroizing::new(DATA_KEY))
        } else {
            None
        })));
        app.manage(SigningState(Mutex::new(Some(Zeroizing::new(private_bytes)))));
        app.manage(SessionState(Mutex::new(Some(SessionUser {
            user_id: author_id.clone(),
            username: "dra.export".to_string(),
            role: "profesional".to_string(),
        }))));
        app.manage(from_embedded().expect("entity_schema.json embebido válido"));

        TestEnv {
            app,
            entity_id,
            _dir: dir,
        }
    }

    fn snapshot_of(env: &TestEnv) -> Result<ExportSnapshot, String> {
        build_export_snapshot(
            env.entity_id,
            env.app.state::<SessionState>(),
            env.app.state::<DbState>(),
            env.app.state::<DataKey>(),
        )
    }

    fn signing_key_of(env: &TestEnv) -> Zeroizing<[u8; 32]> {
        env.app
            .state::<SigningState>()
            .0
            .lock()
            .unwrap()
            .as_ref()
            .expect("signing key en el env de test")
            .clone()
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
        assert_eq!(snapshot.professional.profession, "Médica");
        assert_eq!(snapshot.professional.specialty, "Pediatría");
        assert_eq!(
            snapshot.professional.licenses,
            vec![License {
                number: "MP-99999".to_string(),
                jurisdiction: "Córdoba".to_string(),
            }]
        );
        assert_eq!(snapshot.professional.city, "Córdoba");
        assert_eq!(snapshot.professional.country, "Argentina");
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
        let key = signing_key_of(&env);
        let mut snapshot = snapshot_of(&env).expect("snapshot");
        snapshot.entries.push(ExportEntry {
            category: "SOAP_NOTE".to_string(),
            title: "Primera consulta".to_string(),
            status: "ACTIVE".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            author_name: "Dra. Export Test".to_string(),
            author_profession: "Médica".to_string(),
            payload: {
                let mut p = HashMap::new();
                p.insert("subjetivo".to_string(), "Malestar general".to_string());
                p
            },
            signature: "a".repeat(64),
            is_verified: true,
        });

        let dir = tempfile::tempdir().expect("tempdir");
        let result = write_export(dir.path(), &snapshot, &from_embedded().expect("schema"), &key)
            .expect("write_export");

        assert!(result.filename.ends_with(".md"));
        assert!(result.filename.starts_with("HistoriaClinica_Pérez_Juan_"));
        assert_eq!(result.entries_count, 1);
        assert!(!result.sha256.is_empty());
        assert!(Path::new(&result.path).exists());

        let content = std::fs::read_to_string(&result.path).expect("read");
        assert!(content.contains("# Historia Clínica — Juan Pérez"));
        assert!(content.contains("Primera consulta"));
        assert!(content.contains("SHA-256"));
        assert!(content.contains("## Firma digital (Ed25519)"));
    }

    #[test]
    fn write_export_signature_verifies_roundtrip() {
        // El bloque de firma cubre el documento completo hasta el footer de
        // hash: quitar el bloque y verificar Ed25519 con la pública del env.
        let env = setup(true);
        let key = signing_key_of(&env);
        let public_hex = crypto::public_key_from_private(&key);
        let mut snapshot = snapshot_of(&env).expect("snapshot");
        snapshot.entries.push(ExportEntry {
            category: "SOAP_NOTE".to_string(),
            title: "Nota firmada".to_string(),
            status: "ACTIVE".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            author_name: "Dra. Export Test".to_string(),
            author_profession: "Médica".to_string(),
            payload: HashMap::from([("subjetivo".to_string(), "ok".to_string())]),
            signature: "b".repeat(64),
            is_verified: true,
        });

        let dir = tempfile::tempdir().expect("tempdir");
        let result = write_export(dir.path(), &snapshot, &from_embedded().expect("schema"), &key)
            .expect("write_export");
        let content = std::fs::read_to_string(&result.path).expect("read");

        let marker = "## Firma digital (Ed25519)";
        let idx = content.find(marker).expect("bloque de firma presente");
        let signed_portion = content[..idx]
            .strip_suffix("\n---\n\n")
            .expect("separador antes del bloque");

        let sig_start = content.find("**Firma:** `").expect("línea de firma")
            + "**Firma:** `".len();
        let sig_end = content[sig_start..].find('`').expect("cierre de firma") + sig_start;
        let signature_hex = &content[sig_start..sig_end];

        let hash = crypto::hash_document(signed_portion);
        let ok = crypto::verify_signature(&hash, signature_hex, &public_hex)
            .expect("verificación");
        assert!(ok, "firma del documento debe verificar");

        // Alterar una letra del contenido firmado invalida la firma
        // (verify_signature devuelve Err en matemática inválida).
        let tampered = signed_portion.replace("Nota firmada", "Xota firmada");
        let hash2 = crypto::hash_document(&tampered);
        let ok2 = crypto::verify_signature(&hash2, signature_hex, &public_hex);
        assert!(ok2.is_err() || ok2 == Ok(false), "contenido alterado no debe verificar");
    }

    #[test]
    fn write_export_rejects_empty_history() {
        let env = setup(true);
        let key = signing_key_of(&env);
        let snapshot = snapshot_of(&env).expect("snapshot");
        assert!(snapshot.entries.is_empty());

        let dir = tempfile::tempdir().expect("tempdir");
        let err = write_export(dir.path(), &snapshot, &from_embedded().expect("schema"), &key)
            .expect_err("debe rechazar 0 entries");
        assert!(err.contains("no tiene entries"));
    }

    #[test]
    fn export_history_emits_audit_and_signed_file() {
        // Flujo completo del command con 1 entry real en la DB: archivo
        // firmado + fila export_emitted en audit_log (solo metadata).
        let env = setup(true);
        {
            let db_state = env.app.state::<DbState>();
            let guard = db_state.0.lock().unwrap();
            let conn = guard.as_ref().expect("conn");
            let payload_field =
                crypto::encrypt_text("Dolor leve", &DATA_KEY).expect("enc");
            let payload_json = serde_json::json!({
                "subjetivo": format!("{}:{}", payload_field.ciphertext, payload_field.nonce)
            })
            .to_string()
            .into_bytes();
            conn.execute(
                "INSERT INTO entries (id, category, subject_id, author_id, title, status, timestamp, payload, signature, created_at)
                 VALUES (?1, 'SOAP_NOTE', ?2, ?3, 'Consulta', 'ACTIVE', ?4, ?5, ?6, ?4)",
                rusqlite::params![
                    "entry-audit-1",
                    env.entity_id,
                    "author-export-1",
                    chrono::Utc::now().to_rfc3339(),
                    payload_json,
                    "c".repeat(64),
                ],
            )
            .expect("insert entry");
        }

        let result = export_history(
            env.app.handle().clone(),
            env.entity_id,
            env.app.state::<SessionState>(),
            env.app.state::<DbState>(),
            env.app.state::<DataKey>(),
            env.app.state::<SchemaConfig>(),
            env.app.state::<SigningState>(),
        )
        .expect("export_history");

        assert_eq!(result.entries_count, 1);
        assert!(!result.sha256.is_empty());

        let content = std::fs::read_to_string(&result.path).expect("read");
        assert!(content.contains("## Firma digital (Ed25519)"));
        assert!(content.contains("SHA-256 del documento"));
        // Limpieza: el mock runtime resuelve app_local_data_dir al home real.
        let _ = std::fs::remove_file(&result.path);

        let db_state = env.app.state::<DbState>();
        let guard = db_state.0.lock().unwrap();
        let conn = guard.as_ref().expect("conn");
        let detail: String = conn
            .query_row(
                "SELECT detail FROM audit_log WHERE action = 'export_emitted';",
                [],
                |r| r.get(0),
            )
            .expect("fila export_emitted");
        assert!(detail.contains(&format!("subject_id={}", env.entity_id)));
        assert!(detail.contains("entries=1"));
        assert!(detail.contains(&format!("sha256={}", result.sha256)));
        // Invariante anti-PHI: el audit correlaciona por sha256, nunca
        // registra el filename (lleva apellido+nombre del paciente) ni
        // ningún identificable.
        assert!(!detail.contains("Pérez"), "audit sin nombre del paciente: {}", detail);
        assert!(!detail.contains(".md"), "audit sin filename: {}", detail);
    }

    #[test]
    fn export_history_command_builds_snapshot_and_writes() {
        // Flujo completo por el único command: subject_id → snapshot
        // interno → render → archivo. La bóveda está abierta pero la
        // historia está vacía: el command debe llegar hasta write_export
        // (sesión + schema + estados inyectados) y rechazar en la última
        // barrera.
        let env = setup(true);
        let err = export_history(
            env.app.handle().clone(),
            env.entity_id,
            env.app.state::<SessionState>(),
            env.app.state::<DbState>(),
            env.app.state::<DataKey>(),
            env.app.state::<SchemaConfig>(),
            env.app.state::<SigningState>(),
        )
        .expect_err("0 entries");
        assert!(err.contains("no tiene entries"), "err: {}", err);
    }

    #[test]
    fn export_history_command_requires_open_vault() {
        let env = setup(false);
        let err = export_history(
            env.app.handle().clone(),
            env.entity_id,
            env.app.state::<SessionState>(),
            env.app.state::<DbState>(),
            env.app.state::<DataKey>(),
            env.app.state::<SchemaConfig>(),
            env.app.state::<SigningState>(),
        )
        .expect_err("bóveda cerrada");
        assert!(err.contains("Data key"), "err: {}", err);
    }
}
