# 🧩 Plugin: Teleconsulta Asíncrona

## Simplex Health Core — Arquitectura de Plugins
### Versión 2.0 | 2026-07-09 | PatientDataKey

---

## 1. Visión General

El plugin de Teleconsulta Asíncrona permite la comunicación estructurada entre pacientes y médicos fuera de la consulta presencial. Opera como un **plugin aislado** sobre el Core de Simplex Health.

**Principios:**
- **Aislamiento de errores:** Si el plugin falla, el Core médico sigue funcionando.
- **Zero-Knowledge:** El servidor nunca descifra. Los datos viajan cifrados entre médico y paciente.
- **PatientDataKey:** Cada paciente tiene su propia llave de cifrado para datos compartidos.
- **Mismo binario:** La app Tauri tiene modo médico y modo paciente.

---

## 2. Arquitectura de Llaves

| Llave | Quién la tiene | Dónde vive | Para qué |
|-------|---------------|------------|----------|
| **DataKey** | Consultorio (médicos, admin, dispositivos) | `.data_key` en disco de cada dispositivo del consultorio | Cifrar/descifrar datos médicos en producción |
| **PatientDataKey** | Cada paciente (en su vault) | `vault_{patient_id}.bin` | Descifrar datos que el médico le comparte |
| **SigningKey** | Cada médico (en su vault) | `vault_{user_id}.bin` | Firmar notas médicas |

---

## 3. Creación del Paciente y PatientDataKey

### 3.1 Flujo

```
Médico/Admin crea paciente en Core:
  → INSERT en entities (o patients legacy)
  → Core genera PatientDataKey aleatoria: [u8; 32]
  → Core guarda PatientDataKey en tabla del Core:
      tabla: patient_keys
      { patient_id, patient_data_key_ciphertext, patient_data_key_nonce }
      (cifrado con DataKey del consultorio)
  → Core entrega credenciales al paciente:
      username: DNI
      password: temporal (generado aleatoriamente)
```

### 3.2 Tabla `patient_keys` (Core)

```sql
CREATE TABLE patient_keys (
    patient_user_id TEXT PRIMARY KEY,       -- FK a users.id (rol: paciente)
    patient_data_key_ciphertext TEXT NOT NULL,
    patient_data_key_nonce TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY(patient_user_id) REFERENCES users(id)
);
```

### 3.3 Vault del Paciente

```
vault_{patient_id}.bin (cifrado con password del paciente):
{
  "magic": "SIMPLEX_PATIENT_VAULT",
  "patient_data_key": "base64",     -- PatientDataKey de este paciente
  "installation_id": "uuid"         -- Para vincular al consultorio
}
```

### 3.4 Generación del Vault del Paciente

```rust
fn create_patient_vault(
    app_dir: &PathBuf,
    patient_user_id: &str,
    password_temporal: &str,
    patient_data_key: &[u8; 32],
) -> Result<(), String> {
    // 1. Derivar llave del password temporal con Argon2id
    let salt = SaltString::generate(&mut OsRng);
    let master_key = derive_key_from_password(password_temporal, &salt)?;

    // 2. Crear payload
    let payload = PatientVaultPayload {
        magic: "SIMPLEX_PATIENT_VAULT".to_string(),
        patient_data_key: BASE64.encode(patient_data_key),
        installation_id: get_installation_id(app_dir)?,
    };
    let json_bytes = serde_json::to_vec(&payload)?;

    // 3. Cifrar payload con master_key
    let encrypted = encrypt_vault_data(&json_bytes, &master_key)?;

    // 4. Guardar
    fs::write(app_dir.join(format!("vault_{}.bin", patient_user_id)), encrypted)?;
    fs::write(app_dir.join(format!("vault_{}.salt", patient_user_id)), salt.as_str())?;

    Ok(())
}
```

---

## 4. Compartir Datos con el Paciente

### 4.1 Flujo

```
Médico quiere compartir nota con Juan Pérez:
  → Core descifra nota con DataKey
  → Core obtiene PatientDataKey de Juan de tabla patient_keys
  → Core cifra copia de la nota con PatientDataKey de Juan
  → Core sube al servidor:
      tabla: shared_notes
      { patient_id, note_external_id, note_copy_ciphertext, note_copy_nonce }
```

### 4.2 Tabla `shared_notes` (Servidor PocketBase)

```javascript
{
  id: "uuid-v4",
  installation_id: "uuid-v4",
  patient_user_id: "uuid-v4",
  original_note_external_id: "uuid-v4",   -- FK a core.notes.external_id
  note_copy_ciphertext: "base64",         -- Cifrado con PatientDataKey
  note_copy_nonce: "base64",
  shared_by_user_id: "uuid-v4",           -- Médico que compartió
  shared_at: "ISO8601",
  expires_at: "ISO8601|null"              -- Opcional: expiración
}
```

### 4.3 Comando Core: `share_note_with_patient`

```rust
#[tauri::command]
pub fn share_note_with_patient(
    note_id: i64,
    patient_user_id: String,
    db_state: State<'_, DbState>,
    data_key_state: State<'_, DataKey>,
) -> Result<String, String> {
    let data_key = get_data_key(&data_key_state)?;

    // 1. Obtener nota del Core
    let note = get_note(note_id, &db_state, &data_key_state, ...)?;

    // 2. Obtener PatientDataKey del paciente
    let patient_data_key = with_conn(&db_state, |conn| {
        conn.query_row(
            "SELECT patient_data_key_ciphertext, patient_data_key_nonce 
             FROM patient_keys WHERE patient_user_id = ?1",
            params![&patient_user_id],
            |row| {
                let ct: String = row.get(0)?;
                let nonce: String = row.get(1)?;
                Ok((ct, nonce))
            }
        ).map_err(|e| e.to_string())
    })?;

    // 3. Descifrar PatientDataKey con DataKey
    let enc_pdk = EncryptedData {
        ciphertext: patient_data_key.0,
        nonce: patient_data_key.1,
    };
    let pdk_hex = decrypt_text(&enc_pdk, &data_key)?;
    let pdk_bytes = hex::decode(&pdk_hex).map_err(|_| "PatientDataKey corrupta")?;
    let mut patient_data_key = [0u8; 32];
    patient_data_key.copy_from_slice(&pdk_bytes);

    // 4. Cifrar copia de la nota con PatientDataKey
    let note_json = serde_json::to_string(&note.fields).map_err(|e| e.to_string())?;
    let note_copy_enc = encrypt_text(&note_json, &patient_data_key)?;

    // 5. Subir al servidor
    let external_id = Uuid::new_v4().to_string();
    upload_shared_note_to_server(
        &patient_user_id,
        &note.external_id.unwrap(),
        &note_copy_enc,
        &note.created_by_user_id,
    )?;

    Ok(external_id)
}
```

---

## 5. Estructura del Plugin

```
plugins/teleconsulta/
├── Cargo.toml
├── teleconsulta.json
├── src/
│   ├── lib.rs              # Trait Plugin + implementación
│   ├── db.rs               # Schema de teleconsulta.db
│   ├── models.rs           # Structs: Thread, Message, SharedNote
│   ├── api.rs              # API con servidor PocketBase
│   ├── notifications.rs    # Push notifications
│   ├── ipc.rs              # Comunicación con Core
│   └── web/                # Portal web (futuro, opcional)
│       ├── index.html
│       ├── app.js
│       └── styles.css
└── migrations/
    └── 001_initial.sql
```

---

## 6. Base de Datos: teleconsulta.db

### 6.1 Tablas

```sql
-- Threads de conversación
CREATE TABLE threads (
    id TEXT PRIMARY KEY,
    patient_user_id TEXT NOT NULL,
    assigned_to_user_id TEXT,
    entity_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('open', 'waiting_patient', 'waiting_doctor', 'closed')),
    subject TEXT,
    created_at TEXT NOT NULL,
    closed_at TEXT,
    last_message_at TEXT,
    is_synced INTEGER DEFAULT 0
);

-- Mensajes dentro de un thread
CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    thread_id TEXT NOT NULL,
    sender_type TEXT NOT NULL CHECK(sender_type IN ('patient', 'doctor')),
    sender_user_id TEXT NOT NULL,
    message_type TEXT NOT NULL CHECK(message_type IN ('text', 'image', 'file', 'note_reference')),
    content_ciphertext TEXT,
    content_nonce TEXT,
    note_external_id TEXT,
    created_at TEXT NOT NULL,
    is_synced INTEGER DEFAULT 0
);

-- Notas compartidas descargadas (modo paciente)
CREATE TABLE shared_notes_local (
    id TEXT PRIMARY KEY,
    original_note_external_id TEXT NOT NULL,
    note_copy_ciphertext TEXT NOT NULL,
    note_copy_nonce TEXT NOT NULL,
    shared_by_user_id TEXT NOT NULL,
    shared_at TEXT NOT NULL,
    downloaded_at TEXT NOT NULL
);

-- Índices
CREATE INDEX idx_threads_patient ON threads(patient_user_id);
CREATE INDEX idx_threads_doctor ON threads(assigned_to_user_id);
CREATE INDEX idx_threads_status ON threads(status);
CREATE INDEX idx_messages_thread ON messages(thread_id);
```

---

## 7. Modos de la App

### 7.1 Modo Médico (Core + Plugin)

| Función | Implementación |
|---------|----------------|
| Ver threads | Plugin: query a teleconsulta.db |
| Responder mensaje | Plugin: cifra con DataKey (vía Core IPC), guarda en teleconsulta.db |
| Crear nota formal | Core: `create_note` (cifrado + firma Ed25519) |
| Compartir nota con paciente | Core: `share_note_with_patient` (cifra con PatientDataKey) |
| Ver nota compartida | Core: descifra con DataKey |

### 7.2 Modo Paciente (Plugin)

| Función | Implementación |
|---------|----------------|
| Login | DNI + password → desbloquea vault → obtiene PatientDataKey |
| Ver threads | Plugin: descarga del servidor, muestra |
| Enviar mensaje | Plugin: envía texto plano al servidor, servidor cifra con DataKey para médico |
| Ver notas compartidas | Plugin: descarga del servidor, descifra con PatientDataKey |
| Ver nota formal | Plugin: llama Core IPC `decrypt_with_patient_key` |

---

## 8. Flujos de Uso

### 8.1 Paciente inicia teleconsulta

```
Paciente (modo paciente):
  → Login con DNI + password
  → Desbloquea vault → PatientDataKey en RAM
  → "Nueva consulta"
  → Selecciona médico o especialidad
  → Escribe: "Dolor de cabeza intenso desde ayer"
  → Enviar

App (modo paciente):
  → POST /patient/thread
  {
    patient_user_id: "...",
    subject: "Dolor de cabeza",
    message: "Dolor de cabeza intenso desde ayer",
    // Mensaje en texto plano, servidor lo cifra con DataKey
  }

Servidor:
  → Crea thread
  → Cifra mensaje con DataKey del consultorio
  → Guarda en teleconsulta.db del servidor
  → Notificación push al médico
```

### 8.2 Médico responde

```
Médico (modo médico):
  → Notificación: "Nueva consulta de Juan Pérez"
  → Abre thread
  → Plugin descifra mensaje con DataKey (vía Core IPC)
  → Responde: "Tome paracetamol y avise en 24hs"
  → Enviar

App (modo médico):
  → Plugin cifra mensaje con DataKey (vía Core IPC)
  → Guarda en teleconsulta.db local
  → Inserta en sync_queue

Sync Engine (Core):
  → Push mensaje al servidor

Servidor:
  → Notificación push al paciente
```

### 8.3 Médico crea nota y comparte

```
Médico:
  → En thread, click "Crear nota médica"
  → Se abre formulario SOAP (Core)
  → Redacta y guarda
  → Core: create_note (cifrado con DataKey, firmado con SigningKey)
  → Click "Compartir con paciente"
  → Core: share_note_with_patient
      1. Descifra nota con DataKey
      2. Obtiene PatientDataKey de Juan
      3. Cifra copia con PatientDataKey
      4. Sube a servidor

Paciente:
  → Notificación: "El Dr. García compartió una nota"
  → Abre nota
  → App descarga del servidor
  → Descifra con PatientDataKey (del vault)
  → Ve nota SOAP completa
```

### 8.4 Paciente ve nota compartida

```
Paciente (modo paciente):
  → Login, desbloquea vault
  → "Mis notas"
  → Descarga shared_notes del servidor
  → Para cada nota:
      descifra note_copy_ciphertext con PatientDataKey
      muestra contenido
  → Ve: "Consulta del 2026-07-09 - Dr. García"
      Subjetivo: "..."
      Objetivo: "..."
      Análisis: "..."
      Plan: "..."
```

---

## 9. IPC con el Core

### 9.1 Comandos del Plugin al Core

| Comando | Parámetros | Retorno | Uso |
|---------|-----------|---------|-----|
| `encrypt_with_datakey` | `plaintext: String` | `EncryptedData` | Médico envía mensaje |
| `decrypt_with_datakey` | `ciphertext, nonce` | `String` | Médico lee mensaje |
| `get_patient_datakey` | `patient_user_id: String` | `[u8; 32]` | Compartir nota con paciente |
| `decrypt_with_patient_key` | `ciphertext, nonce, patient_user_id` | `String` | Paciente lee nota compartida |
| `create_medical_note` | `entity_id, template_id, fields, user_id` | `note_external_id` | Médico crea nota formal |
| `get_medical_note` | `note_external_id: String` | `NoteRecord` | Ver nota formal |

### 9.2 Ejemplo: Paciente descifra nota compartida

```rust
// En Plugin (modo paciente)
async fn view_shared_note(
    &self,
    shared_note_id: &str,
    patient_data_key: &[u8; 32],
) -> Result<NoteFields, String> {
    // 1. Descargar del servidor
    let shared = self.api.get_shared_note(shared_note_id).await?;

    // 2. Descifrar con PatientDataKey
    let enc = EncryptedData {
        ciphertext: shared.note_copy_ciphertext,
        nonce: shared.note_copy_nonce,
    };
    let note_json = decrypt_text(&enc, patient_data_key)?;

    // 3. Parsear
    let fields: NoteFields = serde_json::from_str(&note_json)
        .map_err(|e| format!("Error parseando nota: {}", e))?;

    Ok(fields)
}
```

---

## 10. Sync del Plugin

### 10.1 Mecanismo

El plugin usa el Sync Engine del Core con su propia cola:

```sql
-- teleconsulta.db
CREATE TABLE plugin_sync_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    table_name TEXT NOT NULL,
    record_id TEXT NOT NULL,
    operation TEXT NOT NULL,
    created_at TEXT NOT NULL
);
```

El Sync Engine del Core lee `plugin_sync_queue` y empuja al servidor junto con sus propios datos.

### 10.2 Colecciones en Servidor

```javascript
// sync_threads
{
  id: "uuid-v4",
  installation_id: "uuid-v4",
  patient_user_id: "uuid-v4",
  assigned_to_user_id: "uuid-v4|null",
  entity_id: "uuid-v4",
  status: "open|waiting_patient|waiting_doctor|closed",
  subject: "Dolor de cabeza",
  created_at: "ISO8601",
  closed_at: "ISO8601|null",
  last_message_at: "ISO8601"
}

// sync_messages
{
  id: "uuid-v4",
  installation_id: "uuid-v4",
  thread_id: "uuid-v4",
  sender_type: "patient|doctor",
  sender_user_id: "uuid-v4",
  message_type: "text|image|file|note_reference",
  content_ciphertext: "base64",       -- Cifrado con DataKey
  content_nonce: "base64",
  note_external_id: "uuid-v4|null",
  created_at: "ISO8601"
}

// sync_shared_notes
{
  id: "uuid-v4",
  installation_id: "uuid-v4",
  patient_user_id: "uuid-v4",
  original_note_external_id: "uuid-v4",
  note_copy_ciphertext: "base64",       -- Cifrado con PatientDataKey
  note_copy_nonce: "base64",
  shared_by_user_id: "uuid-v4",
  shared_at: "ISO8601",
  expires_at: "ISO8601|null"
}
```

---

## 11. Seguridad

### 11.1 Lo que el plugin NUNCA hace

| Operación | Quién la hace | Por qué el plugin no la hace |
|-----------|---------------|------------------------------|
| Firmar nota médica | **Core** (SigningState) | Plugin no tiene llave privada Ed25519 |
| Cifrar con DataKey directamente | **Core** (DataKey) | Plugin accede vía IPC |
| Crear nota médica formal | **Core** (`create_note`) | Plugin llama al Core |
| Verificar firma de nota | **Core** (`verify_signature`) | Plugin llama al Core |

### 11.2 Lo que el plugin SÍ hace

| Operación | Cómo |
|-----------|------|
| Crear thread | Inserta en teleconsulta.db |
| Enviar mensaje (modo médico) | Cifra con DataKey vía Core IPC |
| Enviar mensaje (modo paciente) | Envía texto plano al servidor |
| Descifrar nota compartida (modo paciente) | Descifra con PatientDataKey del vault |
| Ver nota formal | Llama Core IPC `get_medical_note` |

### 11.3 Modelo de amenazas

| Amenaza | Mitigación |
|---------|-----------|
| Plugin comprometido | Solo accede a teleconsulta.db. No tiene DataKey ni SigningKey. |
| Servidor comprometido | Tiene blobs cifrados con DataKey y PatientDataKey. Sin llaves, no descifra. |
| Paciente ve datos de otro paciente | Queries filtradas por patient_user_id. Imposible sin autenticación. |
| Paciente modifica nota médica | Imposible. Las notas solo las crea el Core. |
| Robo de vault del paciente | El ladrón necesita el password para desbloquear. |

---

## 12. Configuración

```json
// plugins/teleconsulta/teleconsulta.json
{
  "id": "teleconsulta",
  "name": "Teleconsulta Asíncrona",
  "version": "1.0.0",
  "active": true,
  "category": "communication",
  "config": {
    "server_url": "https://api.simplexhealth.io",
    "sync_interval_seconds": 30,
    "push_notifications": {
      "enabled": true,
      "provider": "firebase"
    },
    "patient_mode": {
      "enabled": true,
      "vault_encryption": "argon2id_aes256gcm"
    }
  }
}
```

---

## 13. Dependencias

```toml
# plugins/teleconsulta/Cargo.toml
[package]
name = "teleconsulta-plugin"
version = "1.0.0"
edition = "2021"

[dependencies]
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
rusqlite = { version = "0.32", features = ["bundled", "chrono"] }
reqwest = { version = "0.12", features = ["json", "multipart"] }
tokio = { version = "1", features = ["full"] }
chrono = "0.4"
uuid = { version = "1.0", features = ["v4", "serde"] }

# Core IPC
simplex-core = { path = "../../src-tauri" }
```

---

## 14. Checklist de Implementación

- [ ] Trait `Plugin` base en Core
- [ ] Tabla `patient_keys` en Core
- [ ] Comando `share_note_with_patient` en Core
- [ ] Generación de vault del paciente
- [ ] Estructura del plugin
- [ ] Schema `teleconsulta.db`
- [ ] IPC: `encrypt_with_datakey`, `decrypt_with_datakey`, `get_patient_datakey`
- [ ] IPC: `decrypt_with_patient_key`, `create_medical_note`, `get_medical_note`
- [ ] API con servidor PocketBase
- [ ] Sync del plugin
- [ ] Modo paciente en app Tauri
- [ ] Modo médico: integración con Core
- [ ] Notificaciones push
- [ ] Tests de integración

---

*Documento generado el 2026-07-09. Arquitectura validada: PatientDataKey, Zero-Knowledge, modo médico/modo paciente en mismo binario.*
