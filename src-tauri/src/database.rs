use rusqlite::{Connection, Result};
use std::fs;
use std::path::PathBuf;

/// Inicializa la base de datos local y crea las tablas si no existen
pub fn init_db(app_dir: PathBuf) -> Result<Connection, String> {
    // Asegurar que la carpeta de la app exista en el sistema de archivos del usuario
    fs::create_dir_all(&app_dir)
        .map_err(|e| format!("No se pudo crear el directorio de datos: {}", e))?;

    // Definir la ruta del archivo SQLite (ej: AppData/Local/historia_clinica.db)
    let db_path = app_dir.join("historia_clinica.db");
    println!("Conectando a la base de datos local en: {:?}", db_path);

    let conn = Connection::open(db_path).map_err(|e| format!("Error al abrir SQLite: {}", e))?;

    // Habilitar Llaves Foráneas en SQLite
    conn.execute("PRAGMA foreign_keys = ON;", [])
        .map_err(|e| format!("Error al activar foreign_keys: {}", e))?;

    // Crear Tabla de Consultas Presenciales (SOAP) - Cifrada
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
            is_synced INTEGER DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY(paciente_id) REFERENCES patients(id)
        );",
        [],
    )
    .map_err(|e| format!("Error al crear tabla consultas: {}", e))?;

    // Crear Tabla de Métricas Clínicas (EAV) - Cifrado Selectivo
    conn.execute(
        "CREATE TABLE IF NOT EXISTS patient_metrics (
            id TEXT PRIMARY KEY,
            paciente_id TEXT NOT NULL,
            metric_type TEXT NOT NULL,
            sub_metric TEXT NOT NULL,
            value_num REAL,
            value_text_ciphertext TEXT,
            value_text_nonce TEXT,
            is_shared INTEGER DEFAULT 0,
            measured_at TEXT NOT NULL
        );",
        [],
    )
    .map_err(|e| format!("Error al crear tabla metricas: {}", e))?;

    // Crear Tabla de Usuarios (Médicos, Admins, Pacientes Autoregistrados)
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

    // Crear Tabla de Pacientes con Índice Ciego Único
    conn.execute(
        "CREATE TABLE IF NOT EXISTS patients (
                id TEXT PRIMARY KEY,
                created_by_user_id TEXT NOT NULL,
                full_name_ciphertext TEXT NOT NULL,
                full_name_nonce TEXT NOT NULL,
                identity_blind_index TEXT UNIQUE NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY(created_by_user_id) REFERENCES users(id)
            );",
        [],
    )
    .map_err(|e| format!("Error al crear tabla patients: {}", e))?;

    println!("¡Tablas de la base de datos local verificadas/creadas con éxito!");
    Ok(conn)
}
