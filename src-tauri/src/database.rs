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

    // 1. Tabla de Usuarios (Médicos, Admins, Pacientes Autoregistrados)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                username TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                role TEXT CHECK(role IN ('admin', 'medico', 'paciente')) NOT NULL,
                created_at TEXT NOT NULL
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

    // 3. Tabla de Pacientes (Patrón: Blob Cifrado + Índices Ciegos)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS patients (
                id TEXT PRIMARY KEY,
                created_by_user_id TEXT NOT NULL,

                -- Índices Ciegos (Texto plano hasheado, para buscar rápido sin descifrar)
                identity_blind_index TEXT UNIQUE NOT NULL,
                name_blind_index TEXT NOT NULL,

                -- Datos Básicos para mostrar rápido en listados (Cifrados)
                full_name_ciphertext TEXT NOT NULL,
                full_name_nonce TEXT NOT NULL,

                -- Resto de datos sensibles (JSON Cifrado: nacimiento, mail, teléfono, etc.)
                encrypted_data_blob TEXT NOT NULL,
                encrypted_data_nonce TEXT NOT NULL,

                created_at TEXT NOT NULL,
                FOREIGN KEY(created_by_user_id) REFERENCES users(id)
            );",
        [],
    )
    .map_err(|e| format!("Error al crear tabla patients: {}", e))?;

    // 4. Tabla de Consultas Presenciales (SOAP) - Cifrada y lista para Firma Digital
    conn.execute(
        "CREATE TABLE IF NOT EXISTS in_person_consultations (
            id TEXT PRIMARY KEY,
            paciente_id TEXT NOT NULL,
            medico_id TEXT NOT NULL,
            s_subjetivo_ciphertext TEXT NOT NULL,
            s_subjetivo_nonce TEXT NOT NULL,
            o_objetivo_ciphertext TEXT NOT NULL,
            o_objetivo_nonce TEXT NOT NULL,
            a_analisis_ciphertext TEXT NOT NULL,
            a_analisis_nonce TEXT NOT NULL,
            p_plan_ciphertext TEXT NOT NULL,
            p_plan_nonce TEXT NOT NULL,
            digital_signature TEXT,
            is_synced INTEGER DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY(paciente_id) REFERENCES patients(id),
            FOREIGN KEY(medico_id) REFERENCES users(id)
        );",
        [],
    )
    .map_err(|e| format!("Error al crear tabla consultas: {}", e))?;

    // 5. Tabla de Métricas Clínicas (EAV) - Cifrado Selectivo y Firma Opcional
    conn.execute(
        "CREATE TABLE IF NOT EXISTS patient_metrics (
            id TEXT PRIMARY KEY,
            paciente_id TEXT NOT NULL,
            medico_id TEXT NOT NULL,
            metric_type TEXT NOT NULL,
            sub_metric TEXT NOT NULL,
            value_num REAL,
            value_text_ciphertext TEXT,
            value_text_nonce TEXT,
            digital_signature TEXT,
            is_shared INTEGER DEFAULT 0,
            measured_at TEXT NOT NULL,
            FOREIGN KEY(paciente_id) REFERENCES patients(id),
            FOREIGN KEY(medico_id) REFERENCES users(id)
        );",
        [],
    )
    .map_err(|e| format!("Error al crear tabla metricas: {}", e))?;

    println!("¡Tablas de la base de datos local verificadas/creadas con éxito!");

    // ============================================================
    // TABLAS GENÉRICAS (Template v1 - Experimental)
    // Conviven con tablas legacy durante migración gradual.
    // No se usan en producción aún. Solo para desarrollo y testing.
    // ============================================================

    // Entidades genéricas: reemplazará 'patients' en el futuro
    conn.execute(
        "CREATE TABLE IF NOT EXISTS entities (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            entity_type TEXT NOT NULL,
            created_by_user_id INTEGER NOT NULL,
            blind_index BLOB UNIQUE,
            enc_data_blob BLOB NOT NULL,
            created_at TEXT NOT NULL
        );",
        [],
    )
    .ok();

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type);",
        [],
    )
    .ok();

    // Notas genéricas: reemplazará 'in_person_consultations' en el futuro
    conn.execute(
        "CREATE TABLE IF NOT EXISTS notes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            entity_id INTEGER NOT NULL,
            template_id TEXT NOT NULL,
            created_by_user_id TEXT NOT NULL,
            enc_fields BLOB NOT NULL,
            signature BLOB NOT NULL,
            created_at TEXT NOT NULL
        );",
        [],
    )
    .ok();

    // Cola de sincronización: reemplazará 'is_synced' en tablas individuales
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
    .ok();

    println!("Tablas genéricas (template) verificadas.");

    Ok(conn)
}
