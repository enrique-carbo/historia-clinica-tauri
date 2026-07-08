# 📘 Documentación Maestra Actualizada — Simplex Health Core
## Estado al 2026-07-08 | Fase 3 Completada | Corrección de Arquitectura de Cifrado

---

## 1. Arquitectura de Seguridad y Cifrado (Zero-Knowledge Parcial)

### 1.1 Modelo de Amenazas y Diseño de Cifrado

El sistema implementa un **modelo de cifrado híbrido** con dos capas distintas:

| Capa | Llave | Alcance | Persistencia | Propósito |
|------|-------|---------|--------------|-----------|
| **Vault de Usuario** | `CryptoState` + `SigningState` | Por usuario | Volátil (RAM) | Autenticación + Firma digital |
| **Datos Médicos** | `DataKey` | Por instalación | Persistente (disco) | Cifrado/descifrado de entidades y notas |

> **Decisión de diseño documentada:** Los datos médicos (entidades y notas) se cifran con una llave **compartida a nivel de instalación** (`DataKey`), no con la llave maestra del usuario. Esto permite que cualquier médico autenticado descifre los datos de cualquier paciente, mientras que la **autenticación y la firma digital** son estrictamente por usuario.

**Implicaciones de seguridad:**
- ✅ El servidor remoto (futuro) solo verá blobs cifrados con `DataKey`. Sin acceso al disco local, no puede descifrar.
- ⚠️ Un atacante con acceso físico al disco puede leer `.data_key` y descifrar toda la base de datos.
- ✅ Un atacante sin acceso al disco debe romper Argon2id del vault del usuario (protegido por password).
- ✅ La firma digital (Ed25519) garantiza no-repudio: incluso si alguien roba `.data_key`, no puede firmar notas como médico.

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

// 2. DATA_KEY — para cifrado de entidades y notas (global, persistente)
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

> **Nota:** `DataKey` **NO** se modifica durante `unlock_vault`. Ya está inicializada desde el `setup` y permanece constante durante toda la vida de la aplicación.

#### Cierre de Sesión (`lock_vault`)

```rust
*crypto_state.0.lock().unwrap() = None;      // Muere master key
*signing_state.0.lock().unwrap() = None;     // Muere private key
// DataKey permanece en RAM — los datos médicos siguen descifrables
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

1. Descifra campos con `DataKey`.
2. Reconstruye payload según template.
3. Calcula hash.
4. Busca `public_key` en `professional_profiles` por `created_by_user_id`.
5. `verify_signature(hash, sig, pub_key)` → `is_verified: bool`.

---

## 2. Estructura de la Base de Datos

### 2.1 Tablas Legacy (Producción)

| Tabla | Propósito | Campos clave |
|-------|-----------|--------------|
| `users` | Autenticación | `id TEXT PK`, `username TEXT UNIQUE`, `password_hash TEXT`, `role TEXT CHECK('admin','medico','paciente')` |
| `professional_profiles` | Perfil + llave pública | `user_id TEXT PK FK`, `full_name_ciphertext/nonce`, `license_number_ciphertext/nonce`, `specialty_ciphertext/nonce`, `public_key TEXT` |
| `patients` | Pacientes legacy | `id TEXT PK`, `identity_blind_index TEXT UNIQUE`, `name_blind_index TEXT`, `full_name_ciphertext/nonce`, `encrypted_data_blob/nonce` |
| `in_person_consultations` | SOAP legacy | `id TEXT PK`, `paciente_id FK`, `medico_id FK`, `s/o/a/p_ciphertext/nonce`, `digital_signature TEXT`, `is_synced INTEGER DEFAULT 0` |
| `patient_metrics` | Métricas EAV | `id TEXT PK`, `paciente_id FK`, `medico_id FK`, `metric_type TEXT`, `sub_metric TEXT`, `value_num REAL`, `value_text_ciphertext/nonce`, `digital_signature TEXT` |

### 2.2 Tablas Genéricas — Template v1 (Experimental)

> Estado: Creadas en `database.rs` con `.ok()` (tolerantes a fallos). Conviven con legacy.

| Tabla | Esquema |
|-------|---------|
| `entities` | `id INTEGER PK AUTOINCREMENT`, `external_id TEXT UNIQUE`, `entity_type TEXT`, `created_by_user_id INTEGER`, `blind_index TEXT UNIQUE`, `enc_data_blob BLOB`, `created_at TEXT` |
| `notes` | `id INTEGER PK AUTOINCREMENT`, `external_id TEXT UNIQUE`, `entity_id INTEGER FK`, `template_id TEXT`, `created_by_user_id TEXT`, `enc_fields BLOB`, `signature BLOB`, `created_at TEXT` |
| `sync_queue` | `id INTEGER PK AUTOINCREMENT`, `table_name TEXT`, `record_id INTEGER`, `operation TEXT`, `created_at TEXT` |

**Índice:** `idx_entities_type ON entities(entity_type)`.

### 2.3 IDs Duales

| Campo | Tipo | Propósito |
|-------|------|-----------|
| `id` | `INTEGER PRIMARY KEY AUTOINCREMENT` | Índice local, FKs rápidos |
| `external_id` | `TEXT UUID` | Identificador global para sincronización |

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

### 3.4 Legacy (comandos adicionales en handler)

- `save_soap_consultation`, `get_patient_history`
- `create_patient`, `get_patients_list`, `search_patients`
- `save_patient_metric`, `get_patient_metrics`

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
- `State<'_, DataKey>` — para cifrado/descifrado de datos médicos
- `State<'_, SigningState>` — para firma digital
- `State<'_, DbState>` — para acceso a SQLite
- `State<'_, SchemaConfig>` / `State<'_, NoteTemplatesConfig>` — para validación

---

## 8. Roadmap Actualizado

### ✅ Fase 3: Completada (2026-07-08)

- [x] Motor criptográfico AES-GCM-256 + Ed25519 + Blind Index
- [x] Vault local con sal única por usuario + Argon2id KDF
- [x] **DataKey compartida por instalación** para cifrado de datos médicos
- [x] **INSTALLATION_SALT persistente** para blind index
- [x] **Configuración empaquetada en binario** (`include_str!`)
- [x] 18 tests unitarios pasando (14 crypto + 4 vault)
- [x] Tablas genéricas `entities`, `notes`, `sync_queue`
- [x] Comandos genéricos de entidades y notas con firma digital
- [x] Verificación dinámica de firma por template
- [x] `auth_commands.rs` completo con `unlock_vault`/`lock_vault`
- [x] `main.rs` con setup completo: salt, data_key, db, schemas

### 🟧 Fase 4: Infraestructura y Sincronización Híbrida

- [ ] Despliegue de VPS con Dokploy y PocketBase
- [ ] Sync Engine (`tokio`): background worker polling `sync_queue`
- [ ] **Sincronización de `.data_key` y `.installation_salt` entre dispositivos** (crítico: sin esto, un dispositivo nuevo no puede descifrar datos ni buscar por blind index)
- [ ] Implementar Blind Index en servidor
- [ ] Cifrar `.data_key` en disco con llave derivada del vault del usuario (mejora de seguridad)

### 🟥 Fase 5: Teleconsulta Asíncrona (Multi-Usuario)

- [ ] Tabla `asynchronous_threads`
- [ ] Cifrado asimétrico o clave de sesión compartida

### ⬜ Fase 6: Interoperabilidad (Futuro)

- [ ] Exportación JSON y traducción a FHIR

---

## 9. Notas Técnicas Críticas

### 9.1 Sobre `DataKey` en Disco

`DataKey` se guarda en `.data_key` (hex) en el directorio de la app. **No está cifrada.** Esto es una decisión de diseño intencional para permitir descifrado sin autenticación (ej. backups, migraciones), pero implica:

- La seguridad física del disco es crítica.
- Para endurecer: cifrar `.data_key` con una llave derivada del password del usuario (pero esto rompería el acceso sin login).

### 9.2 Sobre `search_entities`

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

### 9.3 Sobre Tablas Genéricas y `.ok()`

En `database.rs`:
```rust
conn.execute("CREATE TABLE IF NOT EXISTS entities (...)", []).ok();
```

El `.ok()` silencia errores. Esto es tolerante pero **oculta problemas de esquema**. Recomendación: usar `map_err` explícito o al menos `eprintln!` para debugging.

### 9.4 Sobre `get_data_key` y Mensajes de Error

En `entity_commands.rs` y `note_commands.rs` se usa `get_data_key()` que retorna `"Bóveda cerrada"` si `DataKey` es `None`. Pero `DataKey` se inicializa en `setup()` y **nunca se limpia**. El error solo ocurriría si `setup()` falló. Considerar mensaje más preciso: "Llave de datos no inicializada".

### 9.5 Sobre `professional_profiles` en Registro

En `register_user`, se inserta un perfil profesional con campos cifrados vacíos (`''`):
```rust
INSERT INTO professional_profiles (...) VALUES (..., '', '', '', '', '', '', '', ...);
```

Esto es un placeholder. Los campos cifrados del perfil (nombre, matrícula, especialidad) deben completarse en un flujo posterior de "completar perfil".

---

*Documentación generada el 2026-07-08. Refleja el estado real del código fuente tras análisis de `main.rs`, `lib_types.rs`, `crypto.rs`, `vault.rs`, `auth.rs`, `auth_commands.rs`, `entity_commands.rs`, `note_commands.rs`, `config_schema.rs` y `database.rs`.*
