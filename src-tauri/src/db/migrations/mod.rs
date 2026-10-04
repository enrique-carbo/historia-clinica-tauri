use rusqlite::{Connection, Transaction};

/// Migración de esquema: append-only. Nunca editar una versión ya
/// aplicada — siempre agregar una nueva al final de `MIGRATIONS`.
pub struct Migration {
    pub version: u32,
    pub description: &'static str,
    pub up: fn(&Connection) -> Result<(), String>,
}

// =======================================================================
// CATÁLOGO DE MIGRACIONES
// =======================================================================
//
// Convención:
//  - Las versiones son contiguas y arrancan en 1 (user_version 0 = sin migrar).
//  - Cada migración corre en SU propia transacción; si falla, rollback total.
//  - `PRAGMA user_version` se actualiza en la misma transacción.
//  - Si la DB tiene un user_version MAYOR al último conocido, la app no
//    arranca (una versión más nueva de la app creó esa base).

pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    description: "baseline: esquema inicial (users, profiles, signing_keys, audit_log, entities, entries, sync)",
    up: v001_baseline,
}];

/// v001: esquema inicial completo (movido desde init_db sin cambios).
/// Usa IF NOT EXISTS → idempotente para DBs de desarrollo creadas antes
/// del harness (user_version 0 con tablas ya presentes).
fn v001_baseline(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        -- ============================================================
        -- TABLAS PADRE (se crean primero, son referenciadas por hijas)
        -- ============================================================

        -- 1. Usuarios (administrador, asistente, medico, enfermeria, paciente)
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            role TEXT CHECK(role IN ('administrador', 'asistente', 'medico', 'enfermeria', 'paciente')) NOT NULL,
            entity_id INTEGER,
            created_at TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0, 1)),
            last_login_at TEXT,
            FOREIGN KEY(entity_id) REFERENCES entities(id)
        );

        -- 2. Perfiles profesionales (datos + llave pública vigente de firma)
        CREATE TABLE IF NOT EXISTS professional_profiles (
            user_id TEXT PRIMARY KEY,
            full_name_ciphertext TEXT NOT NULL,
            full_name_nonce TEXT NOT NULL,
            specialty_ciphertext TEXT NOT NULL,
            specialty_nonce TEXT NOT NULL,
            profession_ciphertext TEXT NOT NULL DEFAULT '',
            profession_nonce TEXT NOT NULL DEFAULT '',
            licenses_ciphertext TEXT NOT NULL DEFAULT '',
            licenses_nonce TEXT NOT NULL DEFAULT '',
            city_ciphertext TEXT NOT NULL DEFAULT '',
            city_nonce TEXT NOT NULL DEFAULT '',
            country_ciphertext TEXT NOT NULL DEFAULT '',
            country_nonce TEXT NOT NULL DEFAULT '',
            public_key TEXT NOT NULL,
            address_ciphertext TEXT NOT NULL DEFAULT '',
            address_nonce TEXT NOT NULL DEFAULT '',
            phone_ciphertext TEXT NOT NULL DEFAULT '',
            phone_nonce TEXT NOT NULL DEFAULT '',
            email_ciphertext TEXT NOT NULL DEFAULT '',
            email_nonce TEXT NOT NULL DEFAULT '',
            website_ciphertext TEXT NOT NULL DEFAULT '',
            website_nonce TEXT NOT NULL DEFAULT '',
            updated_at TEXT NOT NULL,
            FOREIGN KEY(user_id) REFERENCES users(id)
        );

        -- 3. Histórico de llaves públicas de firma (Ed25519) por usuario.
        --    valid_to NULL = clave vigente.
        CREATE TABLE IF NOT EXISTS signing_keys (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT NOT NULL,
            public_key TEXT NOT NULL,
            valid_from TEXT NOT NULL,
            valid_to TEXT,
            FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE RESTRICT
        );

        CREATE INDEX IF NOT EXISTS idx_signing_keys_lookup
            ON signing_keys(user_id, valid_from DESC);

        -- 4. Auditoría: acciones sensibles del rol administrador
        CREATE TABLE IF NOT EXISTS audit_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            actor_user_id TEXT,
            action TEXT NOT NULL,
            target_user_id TEXT,
            detail TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY(actor_user_id) REFERENCES users(id) ON DELETE SET NULL,
            FOREIGN KEY(target_user_id) REFERENCES users(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_audit_created ON audit_log(created_at DESC);

        -- ============================================================
        -- TABLAS GENÉRICAS (Template v1)
        -- ============================================================

        -- Entidades genéricas: pacientes humanos o animales
        CREATE TABLE IF NOT EXISTS entities (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            external_id TEXT UNIQUE,
            entity_type TEXT NOT NULL,
            created_by_user_id TEXT NOT NULL,
            blind_index TEXT UNIQUE,
            enc_data_blob BLOB NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type);

        -- Claves de datos por entidad (telemedicina)
        CREATE TABLE IF NOT EXISTS entity_keys (
            entity_id INTEGER PRIMARY KEY,
            entity_data_key_ciphertext TEXT NOT NULL,
            entity_data_key_nonce TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY(entity_id) REFERENCES entities(id)
        );

        -- Cola de sincronización FIFO
        CREATE TABLE IF NOT EXISTS sync_queue (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            table_name TEXT NOT NULL,
            local_record_id INTEGER NOT NULL,
            external_record_id TEXT,
            operation TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        -- ============================================================
        -- TABLA CORE ENTITY-ENTRY (Append-Only)
        -- ============================================================

        -- Entries: hechos clínicos inmutables
        CREATE TABLE IF NOT EXISTS entries (
            id TEXT PRIMARY KEY NOT NULL,
            category TEXT NOT NULL CHECK(category IN ('SOAP_NOTE', 'MEDICATION', 'ALLERGY', 'CONDITION')),
            subject_id INTEGER NOT NULL,
            author_id TEXT NOT NULL,
            title TEXT NOT NULL,
            status TEXT NOT NULL CHECK(status IN ('ACTIVE', 'RESOLVED', 'COMPLETED')),
            timestamp TEXT NOT NULL,
            payload BLOB NOT NULL,
            signature TEXT NOT NULL,
            created_at TEXT NOT NULL,
            is_synced INTEGER NOT NULL DEFAULT 0 CHECK(is_synced IN (0, 1)),
            FOREIGN KEY(subject_id) REFERENCES entities(id) ON DELETE RESTRICT,
            FOREIGN KEY(author_id) REFERENCES users(id) ON DELETE RESTRICT
        );

        CREATE INDEX IF NOT EXISTS idx_entries_subject ON entries(subject_id, timestamp DESC);
        CREATE INDEX IF NOT EXISTS idx_entries_category ON entries(category);
        CREATE INDEX IF NOT EXISTS idx_entries_sync ON entries(is_synced) WHERE is_synced = 0;
        CREATE INDEX IF NOT EXISTS idx_sync_queue_table ON sync_queue(table_name);
        "#,
    )
    .map_err(|e| format!("v001_baseline falló: {}", e))
}

// =======================================================================
// RUNNER
// =======================================================================

/// Aplica las migraciones pendientes de `MIGRATIONS`. Devuelve la versión final.
pub fn run_migrations(conn: &mut Connection) -> Result<u32, String> {
    run_migrations_with(conn, MIGRATIONS)
}

fn run_migrations_with(conn: &mut Connection, migrations: &[Migration]) -> Result<u32, String> {
    let current: u32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|e| format!("No se pudo leer user_version: {}", e))?;

    let latest = migrations.len() as u32;

    if current > latest {
        return Err(format!(
            "La base de datos (versión {}) fue creada por una versión más nueva de la app (máx. {}). Actualizá la aplicación para usarla.",
            current, latest
        ));
    }

    for m in migrations.iter().filter(|m| m.version > current) {
        let tx: Transaction = conn
            .transaction()
            .map_err(|e| format!("No se pudo abrir transacción para v{}: {}", m.version, e))?;

        (m.up)(&tx).map_err(|e| {
            format!(
                "Migración v{} ({}) falló y fue revertida: {}",
                m.version, m.description, e
            )
        })?;

        tx.execute_batch(&format!("PRAGMA user_version = {}", m.version))
            .map_err(|e| format!("No se pudo fijar user_version a {}: {}", m.version, e))?;

        tx.commit()
            .map_err(|e| format!("No se pudo confirmar la migración v{}: {}", m.version, e))?;

        println!(
            "🧬 [DB] Migración aplicada: v{} — {}",
            m.version, m.description
        );
    }

    Ok(latest.max(current))
}

// =======================================================================
// TESTS
// =======================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn open_mem() -> Connection {
        Connection::open_in_memory().unwrap()
    }

    fn user_version(conn: &Connection) -> u32 {
        conn.query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap()
    }

    fn table_exists(conn: &Connection, name: &str) -> bool {
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
            [name],
            |r| r.get::<_, i64>(0),
        )
        .unwrap()
            > 0
    }

    #[test]
    fn test_fresh_db_migrates_to_latest() {
        let mut conn = open_mem();
        assert_eq!(user_version(&conn), 0);

        let v = run_migrations(&mut conn).unwrap();
        assert_eq!(v, MIGRATIONS.len() as u32);
        assert_eq!(user_version(&conn), MIGRATIONS.len() as u32);

        // Las tablas del baseline existen
        assert!(table_exists(&conn, "users"));
        assert!(table_exists(&conn, "entries"));
        assert!(table_exists(&conn, "signing_keys"));
        assert!(table_exists(&conn, "audit_log"));
    }

    #[test]
    fn test_rerun_is_noop_and_preserves_data() {
        let mut conn = open_mem();
        run_migrations(&mut conn).unwrap();

        conn.execute(
            "INSERT INTO users (id, username, password_hash, role, created_at)
             VALUES ('u1', 'admin', 'hash', 'administrador', '2026-01-01')",
            [],
        )
        .unwrap();

        // Segunda pasada: no debe fallar ni borrar datos
        let v = run_migrations(&mut conn).unwrap();
        assert_eq!(v, MIGRATIONS.len() as u32);

        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn test_legacy_dev_db_without_version_migrates_idempotently() {
        // Simula una DB de desarrollo creada ANTES del harness:
        // tablas presentes pero user_version = 0.
        let mut conn = open_mem();
        conn.execute(
            "CREATE TABLE users (
                id TEXT PRIMARY KEY, username TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                role TEXT CHECK(role IN ('administrador','asistente','medico','enfermeria','paciente')) NOT NULL,
                entity_id INTEGER, created_at TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0,1)),
                last_login_at TEXT
            )",
            [],
        )
        .unwrap();

        let v = run_migrations(&mut conn).unwrap();
        assert_eq!(v, MIGRATIONS.len() as u32);
        assert!(table_exists(&conn, "entries"));
    }

    #[test]
    fn test_downgrade_rejected() {
        let mut conn = open_mem();
        run_migrations(&mut conn).unwrap();

        conn.execute_batch("PRAGMA user_version = 999").unwrap();

        let err = run_migrations(&mut conn).unwrap_err();
        assert!(err.contains("más nueva"), "error inesperado: {}", err);
        // La versión futura no se tocó
        assert_eq!(user_version(&conn), 999);
    }

    #[test]
    fn test_failed_migration_rolls_back() {
        fn up_ok(_conn: &Connection) -> Result<(), String> {
            Ok(())
        }
        fn up_fail(conn: &Connection) -> Result<(), String> {
            conn.execute_batch("CREATE TABLE temp_t (id INTEGER)")
                .map_err(|e| e.to_string())?;
            Err("boom".to_string())
        }

        let failing = [
            Migration {
                version: 1,
                description: "ok",
                up: up_ok,
            },
            Migration {
                version: 2,
                description: "falla a mitad",
                up: up_fail,
            },
        ];

        let mut conn = open_mem();
        let err = run_migrations_with(&mut conn, &failing).unwrap_err();
        assert!(err.contains("boom"), "error inesperado: {}", err);
        assert!(err.contains("revertida"), "debe indicar rollback: {}", err);

        // v1 quedó aplicada, v2 no (y su DDL intermedio se revirtió)
        assert_eq!(user_version(&conn), 1);
        assert!(!table_exists(&conn, "temp_t"));
    }
}
