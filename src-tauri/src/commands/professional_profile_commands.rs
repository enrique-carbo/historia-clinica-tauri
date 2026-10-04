// src/commands/professional_profile_commands.rs

use crate::security::crypto;
use crate::types::{DataKey, DbState, SessionState};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

use super::{get_data_key, record_audit, require_session, with_conn};

// =======================================================================
// TIPOS
// =======================================================================

/// Una matrícula profesional: número + jurisdicción (provincia/colegio).
/// El orden del array es el orden de presentación — la primera es la
/// matrícula principal.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct License {
    pub number: String,
    pub jurisdiction: String,
}

#[derive(Serialize, Debug)]
pub struct ProfessionalProfile {
    pub user_id: String,
    pub full_name: String,
    pub profession: String,
    pub specialty: String,
    pub licenses: Vec<License>,
    pub city: String,
    pub country: String,
    pub public_key: String,
    pub address: String,
    pub phone: String,
    pub email: String,
    pub website: String,
}

#[derive(Deserialize)]
pub struct UpdateProfileInput {
    pub user_id: String,
    pub full_name: String,
    pub profession: String,
    pub specialty: String,
    pub licenses: Vec<License>,
    pub city: String,
    pub country: String,
    pub address: String,
    pub phone: String,
    pub email: String,
    pub website: String,
}

// =======================================================================
// INTERNAL
// =======================================================================

const PROFILE_COLUMNS: usize = 21;

/// Carga y descifra el perfil de `user_id`. El caller (command o export)
/// es responsable de que el `user_id` salga de la sesión, no del frontend.
pub(super) fn load_profile(
    user_id: &str,
    db_state: &State<'_, DbState>,
    data_key_state: &State<'_, DataKey>,
) -> Result<ProfessionalProfile, String> {
    let data_key = get_data_key(data_key_state)?;

    // Un par ciphertext/nonce vacío = campo nunca cargado → String vacío.
    let decrypt = |ct: String, nonce: String| -> String {
        if ct.is_empty() {
            return String::new();
        }
        let enc = crypto::EncryptedData {
            ciphertext: ct,
            nonce,
        };
        crypto::decrypt_text(&enc, &data_key).unwrap_or_default()
    };

    // Todas las columnas del SELECT son TEXT NOT NULL → se leen como String
    // en orden y se desestructuran. Evita el tuple gigante de a mano.
    let cols: Vec<String> = with_conn(db_state, |conn| {
        conn.query_row(
            "SELECT full_name_ciphertext, full_name_nonce,
                    specialty_ciphertext, specialty_nonce,
                    profession_ciphertext, profession_nonce,
                    licenses_ciphertext, licenses_nonce,
                    city_ciphertext, city_nonce,
                    country_ciphertext, country_nonce,
                    public_key,
                    address_ciphertext, address_nonce,
                    phone_ciphertext, phone_nonce,
                    email_ciphertext, email_nonce,
                    website_ciphertext, website_nonce
             FROM professional_profiles
             WHERE user_id = ?1",
            params![user_id],
            |row| {
                (0..PROFILE_COLUMNS)
                    .map(|i| row.get::<_, String>(i))
                    .collect::<rusqlite::Result<Vec<String>>>()
            },
        )
        .map_err(|e| e.to_string())
    })?;

    let [
        fn_ct, fn_n, sp_ct, sp_n, pr_ct, pr_n, li_ct, li_n, ci_ct, ci_n, co_ct, co_n, pk,
        ad_ct, ad_n, ph_ct, ph_n, em_ct, em_n, we_ct, we_n,
    ]: [String; PROFILE_COLUMNS] = cols.try_into().map_err(|v: Vec<String>| {
        format!(
            "Perfil profesional corrupto: {} columnas (se esperaban {}).",
            v.len(),
            PROFILE_COLUMNS
        )
    })?;

    let licenses_json = decrypt(li_ct, li_n);
    let licenses = if licenses_json.is_empty() {
        Vec::new()
    } else {
        // Decrypt OK pero JSON inválido = corrupción real: no se esconde.
        serde_json::from_str(&licenses_json)
            .map_err(|e| format!("No se pudieron leer las matrículas del perfil: {}", e))?
    };

    Ok(ProfessionalProfile {
        user_id: user_id.to_string(),
        full_name: decrypt(fn_ct, fn_n),
        profession: decrypt(pr_ct, pr_n),
        specialty: decrypt(sp_ct, sp_n),
        licenses,
        city: decrypt(ci_ct, ci_n),
        country: decrypt(co_ct, co_n),
        public_key: pk,
        address: decrypt(ad_ct, ad_n),
        phone: decrypt(ph_ct, ph_n),
        email: decrypt(em_ct, em_n),
        website: decrypt(we_ct, we_n),
    })
}

// =======================================================================
// COMANDOS
// =======================================================================

/// Obtiene el perfil del usuario autenticado, descifrando los campos sensibles.
/// El usuario sale de la sesión — nunca de un argumento del frontend.
#[tauri::command]
pub fn get_my_profile(
    session_state: State<'_, SessionState>,
    db_state: State<'_, DbState>,
    data_key_state: State<'_, DataKey>,
) -> Result<ProfessionalProfile, String> {
    let session = require_session(&session_state)?;
    load_profile(&session.user_id, &db_state, &data_key_state)
}

/// Actualiza el perfil del usuario autenticado, cifrando los campos sensibles.
/// Rechaza cualquier intento de editar el perfil de otro usuario.
#[tauri::command]
pub fn update_my_profile(
    input: UpdateProfileInput,
    session_state: State<'_, SessionState>,
    db_state: State<'_, DbState>,
    data_key_state: State<'_, DataKey>,
) -> Result<bool, String> {
    let session = require_session(&session_state)?;
    if input.user_id != session.user_id {
        return Err("No podés editar el perfil de otro usuario.".to_string());
    }

    let data_key = get_data_key(&data_key_state)?;

    let encrypt = |text: &str| -> Result<(String, String), String> {
        let enc = crypto::encrypt_text(text, &data_key)?;
        Ok((enc.ciphertext, enc.nonce))
    };

    // Filas sin número no son matrículas: no se persisten.
    let licenses: Vec<License> = input
        .licenses
        .iter()
        .filter(|l| !l.number.trim().is_empty())
        .cloned()
        .collect();
    let licenses_json = serde_json::to_string(&licenses)
        .map_err(|e| format!("No se pudieron serializar las matrículas: {}", e))?;

    let (fn_ct, fn_n) = encrypt(&input.full_name)?;
    let (sp_ct, sp_n) = encrypt(&input.specialty)?;
    let (pr_ct, pr_n) = encrypt(&input.profession)?;
    let (li_ct, li_n) = encrypt(&licenses_json)?;
    let (ci_ct, ci_n) = encrypt(&input.city)?;
    let (co_ct, co_n) = encrypt(&input.country)?;
    let (ad_ct, ad_n) = encrypt(&input.address)?;
    let (ph_ct, ph_n) = encrypt(&input.phone)?;
    let (em_ct, em_n) = encrypt(&input.email)?;
    let (we_ct, we_n) = encrypt(&input.website)?;

    // La auditoría registra QUÉ campos cambiaron, nunca su valor:
    // el perfil está cifrado en reposo y audit_log es texto plano.
    let changed: Vec<&str> = [
        ("full_name", !input.full_name.trim().is_empty()),
        ("profession", !input.profession.trim().is_empty()),
        ("specialty", !input.specialty.trim().is_empty()),
        ("licenses", !licenses.is_empty()),
        ("city", !input.city.trim().is_empty()),
        ("country", !input.country.trim().is_empty()),
        ("address", !input.address.trim().is_empty()),
        ("phone", !input.phone.trim().is_empty()),
        ("email", !input.email.trim().is_empty()),
        ("website", !input.website.trim().is_empty()),
    ]
    .iter()
    .filter(|(_, set)| *set)
    .map(|(k, _)| *k)
    .collect();

    with_conn(&db_state, |conn| {
        conn.execute(
            "UPDATE professional_profiles
             SET full_name_ciphertext = ?1, full_name_nonce = ?2,
                 specialty_ciphertext = ?3, specialty_nonce = ?4,
                 profession_ciphertext = ?5, profession_nonce = ?6,
                 licenses_ciphertext = ?7, licenses_nonce = ?8,
                 city_ciphertext = ?9, city_nonce = ?10,
                 country_ciphertext = ?11, country_nonce = ?12,
                 address_ciphertext = ?13, address_nonce = ?14,
                 phone_ciphertext = ?15, phone_nonce = ?16,
                 email_ciphertext = ?17, email_nonce = ?18,
                 website_ciphertext = ?19, website_nonce = ?20,
                 updated_at = datetime('now')
             WHERE user_id = ?21",
            params![
                fn_ct,
                fn_n,
                sp_ct,
                sp_n,
                pr_ct,
                pr_n,
                li_ct,
                li_n,
                ci_ct,
                ci_n,
                co_ct,
                co_n,
                ad_ct,
                ad_n,
                ph_ct,
                ph_n,
                em_ct,
                em_n,
                we_ct,
                we_n,
                session.user_id,
            ],
        )
        .map_err(|e| e.to_string())?;

        record_audit(
            conn,
            &session.user_id,
            "profile_update",
            None,
            &format!(
                "campos: {} | matriculas: {}",
                if changed.is_empty() {
                    "(ninguno)".to_string()
                } else {
                    changed.join(", ")
                },
                licenses.len()
            ),
        )?;

        Ok(())
    })?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::database::init_db;
    use crate::types::SessionUser;
    use std::sync::Mutex;
    use tauri::Manager;
    use zeroize::Zeroizing;

    const DATA_KEY: [u8; 32] = [9u8; 32];

    struct TestEnv {
        app: tauri::App<tauri::test::MockRuntime>,
        _dir: tempfile::TempDir,
    }

    fn session_of(user_id: &str) -> SessionUser {
        SessionUser {
            user_id: user_id.to_string(),
            username: "dra.test".to_string(),
            role: "medico".to_string(),
        }
    }

    fn setup(session: Option<SessionUser>) -> TestEnv {
        let dir = tempfile::tempdir().expect("tempdir");
        let conn = init_db(dir.path().to_path_buf()).expect("init_db");
        let now = chrono::Utc::now().to_rfc3339();

        for uid in ["user-a", "user-b"] {
            conn.execute(
                "INSERT INTO users (id, username, password_hash, role, created_at)
                 VALUES (?1, ?2, 'hash_test', 'medico', ?3)",
                params![uid, uid, now],
            )
            .expect("insert user");
            conn.execute(
                "INSERT INTO professional_profiles
                    (user_id, full_name_ciphertext, full_name_nonce,
                     specialty_ciphertext, specialty_nonce,
                     public_key, updated_at)
                 VALUES (?1, '', '', '', '', 'pk-test', ?2)",
                params![uid, now],
            )
            .expect("insert profile");
        }

        let app = tauri::test::mock_app();
        app.manage(DbState(Mutex::new(Some(conn))));
        app.manage(DataKey(Mutex::new(Some(Zeroizing::new(DATA_KEY)))));
        app.manage(SessionState(Mutex::new(session)));

        TestEnv { app, _dir: dir }
    }

    fn sample_input(user_id: &str) -> UpdateProfileInput {
        UpdateProfileInput {
            user_id: user_id.to_string(),
            full_name: "Dra. Ana Pérez".to_string(),
            profession: "Médica".to_string(),
            specialty: "Cardiología".to_string(),
            licenses: vec![
                License {
                    number: "MP-11111".to_string(),
                    jurisdiction: "Córdoba".to_string(),
                },
                License {
                    number: "  ".to_string(),
                    jurisdiction: "CABA".to_string(),
                },
                License {
                    number: "MN-22222".to_string(),
                    jurisdiction: "CABA".to_string(),
                },
            ],
            city: "Córdoba".to_string(),
            country: "Argentina".to_string(),
            address: "Av. Siempreviva 742".to_string(),
            phone: "351-555-0000".to_string(),
            email: "ana@example.com".to_string(),
            website: "https://example.com".to_string(),
        }
    }

    fn update(env: &TestEnv, input: UpdateProfileInput) -> Result<bool, String> {
        update_my_profile(
            input,
            env.app.state::<SessionState>(),
            env.app.state::<DbState>(),
            env.app.state::<DataKey>(),
        )
    }

    fn get(env: &TestEnv) -> Result<ProfessionalProfile, String> {
        get_my_profile(
            env.app.state::<SessionState>(),
            env.app.state::<DbState>(),
            env.app.state::<DataKey>(),
        )
    }

    #[test]
    fn update_and_get_roundtrips_profile_with_multiple_licenses() {
        let env = setup(Some(session_of("user-a")));

        update(&env, sample_input("user-a")).expect("update");

        let profile = get(&env).expect("get");
        assert_eq!(profile.user_id, "user-a");
        assert_eq!(profile.full_name, "Dra. Ana Pérez");
        assert_eq!(profile.profession, "Médica");
        assert_eq!(profile.specialty, "Cardiología");
        assert_eq!(profile.city, "Córdoba");
        assert_eq!(profile.country, "Argentina");
        assert_eq!(profile.address, "Av. Siempreviva 742");
        assert_eq!(profile.phone, "351-555-0000");
        assert_eq!(profile.email, "ana@example.com");
        assert_eq!(profile.website, "https://example.com");
        // La matrícula sin número se descarta; el orden se conserva.
        assert_eq!(
            profile.licenses,
            vec![
                License {
                    number: "MP-11111".to_string(),
                    jurisdiction: "Córdoba".to_string(),
                },
                License {
                    number: "MN-22222".to_string(),
                    jurisdiction: "CABA".to_string(),
                },
            ]
        );

        let db = env.app.state::<DbState>();
        let guard = db.0.lock().unwrap();
        let conn = guard.as_ref().unwrap();
        let (action, detail): (String, String) = conn
            .query_row(
                "SELECT action, detail FROM audit_log ORDER BY id DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .expect("audit row");
        assert_eq!(action, "profile_update");
        assert!(detail.contains("full_name"), "detalle: {}", detail);
        assert!(detail.contains("matriculas: 2"), "detalle: {}", detail);
        assert!(
            !detail.contains("Dra. Ana Pérez"),
            "la auditoría no debe volcar valores del perfil: {}",
            detail
        );
    }

    #[test]
    fn update_rejects_foreign_user_id() {
        let env = setup(Some(session_of("user-a")));

        let err = update(&env, sample_input("user-b")).expect_err("debe rechazar");
        assert!(err.contains("otro usuario"), "error: {}", err);

        // El perfil de user-b no fue tocado.
        let db = env.app.state::<DbState>();
        let guard = db.0.lock().unwrap();
        let conn = guard.as_ref().unwrap();
        let full_name_ct: String = conn
            .query_row(
                "SELECT full_name_ciphertext FROM professional_profiles WHERE user_id = 'user-b'",
                [],
                |r| r.get(0),
            )
            .expect("row");
        assert!(full_name_ct.is_empty(), "el perfil ajeno debe seguir vacío");
    }

    #[test]
    fn profile_commands_require_active_session() {
        let env = setup(None);

        let err = get(&env).expect_err("get sin sesión debe fallar");
        assert!(err.contains("Sesión no iniciada"), "error: {}", err);

        let err = update(&env, sample_input("user-a"))
            .expect_err("update sin sesión debe fallar");
        assert!(err.contains("Sesión no iniciada"), "error: {}", err);
    }

    #[test]
    fn get_returns_empty_profile_when_never_edited() {
        let env = setup(Some(session_of("user-a")));

        let profile = get(&env).expect("get");
        assert_eq!(profile.full_name, "");
        assert!(profile.licenses.is_empty());
        assert_eq!(profile.public_key, "pk-test");
    }
}
