# 🩺 Simplex Health Core — Historia Clínica Digital

Aplicación de escritorio nativa orientada a la gestión de historias clínicas y telemedicina asíncrona con un enfoque **Local-First** y arquitectura de seguridad **Zero-Knowledge** (Cifrado de Extremo a Extremo).

El sistema garantiza la máxima privacidad procesando y cifrando toda la información médica sensible directamente en la RAM del hardware local antes de persistirla en el disco o sincronizarla. El servidor remoto actuará en el futuro únicamente como un "tubo ciego" que almacena texto cifrado, sin capacidad de leer la información.

## ⚡ Stack Tecnológico

La arquitectura está dividida de forma estricta en dos capas de alto rendimiento:

### Backend Nativo (Core - Rust)
- **Rust**: Motor principal encargado de la criptografía, seguridad y acceso a datos.
- **Tauri v2**: Puente IPC multiplataforma de consumo ultra-bajo.
- **SQLite (`rusqlite`)**: Base de datos relacional embebida (acceso directo sin plugins de Tauri para mantener el patrón `with_conn`), garantizando autonomía total offline.
- **AES-GCM-256**: Cifrado autenticado para texto médico (entries, nombres, perfil) y para envolver la DataKey en disco.
- **Ed25519 (Dalek)**: Firma digital asimétrica para garantizar el No-Repudio y validez legal de las notas médicas.
- **SHA-256**: Hashing determinista para Índices Ciegos, huellas digitales de documentos y derivación de Seed Phrase.
- **Argon2id**: Hashing de contraseñas y derivación de claves (KDF) — incluye el envoltorio de la DataKey.
- **Seed Phrase**: Frase mnemotécnica de 6 palabras en papel para bootstrap y recovery de la DataKey (no se persiste en disco).

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
 ├── types.rs             # Contenedores de estado seguro (Mutex<Option<T>>).
 │
 ├── security/            # Núcleo criptográfico (stateless respecto al IPC).
 │   ├── auth.rs          # Hashing Argon2id para autenticación de usuarios.
 │   ├── crypto.rs        # Motor matemático puro (AES-GCM, Ed25519, Nonces OsRng, SHA-256 Blind Index).
 │   ├── data_key.rs      # Envoltorio AES-GCM de la DataKey (wrap por usuario + master wrap con seed) + migración legacy.
 │   ├── seed.rs          # Generación de mnemonic (6 palabras) y derivación de key con SHA-256.
 │   └── vault.rs         # Gestión de Bóvedas criptográficas por usuario (Cold Start + Llave Privada de Firma).
 │
 ├── db/                  # Capa de persistencia.
 │   ├── config_schema.rs # Structs para parsear schema.json dinámicamente.
 │   └── database.rs      # Inicialización física de SQLite (WAL Mode activado) y esquemas relacionales.
 │
 ├── export/              # Motor de exportación de historias clínicas.
 │   └── render.rs        # Snapshot tipado + render puro a Markdown (filiación, perfil, entries cronológicas, footer SHA-256).
 │
 └── commands/            # Controladores de API interna (Aduana de peticiones de React).
     ├── mod.rs           # Exportación plana y Helpers (`with_conn`, `get_data_key`).
     ├── auth_commands.rs # Registro, login, unlock_vault (resuelve DataKey), lock_vault, setup_seed_master_wrap.
     ├── entity_commands.rs    # Endpoints genéricos de entidades (Template v1).
     ├── entry_commands.rs     # CRUD append-only de entries (inmutables, con firma + búsqueda).
     ├── export_commands.rs    # `get_export_snapshot` (descifrado completo) + `export_history` (escritura en `exports/`).
     ├── professional_profile_commands.rs # Endpoints de perfil profesional.
     └── seed_commands.rs      # Generación, verificación y derivación de la frase semilla.
```

### Frontend UI (React)
Implementación de un Design System propio basado en componentes atómicos, completamente desacoplados del estado de la aplicación mediante Zustand.

```text
src/
 ├── stores/                 # Capa de Estado Global (Zustand)
 │   ├── useAuthStore.ts     # Estado de la Bóveda (Cold Start UI, cierre seguro de sesión).
 │   ├── useEntityStore.ts   # CRUD genérico de entidades (listado, búsqueda, creación).
 │   ├── useEntryStore.ts    # Gestión de entries con paginación y búsqueda.
 │   ├── useNavigationStore.ts # Estado de navegación (tabs, drawer).
 │   ├── useSeedStore.ts     # Flag de seed configurada (persistido; la frase NO se persiste).
 │   └── usePatientStore.ts  # Fuente única de verdad para paciente seleccionado.
 │
 ├── config/
 │   └── schema.json              # Definición de entidades (copia empaquetada del backend en src-tauri/config/).
 │
 ├── components/
 │   ├── layouts/
 │   │   └── DashboardLayout.tsx  # Logout seguro (limpia DataKey en RAM) + botón seed.
 │   ├── ui/                 # Design System Atómico
 │   │   ├── Button.tsx, Card.tsx, Input.tsx, Navbar.tsx, Textarea.tsx, Alert.tsx
 │   │   └── Drawer.tsx      # NavigationDrawer con menú por roles.
 │   ├── views/
 │   │   ├── AdminView.tsx
 │   │   ├── MedicoView.tsx  # Tabs: Perfil, Paciente, EHR, Entries (+ export .md).
 │   │   └── PacienteView.tsx
 │   │
 │   ├── PatientEhrView.tsx  # Interfaz clínica unificada (ficha + evoluciones).
 │   ├── Entity.tsx          # Sandbox de admisión con panel dual (consume usePatientStore).
 │   ├── EntryTimeline.tsx   # Vista cronológica con filtros, búsqueda y load-more.
 │   ├── EntryCard.tsx       # Card individual de entry (muestra autor y firma).
 │   ├── EntryForm.tsx       # Formulario dinámico por categoría de entry.
 │   ├── SeedPhraseSetup.tsx # UI de configuración de seed (4 pasos + master wrap).
 │   ├── ProfileView.tsx
 │   └── AuthBox.tsx         # Login/registro + manejo de SEED_REQUIRED (bootstrap).
```

> 🎨 Las reglas visuales del frontend (tokens de color, tipografía, radios, estados interactivos, semántica de color y estrategia de temas claro/oscuro) están definidas en **[DESIGN.md](./DESIGN.md)**.

## Principios de Diseño del Core & Frontend

- **Patrón `with_conn`**: Ningún comando saca la conexión a la base de datos de su `Mutex`. Se le inyecta un closure para garantizar que el lock se libere instantáneamente, evitando congelamientos de UI.
- **Inyección de Dependencias**: Los comandos que requieren cifrado reciben `State<'_, DataKey>` o `State<'_, SigningState>`, extraen la llave y fallan de forma segura si no está inicializada.
- **Estado Reactivo Seguro (Zustand)**: El Frontend jamás almacena contraseñas ni claves de cifrado en JavaScript. Los Stores solo contienen booleanos de estado y datos descifrados para renderizar.
- **Store Unificado de Paciente**: `usePatientStore` es la fuente única de verdad para la selección de paciente. `PatientEhrView`, `Entity`, `EntryForm` y `EntryTimeline` consumen el mismo estado, garantizando sincronización instantánea sin props-drilling.
- **Entries Append-Only**: Las entries son inmutables una vez creadas. No existen operaciones de update ni delete, garantizando integridad y no-repudio. La corrección se hace con nuevas entries (ej: "Corrección de...").
- **Flujo Unificado de EHR**: La interfaz `PatientEhrView` consolida búsqueda, ficha y evoluciones en una sola vista, eliminando el context-switching entre tabs.
- **Limpieza de Datos Sensibles**: El frontend nunca retiene la contraseña en estado de React. Se limpia inmediatamente después de `unlock_vault`. `lock_vault` destruye CryptoState, SigningState y DataKey de la RAM.
- **DataKey Nunca en Claro**: La llave de cifrado de datos médicos no se persiste en texto plano. Se envuelve con Argon2id(password) por usuario (fast path) y con SHA-256(seed) en el master wrap (bootstrap/recovery). La seed vive en papel, no en disco.
- **Seed Phrase en Papel**: La frase semilla (6 palabras) no se persiste en `localStorage`. Solo el flag `isSeedConfigured` se guarda; la frase se muestra una vez en `SeedPhraseSetup` para anotarla. Se tipea únicamente en bootstrap de usuarios nuevos o recovery.

## 🔒 Especificación de Seguridad (Zero-Knowledge & Vault)

### 1. Ciclo de Vida de las Claves (Cold Start)
El sistema no utiliza claves hardcodeadas. Las claves nacen y mueren en la RAM:
1. El usuario ingresa su contraseña en la UI.
2. React llama a `unlock_vault`. Rust verifica el hash en la tabla `users` (Argon2id).
3. La misma contraseña se usa como semilla para derivar 32 bytes mediante Argon2id en modo KDF (Llave Maestra). La sal del KDF es única por usuario (`vault_{user_id}.salt`).
4. Se desbloquea el archivo `vault_{user_id}.bin`, el cual contiene la Llave Privada de Firma (Ed25519) cifrada con la Llave Maestra.
5. Se resuelve la **DataKey** (ver §3.1): wrap personal → seed+master wrap → migración legacy → generación.
6. Las tres llaves (Master, Signing, DataKey) se inyectan en `CryptoState`, `SigningState` y `DataKey` (RAM volátil de Rust).
7. Al cerrar sesión, React llama a `lock_vault`. Rust destruye los `Option` de la RAM. Las claves dejan de existir.

**Nota**: la DataKey **no** se carga al arrancar la app — solo existe en RAM después de un `unlock_vault` exitoso.

### 2. Aislamiento Multi-Usuario (Vaults)
Cada médico tiene su propio archivo de bóveda (`vault_{user_id}.bin`), su propia sal de KDF (`vault_{user_id}.salt`) y su propio wrap de DataKey (`.data_key.{user_id}`).

### 3. Modelo de Cifrado por Capas

#### 3.1 DataKey en Disco (Wrap)
La DataKey (llave que cifra todos los datos médicos) **nunca se persiste en claro**:

| Archivo | Llave de envoltorio | Propósito |
|---|---|---|
| `.data_key.{user_id}` | Argon2id(password) | Fast path — login normal sin seed |
| `.data_key.master` | SHA-256(seed) | Bootstrap de usuarios nuevos + recovery |
| `.data_key` (legacy) | — | Hex en claro — se migra y **borra** en el primer login |

**Flujo `resolve_data_key`** (en `unlock_vault`):
1. Wrap personal existe → desenvuelve con master_key → listo.
2. Master wrap existe + seed provista → desenvuelve, crea wrap personal → listo.
3. Master wrap existe sin seed → error `SEED_REQUIRED` → frontend pide la frase del papel.
4. Legacy hex existe → migra, borra legacy, crea wrap personal.
5. Nada → genera DataKey nueva (primera instalación).

#### 3.2 Datos Médicos
| Capa | Tabla/Archivo | Llave | Propósito |
|---|---|---|---|
| Filiación | `entities.enc_data_blob` | `DataKey` | Datos demográficos cifrados campo a campo |
| Entries | `entries.payload` | `DataKey` | Entries clínicas inmutables cifradas + firma Ed25519 |
| Perfil Profesional | `professional_profiles.*_ciphertext` | `DataKey` | Datos del médico cifrados individualmente |
| Vault | `vault_{user_id}.bin` | Master Key (derivada) | Llave privada de firma cifrada |
| DataKey | `.data_key.{user_id}` / `.data_key.master` | Password / Seed | Envoltorio de la DataKey (ver §3.1) |

### 4. Blind Index — Búsqueda Sin Descifrado
Hash determinista SHA-256 (DNI normalizado + sal de instalación) para búsqueda exacta sin exponer el dato real. La sal es global a la instalación (`/.installation_salt`) para permitir búsqueda compartida entre médicos de la misma máquina.

### 5. Firma Digital y No-Repudio
Cada entry se firma con Ed25519 usando la llave privada del médico en RAM. El payload de firma es `category|subject_id|title|status|timestamp` (consistente entre creación y verificación). La llave pública se almacena en `professional_profiles.public_key` y se sincroniza en cada `unlock_vault` (derivada desde la privada si el vault ya existía).

### 6. Seed Phrase (Frase Semilla) — Bootstrap y Recovery
Sistema de recuperación que protege el master wrap de la DataKey:
1. **Generación**: 6 palabras mnemotécnicas de una wordlist curada (evita palabras ambiguas).
2. **Derivación**: SHA-256 sobre las 6 palabras genera una key de 32 bytes.
3. **Almacenamiento**: La frase **NO se persiste** — se muestra una vez en `SeedPhraseSetup` para anotarla en papel. Solo el flag `isSeedConfigured` vive en `localStorage`.
4. **Master wrap**: Tras verificar la frase, `setup_seed_master_wrap` envuelve la DataKey con la seed-derived key → `.data_key.master`.
5. **Uso**: Solo se tipea en (a) primer login de un usuario nuevo sin wrap personal, o (b) recovery. Los logins normales usan el wrap por contraseña.
6. **Flujo Cold Start**: Si no hay seed configurada, `SeedPhraseSetup` guía en 4 pasos (Generar → Mostrar → Verificar → Confirmar). El botón "Omitir" no marca la seed como configurada.

## 🗄️ Estructura de la Base de Datos

### Tablas Principales
| Tabla | Propósito | Relación |
|---|---|---|
| `users` | Autenticación (Médicos, Admins, Pacientes) | PK `id TEXT` |
| `professional_profiles` | Perfil profesional + llave pública Ed25519 | FK → `users(id)` |
| `entities` | Entidades genéricas (filiación de pacientes) | PK `id INTEGER` |
| `entries` | Entries clínicas inmutables (append-only) — único sistema clínico | FK → `entities(id)`, FK → `users(id)` |
| `entity_keys` | Claves de datos por entidad-usuario (telemedicina) | FK → `entities(id)` |
| `sync_queue` | Cola FIFO de sincronización | Reemplaza `is_synced` |

> **Nota**: Las tablas legacy `medical_history` y `notes` fueron eliminadas del esquema. Todo el dato clínico vive en `entries` (categorías `SOAP_NOTE`, `CONDITION`, `ALLERGY`, `MEDICATION`). En dev, borrá `historia_clinica.db` para limpiar tablas obsoletas.

### Tabla `entries` (Entity-Entry Paradigm — única tabla clínica)
```sql
CREATE TABLE entries (
    id TEXT PRIMARY KEY NOT NULL,
    category TEXT NOT NULL CHECK(category IN ('SOAP_NOTE', 'MEDICATION', 'ALLERGY', 'CONDITION')),
    subject_id INTEGER NOT NULL,            -- FK → entities(id)
    author_id TEXT NOT NULL,                -- FK → users(id)
    title TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('ACTIVE', 'RESOLVED', 'COMPLETED')),
    timestamp TEXT NOT NULL,
    payload BLOB NOT NULL,                  -- Cifrado con DataKey (AES-GCM-256)
    signature TEXT NOT NULL,                -- Firma Ed25519
    created_at TEXT NOT NULL,
    is_synced INTEGER NOT NULL DEFAULT 0 CHECK(is_synced IN (0, 1)),
    FOREIGN KEY(subject_id) REFERENCES entities(id) ON DELETE RESTRICT,
    FOREIGN KEY(author_id) REFERENCES users(id) ON DELETE RESTRICT
);

CREATE INDEX idx_entries_subject ON entries(subject_id, timestamp DESC);
CREATE INDEX idx_entries_category ON entries(category);
CREATE INDEX idx_entries_sync ON entries(is_synced) WHERE is_synced = 0;
```

**Nota**: Las entries son **inmutables**. No existen `update_entry` ni `delete_entry`.

## 🏗️ Arquitectura de Template (v1)

*"El core no sabe si el paciente es humano o animal. Sabe que hay entidades, atributos, relaciones y cifrado. El resto es configuración."*

### Archivos de Configuración Empaquetados
| Archivo | Propósito | Consumidor |
|---|---|---|
| `src/config/schema.json` (copia del backend en `src-tauri/config/`) | Define entidades y campos de filiación | `entity_commands.rs` (via `include_str!`) + `Entity.tsx` |
| `config/entry_templates/soap.json` | Template de entry SOAP | `EntryForm.tsx` |
| `config/entry_templates/allergy.json` | Template de alergia | `EntryForm.tsx` |
| `config/entry_templates/medication.json` | Template de medicación | `EntryForm.tsx` |
| `config/entry_templates/condition.json` | Template de condición médica | `EntryForm.tsx` |

`schema.json` se empaqueta en el binario con `include_str!` en `lib.rs`. Los `entry_templates/` se embeben en el bundle de Vite con `import.meta.glob(..., { eager: true })`: `EntryForm.tsx` los descubre del disco en build time, valida el shape (`category`, `label`, `fields`) y arma el diccionario de templates. Todo funciona offline y es inmutable en runtime.

**Paradigma Entity-Entry**: El core no conoce el significado clínico de cada campo. Simplemente almacena `entries` cifradas con su categoría y las presenta según el template JSON correspondiente. Agregar nuevos tipos de entries solo requiere crear un nuevo archivo JSON en `config/entry_templates/` (y sumar la categoría al tipo `EntryCategory` y al CHECK de SQLite).

## 📤 Exportación de Historia Clínica (Markdown)

La historia completa de un paciente se exporta a un archivo `.md` legible y versionable. El diseño es **stateless respecto a claves**: los datos se descifran en Rust y el frontend solo recibe el snapshot ya resuelto (nunca la DataKey).

### Flujo

```text
MedicoView (tab Entries) ─ "Exportar .md"
  │
  ├─ 1. get_export_snapshot(subject_id, professional_user_id)
  │      Descifra en un solo roundtrip: paciente + perfil profesional + TODAS las entries.
  │      Falla con error claro si la bóveda está cerrada.
  │
  └─ 2. export_history(snapshot)
         Render puro a Markdown → escritura en <app_local_data_dir>/exports/
         Devuelve { path, filename, entries_count } → banner "Revelar en carpeta"
```

### Decisiones de diseño

| Aspecto | Decisión | Razón |
|---|---|---|
| Destino | Carpeta fija `exports/` de la app | Sin `tauri-plugin-dialog` — compatible con el futuro build mobile |
| Nombre de archivo | `HistoriaClinica_Apellido_Nombre_YYYY-MM-DD_HHMMSS.md` (sanitizado, único por timestamp) | Sin conflictos de sobrescritura ni diálogos |
| Campos por categoría | Labels canónicos de `config/entry_templates/*.json` (orden y nombre) | El `.md` es espejo del formulario clínico |
| Orden | Cronológico ascendente | Lectura continua de la evolución |
| Firma | Check ✓/✗ por entry + longitud corta de la firma | El `.md` es una foto, no la fuente de verdad — la verificación real sigue en la app |
| Integridad | SHA-256 del contenido en el footer + aviso de confidencialidad | Permite detectar alteraciones del archivo exportado |
| "Revelar en carpeta" | `revealItemInDir` de `@tauri-apps/plugin-opener` | Ya instalado y concedido vía `opener:default` (sin cambios de capability) |

Los commands se registran en `lib.rs` (`get_export_snapshot`, `export_history`), la lógica vive en `commands/export_commands.rs` y el render puro en `export/render.rs` (sin estado, sin I/O → 6 tests unitarios directos).

## 🧪 Tests Automatizados

El Core cuenta con 70 tests unitarios que cubren:
| Módulo | Tests | Cobertura |
|---|---|---|
| `security/crypto.rs` | 13 | AES-GCM roundtrip, nonces únicos, clave incorrecta, manipulación, Blind Index, Firma Ed25519, derivación de pública desde privada |
| `security/vault.rs` | 10 | Creación, desbloqueo, contraseña incorrecta, sal única por usuario, `change_password` (vault re-cifrado, invariante data_key resolvable, wrap huérfano recupera con seed) |
| `security/seed.rs` | 12 | Generación mnemonic, unicidad, derivación SHA-256, verificación, edge cases, word list sin duplicados ni tildes |
| `security/data_key.rs` | 10 | Wrap/unwrap AES-GCM, prioridades de resolución, `SEED_REQUIRED`, migración legacy |
| `commands/entry_commands.rs` | 15 | CRUD completo end-to-end con DB real (roundtrip cifrado→descifrado, payload sin texto plano en DB, validaciones de categoría/status, states obligatorios, firma rota por manipulación, wrong data key, autor sin perfil, paginación e isolación por paciente, filtro por categoría, búsqueda case-insensitive, `decrypt_payload` |
| `commands/export_commands.rs` | 4 | Snapshot descifrado (paciente + perfil), bóveda cerrada, escritura del `.md` (heading, filename, footer) y rechazo con 0 entries |
| `export/render.rs` | 6 | Header con paciente/perfil, orden cronológico, orden/labels canónicos de campos, footer SHA-256 + aviso, filename sanitizado y fallback sin nombre |

```bash
cargo test --lib  # 70 passed; 0 failed
```

> Los tests de `entry_commands` usan `tauri::test` (feature habilitada solo en `[dev-dependencies]`; el binario de producción no la incluye).

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

### 🟩 Fase 3.5: Separación de Datos Clínicos (✅ Completado — legacy eliminado)
- [x] Tabla dedicada `medical_history` con UNIQUE por paciente
- [x] Schema separado `medical_history_schema.json`
- [x] Comandos `upsert_medical_history` + `get_medical_history`
- [x] Store unificado `usePatientStore` (paciente + clinical compuesto)
- [x] `MedicalHistoryForm` desacoplado con optimistic update
- [x] `Entity.tsx` con panel dual sincronizado al store global
- [x] `PatientEhrView` consume antecedentes desde `selected.clinical`
- [x] Limpieza de `schema.json` (solo filiación en `patient`)
- [x] **Eliminado en favor de entries**: tabla, schema, comandos, `MedicalHistoryForm` y tab "Antecedentes"

### 🟩 Fase 3.6: Entity-Entry Paradigm (✅ Completado)
- [x] **Seed Phrase**: Módulo `seed.rs` con mnemonic de 6 palabras + 10 tests
- [x] **Tabla `entries`**: Append-only con índices para subject/category/timestamp
- [x] **Entry Templates**: 4 templates JSON (soap, allergy, medication, condition)
- [x] **Backend**: `entry_commands.rs` con create/get/by_subject/by_category/search_entries
- [x] **Frontend**: `useEntryStore`, `EntryTimeline`, `EntryCard`, `EntryForm`
- [x] **Seed UI**: `SeedPhraseSetup` con flujo de 4 pasos + `useSeedStore`
- [x] **Navegación**: `NavigationDrawer` con menú por roles + `useNavigationStore`
- [x] **Búsqueda**: `search_entries` con debounce de 300ms + filtros por categoría
- [x] **Cleanup**: Removido tab "Antecedentes" (legacy), 26 tests pasando

### 🟩 Fase 3.7: Cifrado de DataKey + Limpieza Legacy (✅ Completado)
- [x] **Módulo `data_key.rs`**: wrap/unwrap AES-GCM + `resolve_data_key` con prioridades
- [x] **DataKey cifrada en disco**: `.data_key.{user_id}` (Argon2id password) + `.data_key.master` (SHA-256 seed)
- [x] **Migración automática**: `.data_key` legacy hex → wrap personal, archivo borrado
- [x] **Bootstrap**: `unlock_vault` devuelve `SEED_REQUIRED`; `AuthBox` pide la frase
- [x] **Master wrap**: `setup_seed_master_wrap` tras verificar la frase en `SeedPhraseSetup`
- [x] **Limpieza `medical_history`**: eliminados comandos, schema (ambas copias), `MedicalHistoryForm`, `usePatientStore.clinical`
- [x] **Seed en papel**: frase no persistida en `localStorage`, solo flag `isSeedConfigured`
- [x] 36 tests pasando

### 🟩 Fase 3.8: Unificación del Sistema Clínico (✅ Completado)
- [x] **Eliminado el sistema SOAP viejo (`notes`)**: tabla, índices, `note_commands.rs`, `NoteTemplatesConfig`, `include_str!(soap.json)`, comandos `create_note`/`get_note`/`get_notes_by_entity`
- [x] **Frontend limpio**: eliminados `useNoteStore.ts`, `note_templates/soap.json` (ambas copias), uso en `PatientEhrView` y `DashboardLayout`
- [x] **Único sistema clínico**: `PatientEhrView` + `EntryTimeline` consumen solo `entries` (paradigma Entity-Entry)
- [x] **Fix firma multi-usuario**: `unlock_vault` sincroniza `professional_profiles.public_key` siempre (derivada desde la privada si el vault ya existía), antes de `resolve_data_key`
- [x] 60 tests pasando

### 🟩 Fase 3.9: Exportación de Historia Clínica + Orden del Core (✅ Completado)
- [x] **`get_export_snapshot`**: descifra paciente + perfil + todas las entries en un solo roundtrip IPC (la DataKey nunca llega al frontend); error claro si la bóveda está cerrada
- [x] **`export_history`**: render puro a Markdown → escritura en `<app_local_data_dir>/exports/` (sin plugin de diálogos, compatible con mobile futuro)
- [x] **Render `export/render.rs`**: tabla de filiación y perfil, resumen por categoría, entries cronológicas con labels canónicos de los templates, check de firma ✓/✗, footer con SHA-256 + aviso de confidencialidad
- [x] **UI en `MedicoView` (tab Entries)**: botón "Exportar .md", banner de éxito con "Revelar en carpeta" (`revealItemInDir`), banners descartables y reset al cambiar de paciente
- [x] **Reorganización `src-tauri/src/` por dominio**: `security/` (auth, crypto, data_key, seed, vault), `db/`, `export/` y `commands/` (incluye `seed_commands`); `lib_types.rs` → `types.rs`
- [x] **Templates en JSON**: `EntryForm.tsx` consume `config/entry_templates/*.json` via `import.meta.glob` (eliminado el `TEMPLATES` hardcodeado)
- [x] 70 tests pasando

### 🟧 Fase 4: Infraestructura y Sincronización Híbrida (Próximo paso)
- [ ] Despliegue de VPS con Dokploy y PocketBase
- [ ] Sync Engine (`tokio`): background worker polling `sync_queue`
- [ ] Sincronización de `.data_key.master` y `.installation_salt` entre dispositivos
- [ ] Implementar Blind Index en servidor
- [x] Cifrar `.data_key` en disco con llave derivada (Fase 3.7)
- [ ] `enroll_data_key`: admin envuelve DataKey al registrar médico (sin tipear seed)
- [x] `change_password`: verifica actual, re-cifra vault, re-envuelve `.data_key.{user_id}`, actualiza `password_hash` (no toca master wrap) + UI en Perfil & 6 tests

### 🟥 Fase 5: Teleconsulta Asíncrona (Multi-Usuario)
- [ ] Tabla `asynchronous_threads` y linkage paciente-usuario
- [ ] Cifrado asimétrico o clave de sesión compartida
- [ ] Flujo de compartir `entity_keys` entre médicos

### ⬜ Fase 6: Interoperabilidad (Futuro)
- [ ] Motor de exportación JSON y traducción a estándares FHIR (IA-Driven)

## 🚀 Comandos Útiles de Desarrollo

```bash
pnpm tauri dev          # Levanta el entorno (Rust + React HMR)
pnpm tauri build        # Compila en modo release para producción
npx tsc --noEmit        # Verifica TypeScript sin generar archivos
cargo check             # Verifica compilación Rust
cargo test --lib        # Ejecuta los 70 tests unitarios del Core
cargo add <crate>       # Añade dependencias al backend (desde src-tauri)
```
