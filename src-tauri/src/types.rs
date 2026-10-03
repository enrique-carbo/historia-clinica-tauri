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
