use rusqlite::{Connection, Result};
use std::fs;
use std::path::PathBuf;

use crate::db::migrations::run_migrations;

/// Inicializa la base de datos local: abre SQLite, activa PRAGMAs y
/// aplica las migraciones pendientes (`db::migrations`).
pub fn init_db(app_dir: PathBuf) -> Result<Connection, String> {
    // Asegurar que la carpeta de la app exista en el sistema de archivos del usuario
    fs::create_dir_all(&app_dir)
        .map_err(|e| format!("No se pudo crear el directorio de datos: {}", e))?;

    // Definir la ruta del archivo SQLite
    let db_path = app_dir.join("historia_clinica.db");
    println!("Conectando a la base de datos local en: {:?}", db_path);

    let mut conn = Connection::open(db_path).map_err(|e| format!("Error al abrir SQLite: {}", e))?;

    // Habilitar Llaves Foráneas y Modo WAL (Concurrencia segura para múltiples médicos)
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;",
    )
    .map_err(|e| format!("Error al activar PRAGMAs: {}", e))?;

    // Migraciones (schema_version via PRAGMA user_version)
    run_migrations(&mut conn)?;

    println!("¡Base de datos local verificada y al día!");
    Ok(conn)
}

// =======================================================================
// COMPATIBILIDAD HACIA ATRÁS (fixture congelado)
// =======================================================================
//
// Los tests de migraciones abren bases que crea el MISMO build: prueban que
// el schema actual es coherente consigo mismo, no que una base vieja siga
// funcionando. Este fixture es una DB real de la versión 0.1.2 congelada en
// el repo: si una migración futura rompe datos existentes, el test falla en
// CI en vez de fallar en la máquina de un cliente.
//
// Regenerarlo SOLO a mano y commitear el resultado (ver docs/release.md).
#[cfg(test)]
mod compat_tests {
    use super::*;
    use crate::db::migrations::MIGRATIONS;

    fn user_version(conn: &Connection) -> u32 {
        conn.query_row("PRAGMA user_version", [], |r| r.get(0))
            .expect("user_version")
    }

    /// Base congelada de 0.1.2 (user_version = 1).
    const GOLDEN_DB: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/db-v0.1.2.db"
    ));

    fn golden_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("historia_clinica.db"), GOLDEN_DB)
            .expect("escribir fixture");
        dir
    }

    #[test]
    fn una_base_de_0_1_2_abre_y_migra_al_dia() {
        let dir = golden_dir();
        let conn = init_db(dir.path().to_path_buf()).expect("init_db sobre fixture");

        assert_eq!(
            user_version(&conn),
            MIGRATIONS.len() as u32,
            "la base vieja debe quedar en la versión de schema actual"
        );

        // Los datos previos sobrevivieron la migración.
        let (username, role): (String, String) = conn
            .query_row(
                "SELECT username, role FROM users WHERE id = 'admin.fixture'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .expect("el usuario congelado debe existir");
        assert_eq!(username, "admin.fixture");
        assert_eq!(role, "administrador");

        let users: i64 = conn
            .query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))
            .unwrap();
        let entities: i64 = conn
            .query_row("SELECT COUNT(*) FROM entities", [], |r| r.get(0))
            .unwrap();
        let entries: i64 = conn
            .query_row("SELECT COUNT(*) FROM entries", [], |r| r.get(0))
            .unwrap();
        assert_eq!((users, entities, entries), (2, 1, 2));

        // El perfil profesional y su llave pública siguen legibles (base de
        // la verificación de firmas de entries viejas).
        let (profile_user, public_key): (String, String) = conn
            .query_row(
                "SELECT user_id, public_key FROM professional_profiles",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .expect("perfil congelado");
        assert_eq!(profile_user, "dra.fixture");
        assert!(!public_key.is_empty());

        let keys: i64 = conn
            .query_row("SELECT COUNT(*) FROM signing_keys", [], |r| r.get(0))
            .unwrap();
        assert_eq!(keys, 1, "el histórico de claves debe sobrevivir");
    }

    #[test]
    fn el_fixture_esta_en_la_version_que_declara() {
        // Guarda contra un fixture regenerado por error con un build nuevo:
        // si esto falla, el fixture ya no representa 0.1.2.
        let dir = golden_dir();
        let conn = Connection::open(dir.path().join("historia_clinica.db")).unwrap();
        assert_eq!(user_version(&conn), 1);
    }

    /// Regenera el fixture a partir del schema actual. NO corre en CI.
    ///
    /// ```text
    /// cargo test regenerar_fixture -- --ignored
    /// ```
    ///
    /// Después: `git add src-tauri/tests/fixtures/db-v0.1.2.db`. Solo tiene
    /// sentido cuando el build actual es el de la versión que se quiere
    /// congelar (antes de agregar la próxima migración).
    #[test]
    #[ignore = "regenera el fixture de compatibilidad; correr a mano y commitear"]
    fn regenerar_fixture() {
        let dir = tempfile::tempdir().expect("tempdir");
        let conn = init_db(dir.path().to_path_buf()).expect("init_db");

        // Datos sintéticos deterministas: no hay datos de pacientes reales.
        conn.execute_batch(
            "INSERT INTO users (id, username, password_hash, role, created_at)
             VALUES ('admin.fixture', 'admin.fixture', 'hash-fixture', 'administrador',
                     '2026-10-31T12:00:00+00:00');
             INSERT INTO users (id, username, password_hash, role, created_at)
             VALUES ('dra.fixture', 'dra.fixture', 'hash-fixture', 'profesional',
                     '2026-10-31T12:00:00+00:00');
             INSERT INTO professional_profiles
                 (user_id, full_name_ciphertext, full_name_nonce,
                  specialty_ciphertext, specialty_nonce, public_key, updated_at)
             VALUES ('dra.fixture', '00', '00', '00', '00', 'aabbccdd', '2026-10-31T12:00:00+00:00');
             INSERT INTO signing_keys (user_id, public_key, valid_from)
             VALUES ('dra.fixture', 'aabbccdd', '2026-10-31T12:00:00+00:00');
             INSERT INTO entities (external_id, entity_type, created_by_user_id, enc_data_blob, created_at)
             VALUES ('fixture-entity', 'patient', 'dra.fixture', x'00', '2026-10-31T12:00:00+00:00');
             INSERT INTO entries (id, category, subject_id, author_id, title, status, timestamp, payload, signature, created_at)
             VALUES ('fixture-entry-1', 'SOAP_NOTE', 1, 'dra.fixture', 'Nota congelada', 'ACTIVE',
                     '2026-10-31T12:00:00+00:00', x'00', 'ff', '2026-10-31T12:00:00+00:00');
             INSERT INTO entries (id, category, subject_id, author_id, title, status, timestamp, payload, signature, created_at)
             VALUES ('fixture-entry-2', 'ALLERGY', 1, 'dra.fixture', 'Alergia congelada', 'ACTIVE',
                     '2026-10-31T12:00:00+00:00', x'00', 'ff', '2026-10-31T12:00:00+00:00');
             INSERT INTO audit_log (actor_user_id, action, detail, created_at)
             VALUES ('admin.fixture', 'user_create', 'fixture', '2026-10-31T12:00:00+00:00');",
        )
        .expect("datos del fixture");

        // WAL → un solo archivo autocontenido antes de copiar.
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode = DELETE;")
            .expect("checkpoint");
        drop(conn);

        let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/db-v0.1.2.db");
        std::fs::create_dir_all(out.parent().unwrap()).expect("crear fixtures");
        std::fs::copy(dir.path().join("historia_clinica.db"), &out).expect("copiar fixture");
        println!("fixture regenerado en {}", out.display());
    }
}
