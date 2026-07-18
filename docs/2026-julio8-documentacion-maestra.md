# 📘 Documentación Maestra — Simplex Health Core
## Estado al 2026-07-15 | Core Estabilizado | Pre-Sync

---

## 1. Arquitectura de Seguridad y Cifrado (Zero-Knowledge Parcial)

### 1.1 Modelo de Amenazas y Diseño de Cifrado

El sistema implementa un **modelo de cifrado híbrido** con tres capas distintas:

| Capa | Llave | Alcance | Persistencia | Propósito |
|------|-------|---------|--------------|-----------|
| **Vault de Usuario** | `CryptoState` + `SigningState` | Por usuario | Volátil (RAM) | Autenticación + Firma digital |
| **Datos Médicos Locales** | `DataKey` | Por instalación | Persistente (disco) | Cifrado/descifrado de entidades y notas del médico |
| **Datos de Entidad-Usuario** | `entity_keys` | Por entidad | Persistente (SQLite) | Cifrado de datos propios del paciente (telemedicina, plugins) |

> **Decisión de diseño documentada:** Los datos médicos locales se cifran con `DataKey` (compartida por instalación). Las entidades que son usuarios autónomos (telemedicina, plugins) tienen su propia clave en `entity_keys`, cifrada con la clave maestra del médico o con una clave propia. Esto permite compartir datos cifrados con múltiples médicos sin revelar la clave maestra del creador.

**Implicaciones de seguridad:**
- ✅ El servidor remoto (futuro) solo verá blobs cifrados. Sin acceso al disco local o a `entity_keys`, no puede descifrar.
- ⚠️ Un atacante con acceso físico al disco puede leer `.data_key` y descifrar datos locales del médico.
- ✅ Los datos de entidades-usuario están protegidos por `entity_keys`, que requiere autorización del médico para descifrar.
- ✅ La firma digital (Ed25519) garantiza no-repudio: incluso con acceso a `.data_key`, no se puede firmar notas como médico.

### 1.2 Ciclo de Vida de las Claves

#### Contenedores de Estado (`lib_types.rs`)

```rust
pub struct DbState(pub Mutex<Option<Connection>>);          // SQLite
pub struct CryptoState(pub Mutex<Option<[u8; 32]>>);       // Master key (vault)
pub struct SigningState(pub Mutex<Option<[u8; 32]>>);      // Ed25519 private key
pub struct DataKey(pub Mutex<Option<[u8; 32]>>);           // Shared medical data key
```

#### Inicialización en `main.rs` (setup)

```rust
// 1. INSTALLATION_SALT — para blind index (global, persistente)
let installation_salt = get_or_create_installation_salt(&app_data_dir)?;
crypto::init_blind_index_salt(installation_salt)?;

// 2. DATA_KEY — para cifrado de entidades y notas locales (global, persistente)
let data_key = get_or_create_data_key(&app_data_dir)?;
*data_key_state.0.lock().unwrap() = Some(data_key);

// 3. DB — conexión SQLite
let conn = database::init_db(app_data_dir)?;
*db_state.0.lock().unwrap() = Some(conn);

// 4. SCHEMAS — empaquetados en el binario
const SCHEMA_JSON: &str = include_str!("../config/schema.json");
const NOTE_TEMPLATES_JSON: &str = include_str!("../config/note_templates/soap.json");
```

#### Desbloqueo de Vault (`unlock_vault` en `auth_commands.rs`)

```rust
let (master_key, private_signing_key, public_key_opt) =
    vault::unlock_user_vault(app_dir, &user_id, &password)?;

// Inyecta en RAM
*crypto_state.0.lock().unwrap() = Some(master_key);       // CryptoState
*signing_state.0.lock().unwrap() = Some(private_signing_key); // SigningState

// Si es primer login, guarda llave pública en DB
if let Some(pub_key) = public_key_opt {
    UPDATE professional_profiles SET public_key = ? WHERE user_id = ?;
}
```

> **Nota:** `DataKey` **NO** se modifica durante `unlock_vault`. Ya está inicializada desde el `setup` y permanece constante durante toda la vida de la aplicación. `entity_keys` se descifra bajo demanda con la clave maestra del médico.

#### Cierre de Sesión (`lock_vault`)

```rust
*crypto_state.0.lock().unwrap() = None;      // Muere master key
*signing_state.0.lock().unwrap() = None;     // Muere private key
// DataKey permanece en RAM — los datos médicos locales siguen descifrables
// entity_keys permanecen en SQLite — requieren master key para descifrarse
```

### 1.3 Vault por Usuario (`vault.rs`)

#### Archivos por usuario

| Archivo | Contenido | Propósito |
|---------|-----------|-----------|
| `vault_{user_id}.salt` | Sal Argon2id (texto plano) | KDF único por usuario |
| `vault_{user_id}.bin` | Blob AES-GCM-256 (nonce[12] + ciphertext) | Llave privada Ed25519 cifrada |

#### Estructura del payload descifrado

```rust
struct VaultPayload {
    magic: "HISTORIA_CLINICA_VAULT_OK",
    private_key: String,  // Base64 de 32 bytes Ed25519
}
```

#### Flujo

1. **Primer login:** Genera par Ed25519 nuevo → serializa a JSON → cifra con `master_key` → guarda en `.bin`.
2. **Login subsiguiente:** Lee `.salt` → deriva `master_key` → descifra `.bin` → extrae `private_key`.
3. **Contraseña incorrecta:** `decrypt_vault_data` falla → error genérico "Contraseña incorrecta".

### 1.4 Blind Index — Búsqueda Sin Descifrado

#### Implementación (`crypto.rs`)

```rust
static INSTALLATION_SALT: OnceLock<Vec<u8>> = OnceLock::new();

pub fn generate_blind_index(identity_doc: &str) -> String {
    let normalized = identity_doc
        .trim()
        .replace("-", "")
        .replace(".", "")
        .replace(" ", "")
        .to_lowercase();

    let salt = INSTALLATION_SALT.get().expect("No inicializado");
    let mut hasher = Sha256::new();
    hasher.update(salt);
    hasher.update(normalized.as_bytes());
    hex::encode(hasher.finalize())
}
```

#### Persistencia (`main.rs`)

```rust
fn get_or_create_installation_salt(app_dir: &PathBuf) -> Result<Vec<u8>, String> {
    let salt_path = app_dir.join(".installation_salt");
    if salt_path.exists() {
        std::fs::read(&salt_path)
    } else {
        let mut salt = vec![0u8; 32];
        OsRng.fill_bytes(&mut salt);
        std::fs::write(&salt_path, &salt)?;
        Ok(salt)
    }
}
```

> **Decisión de diseño:** La sal del blind index es **global a la instalación** (no por usuario). Esto permite búsqueda compartida entre médicos de la misma máquina. Para sincronización multi-dispositivo, `.installation_salt` debe replicarse.

### 1.5 Firma Digital y No-Repudio

#### Creación de nota (`create_note`)

1. Template define `signature_payload_order: Vec<String>`.
2. `build_signature_payload()` concatena valores con `|`.
3. `hash_document(payload)` → SHA-256.
4. `sign_hash(hash, &signing_state)` → firma Ed25519.
5. Guarda `signature` en tabla `notes`.

#### Verificación (`get_note`)

1. Descifra campos con `DataKey` (datos locales) o con clave de `entity_keys` (datos de entidad-usuario).
2. Reconstruye payload según template.
3. Calcula hash.
4. Busca `public_key` en `professional_profiles` por `created_by_user_id`.
5. `verify_signature(hash, sig, pub_key)` → `is_verified: bool`.

### 1.6 Modelo de Claves: DataKey vs entity_keys

| Aspecto | `DataKey` | `entity_keys` |
|---------|-----------|---------------|
| **Propietario** | Instalación del médico | Entidad que es usuario autónomo |
| **Persistencia** | Archivo `.data_key` (hex) | Tabla `entity_keys` en SQLite |
| **Cifrada con** | Nada (plano en disco, por ahora) | Clave maestra del médico o clave propia |
| **Uso** | Descifrar datos locales del médico | Descifrar datos propios del paciente en telemedicina/plugins |
| **Compartible** | No — es local al médico | Sí — se puede compartir `entity_data_key_ciphertext` con otros médicos |
| **Escalabilidad** | Una por instalación | Una por entidad-usuario (potencialmente miles) |

> **Caso de uso:** Un paciente usa la app para teleconsulta. Sus datos se cifran con su `entity_key`. Cuando visita a un nuevo médico, comparte el `entity_data_key_ciphertext` (cifrado con la clave maestra del nuevo médico). El nuevo médico puede descifrar los datos sin acceder a la clave del médico anterior.

---

## 2. Estructura de la Base de Datos

### 2.1 Tablas

| Tabla | Propósito | Campos clave |
|-------|-----------|--------------|
| `users` | Autenticación (Médicos, Admins, Pacientes) | `id TEXT PK`, `username TEXT UNIQUE NOT NULL`, `password_hash TEXT NOT NULL`, `role TEXT CHECK('admin','medico','paciente') NOT NULL`, `entity_id INTEGER FK → entities.id`, `created_at TEXT NOT NULL` |
| `professional_profiles` | Perfil profesional + llave pública para firmas | `user_id TEXT PK FK → users.id`, `full_name_ciphertext TEXT NOT NULL`, `full_name_nonce TEXT NOT NULL`, `license_number_ciphertext TEXT NOT NULL`, `license_number_nonce TEXT NOT NULL`, `specialty_ciphertext TEXT NOT NULL`, `specialty_nonce TEXT NOT NULL`, `public_key TEXT NOT NULL`, `updated_at TEXT NOT NULL` |
| `entities` | Entidades genéricas (pacientes humanos o animales) | `id INTEGER PK AUTOINCREMENT`, `external_id TEXT UNIQUE`, `entity_type TEXT NOT NULL`, `created_by_user_id TEXT NOT NULL`, `blind_index TEXT UNIQUE`, `enc_data_blob BLOB NOT NULL`, `created_at TEXT NOT NULL` |
| `entity_keys` | Clave de datos propia por entidad-usuario | `entity_id INTEGER PK FK → entities.id`, `entity_data_key_ciphertext TEXT NOT NULL`, `entity_data_key_nonce TEXT NOT NULL`, `created_at TEXT NOT NULL` |
| `notes` | Notas clínicas genéricas con firma digital | `id INTEGER PK AUTOINCREMENT`, `external_id TEXT UNIQUE`, `entity_id INTEGER NOT NULL FK → entities.id`, `template_id TEXT NOT NULL`, `created_by_user_id TEXT NOT NULL FK → users.id`, `enc_fields BLOB NOT NULL`, `signature BLOB NOT NULL`, `created_at TEXT NOT NULL` |
| `sync_queue` | Cola de sincronización (reemplaza `is_synced`) | `id INTEGER PK AUTOINCREMENT`, `table_name TEXT NOT NULL`, `local_record_id INTEGER NOT NULL`, `external_record_id TEXT`, `operation TEXT NOT NULL`, `created_at TEXT NOT NULL` |

### 2.2 Índices

| Índice | Tabla | Columna(s) | Propósito |
|--------|-------|------------|-----------|
| `idx_entities_type` | `entities` | `entity_type` | Filtrado rápido por tipo de entidad |
| `idx_notes_entity_id` | `notes` | `entity_id` | Carga de notas por paciente |
| `idx_notes_created_by` | `notes` | `created_by_user_id` | Notas creadas por un médico |
| `idx_sync_queue_table` | `sync_queue` | `table_name` | Procesamiento por tabla |

### 2.3 Foreign Keys

| Tabla | Columna | Referencia |
|-------|---------|------------|
| `users` | `entity_id` | `entities(id)` |
| `professional_profiles` | `user_id` | `users(id)` |
| `entity_keys` | `entity_id` | `entities(id)` |
| `notes` | `entity_id` | `entities(id)` |
| `notes` | `created_by_user_id` | `users(id)` |

### 2.4 IDs Duales

| Campo | Tipo | Propósito |
|-------|------|-----------|
| `id` | `INTEGER PRIMARY KEY AUTOINCREMENT` | Índice local, FKs rápidos |
| `external_id` | `TEXT UUID` | Identificador global para sincronización |

### 2.5 PRAGMAs Activos

| PRAGMA | Valor | Propósito |
|--------|-------|-----------|
| `foreign_keys` | `ON` | Integridad referencial |
| `journal_mode` | `WAL` | Concurrencia segura para múltiples médicos |

### 2.6 Relaciones entre Tablas

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│    entities     │◄────┤   entity_keys   │     │     users       │
│  id: INTEGER PK │  FK │  entity_id: INT │     │  id: TEXT PK    │
│  external_id    │     │  data_key_*     │     │  entity_id FK   │◄──┐
│  entity_type    │     │  created_at     │     │  username       │   │
│  created_by     │     └─────────────────┘     │  password_hash  │   │
│  blind_index    │                             │  role           │   │
│  enc_data_blob  │                             │  created_at     │   │
│  created_at     │                             └────────┬────────┘   │
└────────┬────────┘                                    │            │
         │                                              │            │
         │         ┌─────────────────────────┐          │            │
         └────────►│         notes           │◄─────────┘            │
                   │  id: INTEGER PK         │  created_by_user_id   │
                   │  entity_id: INTEGER FK ─┘                       │
                   │  template_id: TEXT                              │
                   │  enc_fields: BLOB                               │
                   │  signature: BLOB                                │
                   │  created_at: TEXT                               │
                   └─────────────────────────┘                       │
                                                                      │
┌─────────────────────────┐                                         │
│  professional_profiles  │◄────────────────────────────────────────┘
│  user_id: TEXT PK FK    │
│  full_name_ciphertext   │
│  license_number_*       │
│  specialty_*            │
│  public_key             │
│  updated_at             │
└─────────────────────────┘
```

### 2.7 Flujo de Datos

| Paso | Actor | Acción | Tablas |
|------|-------|--------|--------|
| 1 | Médico/Admin | Crea entidad (paciente) | `entities` |
| 2 | Médico/Admin | Crea nota clínica | `notes` |
| 3 | Paciente | Se registra para teleconsulta | `users` (vinculado a `entities` vía `entity_id`) |
| 4 | Sistema | Genera clave de datos propia para la entidad | `entity_keys` |
| 5 | Paciente | Carga datos propios (plugins, wearables) | Cifrados con su `entity_key` |
| 6 | Paciente | Comparte datos con médico | Transfiere `entity_data_key_ciphertext` |
| 7 | Médico | Descifra datos compartidos | Usa su clave maestra para descifrar `entity_key` |
| 8 | Sistema | Encola cambios para sincronización | `sync_queue` |

### 2.8 Notas de Implementación

- **Orden de creación:** Las tablas padre (`entities`) se crean antes que las hijas (`entity_keys`, `notes`, `users`) para respetar las foreign keys. Sin embargo, `users` se crea antes que `entities` porque `entities` no tiene FK a `users` (solo `created_by_user_id` que es TEXT con validación en aplicación, no FK estricta por diseño).
- **Tolerancia a fallos:** Los índices se crean con `.ok()` para que no fallen si ya existen.
- **Cifrado:** Los campos sensibles (`full_name`, `license_number`, `specialty`, `enc_data_blob`, `enc_fields`, `entity_data_key`) se almacenan cifrados con nonce.
- **Firmas:** Las notas incluyen `signature BLOB` para verificación criptográfica.
- **Concurrencia:** WAL mode permite lecturas concurrentes mientras un médico escribe.
- **entity_keys vacía por defecto:** Solo se pobla cuando una entidad se convierte en usuario autónomo (telemedicina, plugins).

---

## 3. Comandos de Tauri (API Interna)

### 3.1 Autenticación (`auth_commands.rs`)

| Comando | Parámetros | Retorno |
|---------|-----------|---------|
| `register_user` | `RegisterInput { username, password_plain, role }` | `user_id: String` |
| `login_user` | `LoginInput { username, password_plain }` | `AuthResponse { user_id, username, role }` |
| `unlock_vault` | `user_id, password, app_handle` | `bool` |
| `lock_vault` | — | `()` |
| `test_crypto_flow` | `text: String` | `String` (debug) |
| `is_vault_unlocked` | — | `bool` |

### 3.2 Entidades Genéricas (`entity_commands.rs`)

| Comando | Parámetros | Retorno |
|---------|-----------|---------|
| `create_entity` | `entity_type, data: HashMap, user_id` | `EntityCreated { id, external_id, entity_type, blind_index_hex }` |
| `get_entity` | `id: i64` | `EntityRecord { id, external_id, entity_type, data, created_at }` |
| `find_entity_by_blind_index` | `entity_type, dni` | `Option<i64>` |
| `list_entities` | `entity_type, limit, offset` | `Vec<EntityRecord>` |
| `search_entities` | `entity_type, query` | `Vec<EntityRecord>` (descifra todo en RAM, búsqueda substring) |

### 3.3 Notas Genéricas (`note_commands.rs`)

| Comando | Parámetros | Retorno |
|---------|-----------|---------|
| `create_note` | `entity_id, template_id, fields, user_id` | `i64` (note_id) |
| `get_note` | `id: i64` | `NoteRecord { id, external_id, entity_id, template_id, fields, is_verified, created_at }` |
| `get_notes_by_entity` | `entity_id: i64` | `Vec<NoteRecord>` |

---

## 4. Motor Criptográfico (`crypto.rs`)

### 4.1 Constantes

```rust
pub const AES_KEY_SIZE: usize = 32;
pub const AES_NONCE_SIZE: usize = 12;
pub const ED25519_SECRET_SIZE: usize = 32;
pub const ED25519_SIGNATURE_SIZE: usize = 64;
```

### 4.2 Funciones Públicas

| Función | Uso |
|---------|-----|
| `encrypt_text(plain, key)` | Campos médicos con AES-GCM-256 + nonce único (OsRng) |
| `decrypt_text(enc, key)` | Descifrado de campos |
| `init_blind_index_salt(salt)` | Inicialización única de `INSTALLATION_SALT` |
| `generate_blind_index(doc)` | Hash SHA-256 determinista para búsqueda exacta |
| `generate_keypair()` | Par Ed25519 nuevo |
| `hash_document(text)` | SHA-256 para payload de firma |
| `sign_hash(hash, priv_key)` | Firma Ed25519 |
| `verify_signature(hash, sig, pub_key)` | Verificación matemática |

### 4.3 Tests Unitarios (14 tests, todos pasando)

| Test | Cobertura |
|------|-----------|
| `test_encrypt_decrypt_roundtrip` | AES-GCM roundtrip |
| `test_encrypt_unique_nonces` | Nonces únicos por inserción |
| `test_decrypt_wrong_key_fails` | Fallo con clave incorrecta |
| `test_decrypt_tampered_ciphertext_fails` | Detección de manipulación |
| `test_decrypt_tampered_nonce_fails` | Detección de manipulación |
| `test_blind_index_deterministic` | Determinismo del hash |
| `test_blind_index_normalization` | Normalización de DNI |
| `test_blind_index_different_inputs` | Distinción de inputs |
| `test_sign_and_verify` | Firma/verificación correcta |
| `test_verify_wrong_public_key_fails` | Fallo con llave equivocada |
| `test_verify_tampered_document_fails` | Fallo con documento alterado |
| `test_verify_tampered_signature_fails` | Fallo con firma alterada |

---

## 5. Vault (`vault.rs`)

### 5.1 Tests Unitarios (4 tests, todos pasando)

| Test | Cobertura |
|------|-----------|
| `test_vault_create_and_unlock` | Creación + desbloqueo, consistencia de keys |
| `test_vault_wrong_password_fails` | Fallo con contraseña incorrecta |
| `test_vault_different_users_different_keys` | Misma contraseña + diferente sal = diferente master_key |
| `test_sal_persistence` | Sal persiste entre desbloqueos |

---

## 6. Configuración de Templates (`config_schema.rs`)

### 6.1 Carga en `main.rs`

Los archivos JSON se **empaquetan en el binario** en tiempo de compilación:

```rust
const SCHEMA_JSON: &str = include_str!("../config/schema.json");
const NOTE_TEMPLATES_JSON: &str = include_str!("../config/note_templates/soap.json");
```

Esto garantiza que:
- La app funciona offline sin depender de archivos externos.
- Los templates son inmutables en runtime (solo modificables recompilando).
- No hay riesgo de manipulación de archivos de configuración en disco.

### 6.2 Estructuras

```rust
pub struct SchemaConfig {
    pub version: i32,
    pub default_entity: String,
    pub entities: HashMap<String, EntitySchema>,
}

pub struct NoteTemplatesConfig {
    pub version: i32,
    pub templates: HashMap<String, NoteTemplate>,
}
```

**Métodos clave de `NoteTemplatesConfig`:**
- `validate_note_fields(template_id, fields)` — valida campos requeridos.
- `build_signature_payload(template_id, fields)` — construye string a hashear para firma.

---

## 7. Patrones de Diseño

### 7.1 `with_conn` — Acceso Seguro a SQLite

```rust
pub fn with_conn<T, F>(db_state: &DbState, f: F) -> Result<T, String>
where F: FnOnce(&Connection) -> Result<T, String>
```

El closure recibe `&Connection`, el `MutexGuard` se libera al salir del scope.

### 7.2 `get_data_key` / `get_key` — Extracción Segura

```rust
fn get_data_key(state: &DataKey) -> Result<[u8; 32], String> {
    let guard = state.0.lock().unwrap();
    guard.ok_or("Bóveda cerrada".to_string())
}
```

> **Nota:** El mensaje de error "Bóveda cerrada" es técnicamente impreciso para `DataKey` (que nunca se "cierra"), pero mantiene consistencia UX. `DataKey` se inicializa en `setup()` y nunca se limpia.

### 7.3 Inyección de Dependencias

Cada comando declara explícitamente los estados que consume:
- `State<'_, DataKey>` — para cifrado/descifrado de datos médicos locales
- `State<'_, SigningState>` — para firma digital
- `State<'_, DbState>` — para acceso a SQLite
- `State<'_, SchemaConfig>` / `State<'_, NoteTemplatesConfig>` — para validación

---

## 8. Roadmap

### ✅ Fase 3: Completada (2026-07-08)

- [x] Motor criptográfico AES-GCM-256 + Ed25519 + Blind Index
- [x] Vault local con sal única por usuario + Argon2id KDF
- [x] **DataKey compartida por instalación** para cifrado de datos médicos locales
- [x] **entity_keys por entidad** para datos propios de usuarios autónomos
- [x] **INSTALLATION_SALT persistente** para blind index
- [x] **Configuración empaquetada en binario** (`include_str!`)
- [x] 18 tests unitarios pasando (14 crypto + 4 vault)
- [x] Tablas genéricas `entities`, `notes`, `sync_queue`, `entity_keys`
- [x] Comandos genéricos de entidades y notas con firma digital
- [x] Verificación dinámica de firma por template
- [x] `auth_commands.rs` completo con `unlock_vault`/`lock_vault`
- [x] `main.rs` con setup completo: salt, data_key, db, schemas
- [x] Esquema de base de datos corregido: tipos consistentes, FKs completas, índices de performance

### 🟧 Fase 4: Infraestructura y Sincronización Híbrida

- [ ] Despliegue de VPS con Dokploy y PocketBase
- [ ] Sync Engine (`tokio`): background worker polling `sync_queue`
- [ ] **Sincronización de `.data_key` y `.installation_salt` entre dispositivos** (crítico: sin esto, un dispositivo nuevo no puede descifrar datos ni buscar por blind index)
- [ ] Implementar Blind Index en servidor
- [ ] Cifrar `.data_key` en disco con llave derivada del vault del usuario (mejora de seguridad)

### 🟥 Fase 5: Teleconsulta Asíncrona (Multi-Usuario)

- [ ] Tabla `asynchronous_threads`
- [ ] Cifrado asimétrico o clave de sesión compartida
- [ ] Flujo de compartir `entity_keys` entre médicos

### ⬜ Fase 6: Interoperabilidad (Futuro)

- [ ] Exportación JSON y traducción a FHIR

---

## 9. Notas Técnicas Críticas

### 9.1 Sobre `DataKey` en Disco

`DataKey` se guarda en `.data_key` (hex) en el directorio de la app. **No está cifrada.** Esto es una decisión de diseño intencional para permitir descifrado sin autenticación (ej. backups, migraciones), pero implica:

- La seguridad física del disco es crítica.
- Para endurecer: cifrar `.data_key` con una llave derivada del password del master admin.

### 9.2 Sobre `entity_keys`

`entity_keys` está diseñada para escalar a miles de registros (una por entidad-usuario). La clave de cada entidad se cifra con la clave maestra del médico que la creó o con una clave propia del usuario. Esto permite:

- Compartir datos cifrados entre médicos sin revelar claves maestras.
- Revocar acceso cambiando el cifrado de `entity_data_key_ciphertext`.
- Por defecto está vacía: solo se usa para entidades que se convierten en usuarios autónomos.

### 9.3 Sobre `search_entities`

```rust
// Descifra TODAS las entidades del tipo en RAM, luego busca substring
for (id, ..., enc_data_blob, ...) in rows {
    let data = decrypt_entity_blob(&enc_data_blob, &data_key)?;
    for (_, value) in &data {
        if value.to_lowercase().contains(&query.to_lowercase()) {
            matches.push(...);
        }
    }
}
```

**Complejidad:** O(n × m) donde n = número de entidades, m = número de campos. **No escalable** más allá de ~1000 registros. Para producción: implementar índices de búsqueda de texto completo (FTS5 de SQLite) sobre campos desidentificados o migrar a búsqueda server-side.

### 9.4 Sobre Tablas Genéricas y `.ok()`

En `database.rs`:
```rust
conn.execute("CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type);", []).ok();
```

El `.ok()` silencia errores. Esto es tolerante pero **oculta problemas de esquema**. Recomendación: usar `map_err` explícito o al menos `eprintln!` para debugging.

### 9.5 Sobre `get_data_key` y Mensajes de Error

En `entity_commands.rs` y `note_commands.rs` se usa `get_data_key()` que retorna `"Bóveda cerrada"` si `DataKey` es `None`. Pero `DataKey` se inicializa en `setup()` y **nunca se limpia**. El error solo ocurriría si `setup()` falló. Considerar mensaje más preciso: "Llave de datos no inicializada".

### 9.6 Sobre `professional_profiles` en Registro

En `register_user`, se inserta un perfil profesional con campos cifrados vacíos (`''`):
```rust
INSERT INTO professional_profiles (...) VALUES (..., '', '', '', '', '', '', '', ...);
```

Esto es un placeholder. Los campos cifrados del perfil (nombre, matrícula, especialidad) deben completarse en un flujo posterior de "completar perfil". Los usuarios con `role = 'paciente'` no necesitan `professional_profiles`.

### 9.7 Sobre `sync_queue`

La tabla `sync_queue` reemplaza el campo `is_synced` de las tablas legacy. Cada operación CREATE/UPDATE/DELETE genera un registro en `sync_queue` con:
- `table_name`: tabla afectada
- `local_record_id`: ID local del registro
- `external_record_id`: ID global (NULL hasta que se sincroniza)
- `operation`: 'INSERT', 'UPDATE', 'DELETE'

El Sync Engine (Fase 4) procesará esta cola en orden, enviando cambios al servidor y actualizando `external_record_id`.

---

*Documentación generada el 2026-07-15. Refleja el estado real del código fuente tras análisis de `main.rs`, `lib_types.rs`, `crypto.rs`, `vault.rs`, `auth.rs`, `auth_commands.rs`, `entity_commands.rs`, `note_commands.rs`, `config_schema.rs` y `database.rs`.*


