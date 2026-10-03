use rusqlite::Connection;
use std::sync::Mutex;
use zeroize::Zeroizing;

// El contenedor seguro para compartir la conexión de SQLite
pub struct DbState(pub Mutex<Option<Connection>>);

// Llave maestra en RAM. `Zeroizing` pone los bytes a cero al soltar el
// Option (lock_vault / restore_backup) — sin esto, drop() solo libera
// memoria y los bytes quedan residuales en el heap.
pub struct CryptoState(pub Mutex<Option<Zeroizing<[u8; 32]>>>);

// Llave asimétrica (Ed25519) en RAM — misma garantía de zeroize-on-drop.
pub struct SigningState(pub Mutex<Option<Zeroizing<[u8; 32]>>>);

// Llave de cifrado compartida para datos médicos (entidades y notas).
pub struct DataKey(pub Mutex<Option<Zeroizing<[u8; 32]>>>);

// Usuario con sesión activa en esta instalación (quién abrió la bóveda).
// Los commands sensibles (usuarios, reset de contraseña, semilla) verifican
// el rol desde aquí — nunca desde el frontend.
#[derive(Clone)]
pub struct SessionUser {
    pub user_id: String,
    pub username: String,
    pub role: String,
}

pub struct SessionState(pub Mutex<Option<SessionUser>>);
