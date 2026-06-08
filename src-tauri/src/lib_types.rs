use rusqlite::Connection;
use std::sync::Mutex;

// El contenedor seguro para compartir la conexión de SQLite
pub struct DbState(pub Mutex<Option<Connection>>);
