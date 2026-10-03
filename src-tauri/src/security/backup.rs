use crate::db::migrations::MIGRATIONS;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

pub const MANIFEST_FILENAME: &str = "manifest.json";
pub const BACKUP_PREFIX: &str = "respaldo-";
const DB_FILENAME: &str = "historia_clinica.db";

/// Un archivo dentro del respaldo, con su huella de integridad.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FileEntry {
    pub name: String,
    pub sha256: String,
    pub bytes: u64,
}

/// Índice del respaldo: versión de app/esquema + huellas de cada archivo.
/// Todo el contenido listado ya está cifrado en reposo (la DB y los wraps
/// son incomprensibles sin contraseña o frase semilla).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Manifest {
    pub app_version: String,
    pub schema_version: u32,
    pub created_at: String,
    pub files: Vec<FileEntry>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Archivos que integran un respaldo: base, wraps, vaults y la sal de
/// instalación (sin ella los blind indexes del restore no coincidirían).
fn is_backup_candidate(name: &str) -> bool {
    name == DB_FILENAME
        || name == ".installation_salt"
        || name.starts_with(".data_key.")
        || (name.starts_with("vault_") && (name.ends_with(".bin") || name.ends_with(".salt")))
}

fn is_wal_sidecar(name: &str) -> bool {
    name.ends_with("-wal") || name.ends_with("-shm")
}

/// Vuelca el WAL dentro del `.db` para que el archivo quede autocontenido.
pub fn checkpoint(conn: &rusqlite::Connection) -> Result<(), String> {
    let (busy, _log, _checkpointed): (i64, i64, i64) = conn
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .map_err(|e| format!("Error en wal_checkpoint: {}", e))?;
    if busy != 0 {
        return Err(
            "Hay otra conexión activa a la base de datos; no se puede completar la operación."
                .to_string(),
        );
    }
    Ok(())
}

// =======================================================================
// CREAR
// =======================================================================

/// Crea un respaldo completo en `dest_parent/respaldo-{timestamp}/` y
/// devuelve (ruta, manifest).
pub fn create_backup(
    conn: &rusqlite::Connection,
    app_dir: &Path,
    dest_parent: &Path,
) -> Result<(PathBuf, Manifest), String> {
    checkpoint(conn)?;

    if !app_dir.join(DB_FILENAME).exists() {
        return Err("No se encontró la base de datos para respaldar.".to_string());
    }

    // Destino con timestamp (y desempate si existe en el mismo segundo)
    let ts = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let mut dest = dest_parent.join(format!("{}{}", BACKUP_PREFIX, ts));
    let mut n = 1;
    while dest.exists() {
        dest = dest_parent.join(format!("{}{}-{}", BACKUP_PREFIX, ts, n));
        n += 1;
    }
    fs::create_dir_all(&dest)
        .map_err(|e| format!("No se pudo crear la carpeta del respaldo: {}", e))?;

    // Recolectar archivos candidatos (sidecars WAL solo si quedó contenido)
    let mut names: Vec<String> = Vec::new();
    for entry in fs::read_dir(app_dir)
        .map_err(|e| format!("No se pudo leer el directorio de datos: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Error leyendo directorio: {}", e))?;
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_backup_candidate(&name) {
            names.push(name);
        } else if is_wal_sidecar(&name) {
            let len = entry.metadata().map(|m| m.len()).unwrap_or(0);
            if len > 0 {
                names.push(name);
            }
        }
    }
    names.sort();

    // Copiar + hashear
    let mut files = Vec::with_capacity(names.len());
    for name in names {
        let bytes = fs::read(app_dir.join(&name))
            .map_err(|e| format!("No se pudo leer '{}': {}", name, e))?;
        let entry = FileEntry {
            sha256: sha256_hex(&bytes),
            bytes: bytes.len() as u64,
            name: name.clone(),
        };
        fs::write(dest.join(&name), &bytes)
            .map_err(|e| format!("No se pudo copiar '{}': {}", name, e))?;
        files.push(entry);
    }

    let manifest = Manifest {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        schema_version: MIGRATIONS.len() as u32,
        created_at: chrono::Local::now().to_rfc3339(),
        files,
    };

    fs::write(
        dest.join(MANIFEST_FILENAME),
        serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("No se pudo escribir el manifest: {}", e))?;

    println!(
        "📦 [Backup] Respaldo creado en {:?} ({} archivos)",
        dest,
        manifest.files.len()
    );

    Ok((dest, manifest))
}

// =======================================================================
// VALIDAR
// =======================================================================

pub fn read_manifest(src: &Path) -> Result<Manifest, String> {
    let path = src.join(MANIFEST_FILENAME);
    let raw = fs::read(&path)
        .map_err(|e| format!("No se encontró {}: {}", MANIFEST_FILENAME, e))?;
    serde_json::from_slice(&raw).map_err(|e| format!("manifest.json inválido: {}", e))
}

/// Un manifest malicioso podría listar rutas relativas para escribir fuera
/// del directorio. Solo aceptamos nombres de archivo planos.
fn validate_file_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name == "."
        || name == ".."
        || name == MANIFEST_FILENAME
    {
        return Err(format!("Nombre de archivo inválido en el respaldo: {:?}", name));
    }
    Ok(())
}

/// Verifica integridad (SHA-256 por archivo) y compatibilidad de esquema.
pub fn validate_backup(src: &Path) -> Result<Manifest, String> {
    let manifest = read_manifest(src)?;

    if manifest.files.is_empty() {
        return Err("El respaldo no contiene archivos.".to_string());
    }

    let current = MIGRATIONS.len() as u32;
    if manifest.schema_version > current {
        return Err(format!(
            "El respaldo fue creado por una versión más nueva de la app (esquema {}, esta app soporta {}). Actualizá la aplicación.",
            manifest.schema_version, current
        ));
    }

    let mut has_db = false;
    for f in &manifest.files {
        validate_file_name(&f.name)?;
        if f.name == DB_FILENAME {
            has_db = true;
        }
        let path = src.join(&f.name);
        let bytes = fs::read(&path).map_err(|_| {
            format!("Falta el archivo '{}' dentro del respaldo.", f.name)
        })?;
        if bytes.len() as u64 != f.bytes {
            return Err(format!(
                "El archivo '{}' no coincide en tamaño con el manifest.",
                f.name
            ));
        }
        if sha256_hex(&bytes) != f.sha256 {
            return Err(format!(
                "El archivo '{}' está corrupto o alterado (hash no coincide).",
                f.name
            ));
        }
    }

    if !has_db {
        return Err("El respaldo no incluye la base de datos.".to_string());
    }

    Ok(manifest)
}

// =======================================================================
// RESTAURAR
// =======================================================================

/// Valida y copia los archivos del respaldo sobre `app_dir`.
/// No borra archivos que no estén en el manifest. El caller debe cerrar
/// la conexión a la DB antes y reabrirla (con migraciones) después.
pub fn restore_backup(src: &Path, app_dir: &Path) -> Result<Manifest, String> {
    let manifest = validate_backup(src)?;

    fs::create_dir_all(app_dir)
        .map_err(|e| format!("No se pudo crear el directorio de datos: {}", e))?;

    for f in &manifest.files {
        let bytes = fs::read(src.join(&f.name))
            .map_err(|e| format!("No se pudo leer '{}': {}", f.name, e))?;
        fs::write(app_dir.join(&f.name), &bytes)
            .map_err(|e| format!("No se pudo restaurar '{}': {}", f.name, e))?;
    }

    println!(
        "♻️  [Backup] Restauración completada desde {:?} ({} archivos)",
        src,
        manifest.files.len()
    );

    Ok(manifest)
}

// =======================================================================
// TESTS
// =======================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Directorio de datos de una "instalación" real: DB con una fila + wraps de prueba.
    fn make_installation() -> (TempDir, rusqlite::Connection) {
        let dir = TempDir::new().unwrap();
        let conn = crate::db::database::init_db(dir.path().to_path_buf()).unwrap();
        conn.execute(
            "INSERT INTO users (id, username, password_hash, role, created_at)
             VALUES ('u1', 'admin', 'hash', 'administrador', '2026-01-01')",
            [],
        )
        .unwrap();
        fs::write(dir.path().join(".installation_salt"), b"sal-instalacion-32b").unwrap();
        fs::write(dir.path().join(".data_key.master"), b"wrap-master-bytes").unwrap();
        fs::write(dir.path().join(".data_key.u1"), b"wrap-user-bytes").unwrap();
        fs::write(dir.path().join("vault_u1.bin"), b"vault-bytes").unwrap();
        fs::write(dir.path().join("vault_u1.salt"), b"vault-salt").unwrap();
        // Archivos que NO deben ir al respaldo
        fs::write(dir.path().join("preferencias.tmp"), b"no respaldar").unwrap();
        (dir, conn)
    }

    #[test]
    fn test_create_backup_writes_manifest_and_files() {
        let (app, conn) = make_installation();
        let dest_parent = TempDir::new().unwrap();

        let (path, manifest) =
            create_backup(&conn, app.path(), dest_parent.path()).unwrap();

        assert!(path.starts_with(dest_parent.path()));
        assert!(path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with(BACKUP_PREFIX));
        assert_eq!(manifest.schema_version, MIGRATIONS.len() as u32);
        assert!(!manifest.app_version.is_empty());

        // Todos los archivos candidatos, y solo ellos
        let names: Vec<&str> = manifest.files.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&DB_FILENAME));
        assert!(names.contains(&".installation_salt"));
        assert!(names.contains(&".data_key.master"));
        assert!(names.contains(&".data_key.u1"));
        assert!(names.contains(&"vault_u1.bin"));
        assert!(names.contains(&"vault_u1.salt"));
        assert!(!names.contains(&"preferencias.tmp"));

        // Copias idénticas + manifest legible
        assert!(path.join(MANIFEST_FILENAME).exists());
        for f in &manifest.files {
            let copied = fs::read(path.join(&f.name)).unwrap();
            assert_eq!(copied.len() as u64, f.bytes);
            assert_eq!(sha256_hex(&copied), f.sha256);
        }
        assert_eq!(read_manifest(&path).unwrap(), manifest);
    }

    #[test]
    fn test_validate_detects_tampered_file() {
        let (app, conn) = make_installation();
        let dest_parent = TempDir::new().unwrap();
        let (path, _) = create_backup(&conn, app.path(), dest_parent.path()).unwrap();

        // Alterar un byte del .db copiado (mismo tamaño → falla solo el hash)
        let db_path = path.join(DB_FILENAME);
        let mut bytes = fs::read(&db_path).unwrap();
        bytes[10] ^= 0xFF;
        fs::write(&db_path, &bytes).unwrap();

        let err = validate_backup(&path).unwrap_err();
        assert!(err.contains("corrupto o alterado"), "error: {}", err);
    }

    #[test]
    fn test_validate_rejects_missing_file() {
        let (app, conn) = make_installation();
        let dest_parent = TempDir::new().unwrap();
        let (path, _) = create_backup(&conn, app.path(), dest_parent.path()).unwrap();

        fs::remove_file(path.join(".data_key.master")).unwrap();

        let err = validate_backup(&path).unwrap_err();
        assert!(err.contains("Falta el archivo"), "error: {}", err);
    }

    #[test]
    fn test_validate_rejects_newer_schema() {
        let (app, conn) = make_installation();
        let dest_parent = TempDir::new().unwrap();
        let (path, _) = create_backup(&conn, app.path(), dest_parent.path()).unwrap();

        let mut manifest = read_manifest(&path).unwrap();
        manifest.schema_version = MIGRATIONS.len() as u32 + 5;
        fs::write(
            path.join(MANIFEST_FILENAME),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let err = validate_backup(&path).unwrap_err();
        assert!(err.contains("más nueva"), "error: {}", err);
    }

    #[test]
    fn test_validate_rejects_path_traversal() {
        let (app, conn) = make_installation();
        let dest_parent = TempDir::new().unwrap();
        let (path, _) = create_backup(&conn, app.path(), dest_parent.path()).unwrap();

        let mut manifest = read_manifest(&path).unwrap();
        manifest.files.push(FileEntry {
            name: "../fuera.txt".to_string(),
            sha256: "00".to_string(),
            bytes: 0,
        });
        fs::write(
            path.join(MANIFEST_FILENAME),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let err = validate_backup(&path).unwrap_err();
        assert!(err.contains("inválido"), "error: {}", err);
    }

    #[test]
    fn test_restore_roundtrip_recovers_installation() {
        let (app, conn) = make_installation();
        let dest_parent = TempDir::new().unwrap();
        let (path, _) = create_backup(&conn, app.path(), dest_parent.path()).unwrap();
        drop(conn);

        // Nueva instalación vacía (otra carpeta)
        let fresh = TempDir::new().unwrap();
        let manifest = restore_backup(&path, fresh.path()).unwrap();
        assert!(!manifest.files.is_empty());

        // La DB restaurada abre, migra y conserva los datos
        let conn2 = crate::db::database::init_db(fresh.path().to_path_buf()).unwrap();
        let (n, username): (i64, String) = conn2
            .query_row("SELECT COUNT(*), username FROM users", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(n, 1);
        assert_eq!(username, "admin");

        // Wraps y sal tal cual
        assert_eq!(
            fs::read(fresh.path().join(".data_key.master")).unwrap(),
            b"wrap-master-bytes"
        );
        assert_eq!(
            fs::read(fresh.path().join(".installation_salt")).unwrap(),
            b"sal-instalacion-32b"
        );
        assert!(fresh.path().join("vault_u1.bin").exists());

        // El archivo que no estaba en el respaldo no aparece
        assert!(!fresh.path().join("preferencias.tmp").exists());
    }
}
