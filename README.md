# 🩺 Simplex Health Core — Historia Clínica Digital

Aplicación de escritorio nativa orientada a la gestión de historias clínicas y telemedicina asíncrona con un enfoque **Local-First** y arquitectura de seguridad **Zero-Knowledge** (Cifrado de Extremo a Extremo).

El sistema garantiza la máxima privacidad procesando y cifrando toda la información médica sensible directamente en la RAM del hardware local antes de persistirla en el disco o sincronizarla. El servidor remoto actuará en el futuro únicamente como un "tubo ciego" que almacena texto cifrado, sin capacidad de leer la información.

## ⚡ Stack Tecnológico

La arquitectura está dividida de forma estricta en dos capas de alto rendimiento:

### Backend Nativo (Core - Rust)
- **Rust**: Motor principal encargado de la criptografía, seguridad y acceso a datos.
- **Tauri v2**: Puente IPC multiplataforma de consumo ultra-bajo.
- **SQLite (`rusqlite`)**: Base de datos relacional embebida (acceso directo sin plugins de Tauri para mantener el patrón `with_conn`), garantizando autonomía total offline.
- **AES-GCM-256**: Cifrado autenticado para texto médico (Campos SOAP, Nombres, Antecedentes).
- **Ed25519 (Dalek)**: Firma digital asimétrica para garantizar el No-Repudio y validez legal de las notas médicas.
- **SHA-256**: Hashing determinista para Índices Ciegos y huellas digitales de documentos.
- **Argon2id**: Hashing de contraseñas y derivación de claves (KDF).

### Frontend (UI)
- **React 19 + TypeScript (Strict)**: Interfaz declarativa con tipado extremo a extremo (los structs de Rust se espejan en TS).
- **Zustand**: Gestor de estado global ultraligero. Maneja sesiones de seguridad y datos clínicos sin boilerplate ni props-drilling.
- **Tailwind CSS v4**: Framework de utilidades CSS para diseño rápido, consistente y responsivo.
- **Vite 7**: Entorno de compilación rápida con HMR (Hot Module Replacement).
- **pnpm**: Gestor de paquetes eficiente mediante hard links.

## 📂 Arquitectura del Proyecto

### Backend Core (Rust)
Diseño modular estricto para evitar dependencias circulares. El acceso a la base de datos y a la criptografía está aislado mediante patrones de diseño avanzados.

```text
src-tauri/src/
 ├── main.rs              # Punto de entrada al ejecutable nativo.
 ├── lib.rs               # Orquestador de Tauri, inyección de estados globales (DbState, CryptoState, SigningState, DataKey).
 ├── lib_types.rs         # Contenedores de estado seguro (Mutex<Option<T>>).
 │
 ├── config_schema.rs     # Structs para parsear schema.json, note_templates y medical_history_schema dinámicamente.
 ├── vault.rs             # Gestión de Bóvedas criptográficas por usuario (Cold Start + Llave Privada de Firma).
 ├── auth.rs              # Hashing Argon2id para autenticación de usuarios.
 ├── crypto.rs            # Motor matemático puro (AES-GCM, Ed25519, Nonces OsRng, SHA-256 Blind Index).
 ├── database.rs          # Inicialización física de SQLite (WAL Mode activado) y esquemas relacionales.
 │
 └── commands/            # Controladores de API interna (Aduana de peticiones de React).
     ├── mod.rs           # Exportación plana y Helpers (`with_conn`, `get_data_key`).
     ├── auth_commands.rs # Endpoints de registro, login y desbloqueo de bóveda.
     ├── entity_commands.rs    # Endpoints genéricos de entidades (Template v1).
     ├── note_commands.rs      # Endpoints genéricos de notas con firma digital (Template v1).
     ├── medical_history_commands.rs # ← NUEVO: Endpoints dedicados para antecedentes clínicos mutables.
     └── professional_profile_commands.rs # Endpoints de perfil profesional.
```

### Frontend UI (React)
Implementación de un Design System propio basado en componentes atómicos, completamente desacoplados del estado de la aplicación mediante Zustand.

```text
src/
 ├── stores/                 # Capa de Estado Global (Zustand)
 │   ├── useAuthStore.ts     # Estado de la Bóveda (Cold Start UI, cierre seguro de sesión).
 │   ├── useEntityStore.ts   # CRUD genérico de entidades (listado, búsqueda, creación).
 │   ├── useNoteStore.ts     # Evoluciones clínicas con firma digital.
 │   └── usePatientStore.ts  # ← NUEVO: Fuente única de verdad para paciente seleccionado + antecedentes clínicos.
 │
 ├── config/
 │   ├── schema.json              # Definición de entidades (solo filiación en patient).
 │   ├── medical_history_schema.json # ← NUEVO: Definición dedicada de antecedentes clínicos.
 │   └── note_templates/soap.json # Template de notas SOAP con firma.
 │
 ├── components/
 │   ├── layouts/
 │   │   └── DashboardLayout.tsx  # Logout seguro, limpia todos los stores.
 │   ├── ui/                 # Design System Atómico
 │   │   ├── Button.tsx, Card.tsx, Input.tsx, Navbar.tsx, Textarea.tsx, Alert.tsx
 │   ├── views/
 │   │   ├── AdminView.tsx
 │   │   ├── MedicoView.tsx  # Tabs unificados (EHR, Paciente, Perfil).
 │   │   └── PacienteView.tsx
 │   │
 │   ├── PatientEhrView.tsx  # Interfaz clínica unificada (consume usePatientStore).
 │   ├── Entity.tsx          # Sandbox de admisión con panel dual (consume usePatientStore).
 │   ├── MedicalHistoryForm.tsx # ← NUEVO: Formulario dedicado de antecedentes (consume usePatientStore).
 │   ├── ProfileView.tsx
 │   └── AuthBox.tsx
```

## Principios de Diseño del Core & Frontend

- **Patrón `with_conn`**: Ningún comando saca la conexión a la base de datos de su `Mutex`. Se le inyecta un closure para garantizar que el lock se libere instantáneamente, evitando congelamientos de UI.
- **Inyección de Dependencias**: Los comandos que requieren cifrado reciben `State<'_, DataKey>` o `State<'_, SigningState>`, extraen la llave y fallan de forma segura si no está inicializada.
- **Estado Reactivo Seguro (Zustand)**: El Frontend jamás almacena contraseñas ni claves de cifrado en JavaScript. Los Stores solo contienen booleanos de estado y datos descifrados para renderizar.
- **Store Unificado de Paciente**: `usePatientStore` es la fuente única de verdad para la selección de paciente. `PatientEhrView`, `Entity` y `MedicalHistoryForm` consumen el mismo estado, garantizando sincronización instantánea sin props-drilling.
- **Optimistic UI**: Las inserciones y actualizaciones de antecedentes se reflejan en el Store al instante sin esperar un nuevo `SELECT` de SQLite, logrando tiempos de respuesta de 0ms en la UI.
- **Separación de Datos Clínicos**: Los antecedentes clínicos viven en su propia tabla (`medical_history`) con schema dedicado, separados de la filiación (`entities`) y de las notas evolutivas (`notes`).
- **Flujo Unificado de EHR**: La interfaz `PatientEhrView` consolida búsqueda, ficha, antecedentes y evoluciones en una sola vista, eliminando el context-switching entre tabs.
- **Limpieza de Datos Sensibles**: El frontend nunca retiene la contraseña en estado de React. Se limpia inmediatamente después de `unlock_vault`.

## 🔒 Especificación de Seguridad (Zero-Knowledge & Vault)

### 1. Ciclo de Vida de las Claves (Cold Start)
El sistema no utiliza claves hardcodeadas. Las claves nacen y mueren en la RAM:
1. El usuario ingresa su contraseña en la UI.
2. React llama a `unlock_vault`. Rust verifica el hash en la tabla `users` (Argon2id).
3. La misma contraseña se usa como semilla para derivar 32 bytes mediante Argon2id en modo KDF (Llave Maestra). La sal del KDF es única por usuario (`vault_{user_id}.salt`).
4. Se desbloquea el archivo `vault_{user_id}.bin`, el cual contiene la Llave Privada de Firma (Ed25519) cifrada con la Llave Maestra.
5. Ambas llaves se inyectan en el `CryptoState` y `SigningState` (RAM volátil de Rust).
6. Al cerrar sesión, React llama a `lock_vault`. Rust destruye los `Option` de la RAM. Las claves dejan de existir.

### 2. Aislamiento Multi-Usuario (Vaults)
Cada médico tiene su propio archivo de bóveda (`vault_{user_id}.bin`) y su propia sal de KDF (`vault_{user_id}.salt`). Las notas cifradas por un médico no pueden ser descifradas por otro sin compartir explícitamente las claves de entidad.

### 3. Modelo de Cifrado por Capas
| Capa | Tabla/Archivo | Llave | Propósito |
|---|---|---|---|
| Filiación | `entities.enc_data_blob` | `DataKey` | Datos demográficos cifrados campo a campo |
| Antecedentes | `medical_history.enc_fields` | `DataKey` | Datos clínicos mutables cifrados campo a campo |
| Notas SOAP | `notes.enc_fields` | `DataKey` | Evoluciones clínicas cifradas + firma Ed25519 |
| Perfil Profesional | `professional_profiles.*_ciphertext` | `DataKey` | Datos del médico cifrados individualmente |
| Vault | `vault_{user_id}.bin` | Master Key (derivada) | Llave privada de firma cifrada |

### 4. Blind Index — Búsqueda Sin Descifrado
Hash determinista SHA-256 (DNI normalizado + sal de instalación) para búsqueda exacta sin exponer el dato real. La sal es global a la instalación (`/.installation_salt`) para permitir búsqueda compartida entre médicos de la misma máquina.

### 5. Firma Digital y No-Repudio
Cada nota SOAP se firma con Ed25519 usando la llave privada del médico en RAM. El template define `signature_payload_order` para garantizar consistencia en la verificación. La llave pública se almacena en `professional_profiles` para verificación futura.

## 🗄️ Estructura de la Base de Datos

### Tablas Principales
| Tabla | Propósito | Relación |
|---|---|---|
| `users` | Autenticación (Médicos, Admins, Pacientes) | PK `id TEXT` |
| `professional_profiles` | Perfil profesional + llave pública Ed25519 | FK → `users(id)` |
| `entities` | Entidades genéricas (filiación de pacientes) | PK `id INTEGER` |
| `medical_history` | ← NUEVO: Antecedentes clínicos mutables | FK → `entities(id)`, UNIQUE por entidad |
| `notes` | Notas clínicas con firma digital | FK → `entities(id)`, FK → `users(id)` |
| `entity_keys` | Claves de datos por entidad-usuario (telemedicina) | FK → `entities(id)` |
| `sync_queue` | Cola FIFO de sincronización | Reemplaza `is_synced` |

### Tabla `medical_history` (Nueva)
```sql
CREATE TABLE medical_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    external_id TEXT UNIQUE NOT NULL,
    entity_id INTEGER NOT NULL,
    created_by_user_id TEXT NOT NULL,
    enc_fields BLOB NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(entity_id) REFERENCES entities(id),
    UNIQUE(entity_id)  -- Un solo registro mutable por paciente
);
```

## 🏗️ Arquitectura de Template (v1)

*"El core no sabe si el paciente es humano o animal. Sabe que hay entidades, atributos, relaciones y cifrado. El resto es configuración."*

### Archivos de Configuración Empaquetados
| Archivo | Propósito | Consumidor |
|---|---|---|
| `config/schema.json` | Define entidades y campos de filiación | `entity_commands.rs` + `Entity.tsx` |
| `config/medical_history_schema.json` | ← NUEVO: Define campos de antecedentes clínicos | `medical_history_commands.rs` + `MedicalHistoryForm.tsx` |
| `config/note_templates/soap.json` | Define estructura y orden de firma de notas SOAP | `note_commands.rs` + `useNoteStore.ts` |

Todos se empaquetan en el binario con `include_str!` en `lib.rs`, garantizando funcionamiento offline total e inmutabilidad en runtime.

## 🧪 Tests Automatizados

El Core cuenta con 18 tests unitarios que cubren:
| Módulo | Tests | Cobertura |
|---|---|---|
| `crypto.rs` | 14 | AES-GCM roundtrip, nonces únicos, clave incorrecta, manipulación, Blind Index, Firma Ed25519 |
| `vault.rs` | 4 | Creación, desbloqueo, contraseña incorrecta, sal única por usuario |

```bash
cargo test  # 18 passed; 0 failed
```

## 🗺️ Mapa de Ruta del Desarrollo (Roadmap)

### 🟩 Fase 1: Almacenamiento Local y Autenticación (✅ Completado)
- [x] Entorno Tauri v2 + pnpm + React + Tailwind CSS
- [x] Sistema de Vault Local con sal única por usuario
- [x] Refactorización a módulos con patrón `with_conn`

### 🟩 Fase 2: Robustez del Core Local (✅ Completado)
- [x] Índice Ciego (Blind Index) con sal derivada
- [x] Buscador Relacional Clínico
- [x] Creación de Design System propio (`/ui`)

### 🟩 Fase 3: Métricas, Firmas Digitales y Arquitectura Frontend (✅ Completado)
- [x] Cifrado selectivo (Numéricas en claro, notas cifradas)
- [x] Migración del estado a Zustand
- [x] Implementación de Optimistic UI y tipado estricto E2E
- [x] Firma Digital Ed25519 para notas (No-Repudio)
- [x] Arquitectura de Template v1 con tablas genéricas
- [x] Comandos genéricos de entidades y notas
- [x] IDs duales: `id` local + `external_id` UUID
- [x] Interfaz clínica unificada (`PatientEhrView`)
- [x] 18 tests unitarios pasando

### 🟩 Fase 3.5: Separación de Datos Clínicos (✅ Completado)
- [x] Tabla dedicada `medical_history` con UNIQUE por paciente
- [x] Schema separado `medical_history_schema.json`
- [x] Comandos `upsert_medical_history` + `get_medical_history`
- [x] Store unificado `usePatientStore` (paciente + clinical compuesto)
- [x] `MedicalHistoryForm` desacoplado con optimistic update
- [x] `Entity.tsx` con panel dual sincronizado al store global
- [x] `PatientEhrView` consume antecedentes desde `selected.clinical`
- [x] Limpieza de `schema.json` (solo filiación en `patient`)

### 🟧 Fase 4: Infraestructura y Sincronización Híbrida (Próximo paso)
- [ ] Despliegue de VPS con Dokploy y PocketBase
- [ ] Sync Engine (`tokio`): background worker polling `sync_queue`
- [ ] Sincronización de `.data_key` y `.installation_salt` entre dispositivos
- [ ] Implementar Blind Index en servidor
- [ ] Cifrar `.data_key` en disco con llave derivada del vault

### 🟥 Fase 5: Teleconsulta Asíncrona (Multi-Usuario)
- [ ] Tabla `asynchronous_threads` y linkage paciente-usuario
- [ ] Cifrado asimétrico o clave de sesión compartida
- [ ] Flujo de compartir `entity_keys` entre médicos

### ⬜ Fase 6: Interoperabilidad (Futuro)
- [ ] Motor de exportación JSON y traducción a estándares FHIR (IA-Driven)

## 🚀 Comandos Útiles de Desarrollo

```bash
pnpm tauri dev      # Levanta el entorno (Rust + React HMR)
pnpm tauri build    # Compila en modo release para producción
cargo test          # Ejecuta los 18 tests unitarios del Core
cargo add <crate>   # Añade dependencias al backend (desde src-tauri)
```
