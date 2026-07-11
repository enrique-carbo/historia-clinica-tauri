use rusqlite::{Connection, Result};
use std::fs;
use std::path::PathBuf;

/// Inicializa la base de datos local y crea las tablas si no existen
pub fn init_db(app_dir: PathBuf) -> Result<Connection, String> {
    // Asegurar que la carpeta de la app exista en el sistema de archivos del usuario
    fs::create_dir_all(&app_dir)
        .map_err(|e| format!("No se pudo crear el directorio de datos: {}", e))?;

    // Definir la ruta del archivo SQLite
    let db_path = app_dir.join("historia_clinica.db");
    println!("Conectando a la base de datos local en: {:?}", db_path);

    let conn = Connection::open(db_path).map_err(|e| format!("Error al abrir SQLite: {}", e))?;

    // Habilitar Llaves Foráneas y Modo WAL (Concurrencia segura para múltiples médicos)
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;",
    )
    .map_err(|e| format!("Error al activar PRAGMAs: {}", e))?;

    // La regla de oro en SQL: Siempre crea primero las tablas Padre (las que son referenciadas)
    // y luego las Hijas. Orden: users -> professional_profiles -> patients -> consultas/métricas

    // 1. Tabla de Usuarios (Médicos, Admins, Pacientes)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                username TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                role TEXT CHECK(role IN ('admin', 'medico', 'paciente')) NOT NULL,
                entity_id INTEGER,
                created_at TEXT NOT NULL,
                FOREIGN KEY(entity_id) REFERENCES entities(id)
            );",
        [],
    )
    .map_err(|e| format!("Error al crear tabla users: {}", e))?;

    // 2. Tabla de Perfiles Profesionales (Datos del Médico/Admin + Llave Pública para Firmas)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS professional_profiles (
                user_id TEXT PRIMARY KEY,
                full_name_ciphertext TEXT NOT NULL,
                full_name_nonce TEXT NOT NULL,
                license_number_ciphertext TEXT NOT NULL,
                license_number_nonce TEXT NOT NULL,
                specialty_ciphertext TEXT NOT NULL,
                specialty_nonce TEXT NOT NULL,
                public_key TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY(user_id) REFERENCES users(id)
            );",
        [],
    )
    .map_err(|e| format!("Error al crear tabla professional_profiles: {}", e))?;

    // ============================================================
    // TABLAS GENÉRICAS (Template v1)
    // ============================================================

    // 3. Tabla Entidades genéricas: pacientes humanos o animales segun corresponda
    conn.execute(
        "CREATE TABLE IF NOT EXISTS entities (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            external_id TEXT UNIQUE,
            entity_type TEXT NOT NULL,
            created_by_user_id INTEGER NOT NULL,
            blind_index TEXT UNIQUE,
            enc_data_blob BLOB NOT NULL,
            created_at TEXT NOT NULL
        );",
        [],
    )
    .map_err(|e| format!("Error al crear tabla entities: {}", e))?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type);",
        [],
    )
    .ok();

    // 4. Tabla notas genéricas
    conn.execute(
        "CREATE TABLE IF NOT EXISTS notes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            external_id TEXT UNIQUE,
            entity_id INTEGER NOT NULL,
            template_id TEXT NOT NULL,
            created_by_user_id TEXT NOT NULL,
            enc_fields BLOB NOT NULL,
            signature BLOB NOT NULL,
            created_at TEXT NOT NULL
        );",
        [],
    )
    .map_err(|e| format!("Error al crear tabla notes: {}", e))?;

    // 5. Tabla patient_keys

    conn.execute(
        "CREATE TABLE IF NOT EXISTS patient_keys (
        patient_user_id TEXT PRIMARY KEY,
        patient_data_key_ciphertext TEXT NOT NULL,
        patient_data_key_nonce TEXT NOT NULL,
        created_at TEXT NOT NULL,
        FOREIGN KEY(patient_user_id) REFERENCES users(id)
    );",
        [],
    )
    .map_err(|e| format!("Error al crear tabla patients_keys: {}", e))?;

    // 6. Tabla cola de sincronización: reemplazará 'is_synced' en tablas individuales
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sync_queue (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            table_name TEXT NOT NULL,
            record_id INTEGER NOT NULL,
            operation TEXT NOT NULL,
            created_at TEXT NOT NULL
        );",
        [],
    )
    .map_err(|e| format!("Error al crear tabla sync_queue: {}", e))?;

    println!(
        "¡Tablas genéricas (template) de la base de datos local verificadas/creadas con éxito!"
    );

    Ok(conn)
}
