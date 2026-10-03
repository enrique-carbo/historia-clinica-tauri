use rusqlite::Connection;
use std::sync::Mutex;

// El contenedor seguro para compartir la conexión de SQLite
pub struct DbState(pub Mutex<Option<Connection>>);

// Aquí vivirá la llave maestra en RAM
pub struct CryptoState(pub Mutex<Option<[u8; 32]>>);

// Aquí vivirá Llave asimétrica (Ed25519)
pub struct SigningState(pub Mutex<Option<[u8; 32]>>);

// Llave de cifrado compartida para datos médicos (entidades y notas)
pub struct DataKey(pub Mutex<Option<[u8; 32]>>);

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
