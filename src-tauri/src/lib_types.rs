use rusqlite::Connection;
use std::sync::Mutex;

// El contenedor seguro para compartir la conexión de SQLite
pub struct DbState(pub Mutex<Option<Connection>>);

// Aquí vivirá la llave maestra en RAM
pub struct CryptoState(pub Mutex<Option<[u8; 32]>>);
