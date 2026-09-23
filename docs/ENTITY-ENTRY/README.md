# 🩺 Simplex Health Core — Historia Clínica Digital (Entity-Entry Paradigm)

Aplicación de escritorio nativa orientada a la gestión de historias clínicas multi-dominio (medicina humana, veterinaria, odontología) con un enfoque **Local-First**, arquitectura de seguridad **Zero-Knowledge** (Cifrado de Extremo a Extremo) y **Máxima Simplicidad Relacional**.

El sistema garantiza la máxima privacidad procesando y cifrando toda la información médica sensible directamente en la RAM del hardware local antes de persistirla en el disco o sincronizarla. La persistencia central del Core se rige por un principio purista: **indestructible, inmutable y reducida a solo dos conceptos universales (`Entity` y `Entry`)**.

---

## ⚖️ Las Reglas Estrictas del Core (Inquebrantables)

### 1. Inmutabilidad Absoluta de las Entradas (`Append-Only`)
* **Regla:** Ninguna fila de la tabla `entries` de la base de datos del Core puede ser editada (`UPDATE`) ni eliminada (`DELETE`) bajo ninguna circunstancia operativa clínica.
* **Manejo de Errores:** Si un profesional comete un error en una nota SOAP, una medicación o una alergia, el error **no se borra**. Se genera una **nueva `Entry`** de enmienda o corrección que se enlaza al ID del registro erróneo mediante su payload. La base de datos es una línea de tiempo acumulativa de verdades históricas.

### 2. Aislamiento Total de Plugins (Frontera Rígida de Datos)
* **Regla:** Las tablas `entities` y `entries` pertenecen **exclusivamente** al Core y solo almacenan actores del sistema y hechos médicos duros/validados.
* **Manejo de Datos de Plugins:** Cualquier funcionalidad operativa extendida (Agenda, Telemedicina, Facturación, Gestión de Stock) tiene terminantemente prohibido crear tablas dentro de `core_clinical.db`. Cada plugin debe inicializar y gestionar **su propio archivo SQLite independiente** (ej: `scheduler.db`, `telemed.db`).
* **Flujo de Interacción:** Un plugin de telemedicina gestiona el chat informal (textos, audios, adjuntos) pura y exclusivamente dentro de su propia base de datos (`telemed.db`). Al finalizar la teleconsulta, el plugin extrae el resumen relevante y obliga al profesional a guardar una única `Entry` inmutable tipo `SOAP_NOTE` en el Core. El Core jamás almacena logs de mensajería informal.

### 3. Soberanía Criptográfica Absoluta (Zero-Knowledge)
* **Regla:** El desarrollador/servidor central no posee, no conoce y no puede recuperar jamás la llave de acceso a los datos de ninguna instalación.
* **Frase Semilla Local:** Cada instalación genera una única clave global a partir de una frase semilla de **3 o 6 palabras** (estándar mnemónico). Esta frase es la llave maestra para descifrar el archivo local mediante **SQLCipher (AES-256)**.
* **Responsabilidad del Usuario:** Se entrena explícitamente al cliente para que grabe estas palabras en una **placa de metal** física (resistente a incendios/inundaciones). Si la computadora se rompe y el usuario pierde el metal, los datos son matemáticamente irrecuperables. No existe el soporte de "recuperar contraseña".

---

## ⚡ Stack Tecnológico

### Backend Nativo (Core - Rust)
- **Rust**: Motor principal encargado de la criptografía, seguridad y acceso a datos.
- **Tauri v2**: Puente IPC multiplataforma de consumo ultra-bajo.
- **SQLite (`rusqlite`)**: Base de datos relacional embebida operando bajo el patrón `with_conn`.
- **AES-GCM-256**: Cifrado autenticado en RAM para payloads médicos y atributos sensibles.
- **Ed25519 (Dalek)**: Firma digital asimétrica para garantizar el No-Repudio de las `Entries`.
- **SHA-256**: Hashing determinista para Índices Ciegos (*Blind Indexes*) de búsqueda exacta.
- **Argon2id**: Hashing de contraseñas locales y derivación de claves de bóveda (KDF).

### Frontend (UI)
- **React 19 + TypeScript (Strict)**: Interfaz declarativa con tipado extremo a extremo.
- **Zustand**: Gestor de estado global ultraligero (`usePatientStore`, `useAuthStore`).
- **Tailwind CSS v4** & **Vite 7** & **pnpm**.

---

## 🗄️ Estructura de la Base de Datos Central Refactorizada (`core_clinical.db`)

El Core elimina el esquema multi-tabla acoplado (`users`, `notes`, `medical_history`, `professional_profiles`) y lo unifica en dos tablas polimórficas donde el tipado estructural variable lo maneja el motor JSON dinámico.

```sql
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL; -- Modo WAL activo para concurrencia segura local [2]

-- 1. Tabla unificada de Actores (Pacientes, Médicos, Clínicas)
CREATE TABLE IF NOT EXISTS entities (
    id TEXT PRIMARY KEY NOT NULL,          -- UUIDv4 generado en el cliente
    type TEXT NOT NULL CHECK(type IN ('PATIENT', 'USER', 'CLINIC')),
    name TEXT NOT NULL,                    -- Nombre legible en claro para búsquedas rápidas/UI
    attributes BLOB NOT NULL,              -- JSON (HashMap<String, Value>) cifrado con AES-GCM-256 [7]
    blind_index TEXT UNIQUE,               -- Hash SHA-256 para búsquedas exactas (ej: DNI o Microchip) [7]
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 2. Tabla unificada de Hechos Clínicos Inmutables (SOAP, Alergias, Antecedentes, Medicación)
CREATE TABLE IF NOT EXISTS entries (
    id TEXT PRIMARY KEY NOT NULL,          -- UUIDv4
    category TEXT NOT NULL CHECK(category IN ('SOAP_NOTE', 'MEDICATION', 'ALLERGY', 'CONDITION')),
    subject_id TEXT NOT NULL,              -- FK al Paciente (Entity)
    author_id TEXT NOT NULL,               -- FK al Usuario Profesional/Firmante (Entity)
    title TEXT NOT NULL,                   -- Resumen legible (ej: "Alergia a Penicilina", "Enalapril 10mg")
    status TEXT NOT NULL CHECK(status IN ('ACTIVE', 'RESOLVED', 'COMPLETED')),
    timestamp TEXT NOT NULL,               -- Fecha y hora del hecho médico
    payload BLOB NOT NULL,                 -- JSON estructurado cifrado con AES-GCM-256 [7]
    signature TEXT NOT NULL,               -- Firma criptográfica Ed25519 del autor [7]
    updated_at TEXT NOT NULL,              -- Control de tiempo de inserción local
    is_synced INTEGER NOT NULL DEFAULT 0 CHECK(is_synced IN (0, 1)),
    
    FOREIGN KEY (subject_id) REFERENCES entities(id) ON DELETE RESTRICT,
    FOREIGN KEY (author_id) REFERENCES entities(id) ON DELETE RESTRICT
);

-- Índices Críticos de Rendimiento Local
CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(type);
CREATE INDEX IF NOT EXISTS idx_entries_timeline ON entries(subject_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_entries_sync_queue ON entries(is_synced) WHERE is_synced = 0;
```

---

## 🏗️ Refactorización de la Arquitectura de Templates (v1)

Tus tres archivos de configuración (`schema.json`, `medical_history_schema.json` y `soap.json`) se integran al Core sin alterar el motor de base de datos:

| Archivo de Configuración | Categoría en DB | Mecanismo de Datos en el Core |
|---|---|---|
| `config/schema.json` [7] | `entities.type = 'PATIENT'` | Los campos mutables de filiación se validan y se guardan cifrados en bloque dentro de `attributes`. El campo `dni` genera el `blind_index`. |
| `config/medical_history_schema.json` [7] | `entries.category = 'ALLERGY' \| 'MEDICATION' \| 'CONDITION'` | **CAMBIO CRÍTICO**: Cada antecedente del formulario (ej: una alergia a la penicilina o un fármaco diario) ya no actualiza una fila mutable. Se inserta como una `Entry` individual, inmutable e indexada cronológicamente. |
| `config/note_templates/soap.json` [7] | `entries.category = 'SOAP_NOTE'` | El payload con las secciones S-O-A-P se valida, se serializa según el `signature_payload_order`, se firma con Ed25519 y se persiste cifrado. |

---

## 🔒 Especificación de Seguridad (Zero-Knowledge & Vault)

### Modelo de Cifrado por Capas Actualizado

| Capa | Ubicación en DB | Llave | Propósito |
|---|---|---|---|
| Filiación | `entities.attributes` | `DataKey` en RAM | Atributos demográficos dinámicos cifrados en bloque. |
| Eventos Clínicos | `entries.payload` | `DataKey` en RAM | Bloques SOAP, dosis o descripciones de antecedentes cifrados de forma inmutable. |
| Vault | Archivo `vault_{user_id}.bin` | Master Key (derivada) | Llave privada de firma Ed25519 protegida en reposo. |

---

## 📂 Nueva Estructura del Backend (Fase de Refactorización)

El mapa de módulos se simplifica al remover los controladores de tablas acopladas:

```text
src-tauri/src/
 ├── main.rs              
 ├── lib.rs               # Inyección de estados globales (DbState, CryptoState, SigningState).
 ├── lib_types.rs         
 ├── config_schema.rs     # Structs para parsear las configuraciones dinámicas.
 ├── vault.rs             # Gestión de Bóvedas criptográficas por usuario.
 ├── auth.rs              # Autenticación Argon2id.
 ├── crypto.rs            # AES-GCM, Ed25519, SHA-256 Blind Index.
 ├── database.rs          # Inicialización de 'core_clinical.db' (Solo 2 tablas).
 └── commands/            
     ├── mod.rs           # Helpers nativos (`with_conn`, `get_data_key`).
     ├── auth_commands.rs 
     ├── entity_commands.rs    # CRUD dinámico de actores (Pacientes, Usuarios).
     └── entry_commands.rs     # ← NUEVO: Controlador unificado append-only para Notas SOAP y Antecedentes.
```
