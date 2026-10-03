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
