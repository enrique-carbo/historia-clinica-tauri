# 🩺 Simplex Health Core — Historia Clínica Digital

Aplicación de escritorio nativa orientada a la gestión de historias clínicas y telemedicina asíncrona con un enfoque **Local-First** y arquitectura de seguridad **Zero-Knowledge (Cifrado de Extremo a Extremo)**.

El sistema garantiza la máxima privacidad procesando y cifrando toda la información médica sensible directamente en la RAM del hardware local antes de persistirla en el disco o sincronizarla. El servidor remoto actuará en el futuro únicamente como un "tubo ciego" que almacena texto cifrado, sin capacidad de leer la información.

---

## ⚡ Stack Tecnológico

La arquitectura está dividida de forma estricta en dos capas de alto rendimiento:

* **Backend Nativo (Core - Rust):**
  * **Rust:** Motor principal encargado de la criptografía, seguridad y acceso a datos.
  * **Tauri v2:** Puente IPC multiplataforma de consumo ultra-bajo.
  * **SQLite (`rusqlite`):** Base de datos relacional embebida (acceso directo sin plugins de Tauri para mantener el patrón `with_conn`), garantizando autonomía total *offline*.
  * **AES-GCM-256:** Cifrado autenticado para texto médico (Campos SOAP, Nombres).
  * **Ed25519 (Dalek):** Firma digital asimétrica para garantizar el No-Repudio y validez legal de las notas médicas.
  * **SHA-256:** Hashing determinista para Índices Ciegos y huellas digitales de documentos.
  * **Argon2id:** Hashing de contraseñas y derivación de claves (KDF).

* **Frontend (UI):**
  * **React 19 + TypeScript (Strict):** Interfaz declarativa con tipado extremo a extremo (los structs de Rust se espejan en TS).
  * **Zustand:** Gestor de estado global ultraligero. Maneja sesiones de seguridad y datos clínicos sin *boilerplate* ni *props-drilling*.
  * **Tailwind CSS v4:** Framework de utilidades CSS para diseño rápido, consistente y responsivo.
  * **Vite 7:** Entorno de compilación rápida con HMR (Hot Module Replacement).
  * **pnpm:** Gestor de paquetes eficiente mediante *hard links*.

---

## 📂 Arquitectura del Proyecto

### Backend Core (Rust)
Diseño modular estricto para evitar dependencias circulares. El acceso a la base de datos y a la criptografía está aislado mediante patrones de diseño avanzados.

```text
src-tauri/src/
├── main.rs              # Punto de entrada al ejecutable nativo.
├── lib.rs               # Orquestador de Tauri, inyección de estados globales (DbState, CryptoState, SigningState).
├── lib_types.rs         # Contenedores de estado seguro (Mutex<Option<T>>).
│
├── config_schema.rs     # Structs para parsear schema.json y note_templates dinámicamente.
├── vault.rs             # Gestión de Bóvedas criptográficas por usuario (Cold Start + Llave Privada de Firma).
├── auth.rs              # Hashing Argon2id para autenticación de usuarios.
├── crypto.rs            # Motor matemático puro (AES-GCM, Ed25519, Nonces OsRng, SHA-256 Blind Index).
├── database.rs          # Inicialización física de SQLite (WAL Mode activado) y esquemas relacionales.
│
└── commands/            # Controladores de API interna (Aduana de peticiones de React).
    ├── mod.rs           # Exportación plana y Helpers (`with_conn`, `get_key`).
    ├── auth_commands.rs # Endpoints de registro, login y desbloqueo de bóveda.
    ├── entity_commands.rs    # Endpoints genéricos de entidades (Template v1).
    ├── note_commands.rs      # Endpoints genéricos de notas con firma digital (Template v1).
    ├── patient_commands.rs   # Endpoints legacy de gestión de pacientes.
    ├── metric_commands.rs    # Endpoints de métricas clínicas (EAV y cifrado selectivo).
    └── soap_commands.rs      # Endpoints legacy de notas médicas SOAP.
```

### Frontend UI (React)
Implementación de un **Design System** propio basado en componentes atómicos, completamente desacoplados del estado de la aplicación mediante **Zustand**.

```text
src/
├── stores/                 # Capa de Estado Global (Zustand)
│   ├── useAuthStore.ts     # Estado de la Bóveda (Cold Start UI, cierre seguro de sesión).
│   ├── useEntityStore.ts   # ← NUEVO: Padrón de entidades genéricas (Template v1).
│   ├── useNoteStore.ts     # ← NUEVO: Evoluciones clínicas con firma digital.
│   ├── usePatientStore.ts  # Estado legacy de la Consulta Activa (Historial SOAP, Métricas EAV).
│   └── usePatientRegistryStore.ts # Estado legacy del Padrón.
│
├── components/
│   ├── layouts/
│   │   └── DashboardLayout.tsx  # ← ACTUALIZADO: Logout seguro, limpia todos los stores.
│   ├── ui/                 # Design System Atómico
│   │   ├── Button.tsx
│   │   ├── Card.tsx
│   │   ├── Input.tsx
│   │   ├── Navbar.tsx
│   │   ├── Textarea.tsx
│   │   └── Alert.tsx
│   ├── views/
│   │   ├── AdminView.tsx
│   │   ├── MedicoView.tsx  # ← ACTUALIZADO: Tabs unificados (EHR, Consulta, Paciente).
│   │   └── PacienteView.tsx
│   │
│   ├── AuthBox.tsx         # ← ACTUALIZADO: Limpia password de estado tras uso.
│   ├── PatientEhrView.tsx  # ← NUEVO: Interfaz clínica unificada.
│   │                       #     Buscar paciente → Ficha → Evoluciones → Nueva nota + firma.
│   ├── EntityTest.tsx      # Sandbox para testing de entidades (desarrollo).
│   ├── NoteTest.tsx        # Sandbox para testing de notas (desarrollo).
│   ├── PatientManager.tsx  # Admisión legacy.
│   ├── PatientSelector.tsx # Buscador legacy.
│   ├── SoapForm.tsx        # Redacción SOAP legacy.
│   ├── SoapHistory.tsx     # Historial legacy.
│   ├── MetricQuickForm.tsx # Registro rápido EAV legacy.
│   └── MetricViewer.tsx    # Listado de métricas legacy.
```


### Principios de Diseño del Core & Frontend
1. **Patrón `with_conn`:** Ningún comando saca la conexión a la base de datos de su `Mutex`. Se le inyecta un *closure* para garantizar que el lock se libere instantáneamente, evitando congelamientos de UI.
2. **Inyección de Dependencias:** Los comandos que requieren cifrado reciben `State<'_, CryptoState>`, extraen la llave maestra y si el Vault no fue desbloqueado, fallan de forma segura.
3. **Estado Reactivo Seguro (Zustand):** El Frontend jamás almacena contraseñas ni claves de cifrado en JavaScript. Los Stores solo contienen booleanos de estado (ej: `isVaultUnlocked`) y datos descifrados para renderizar.
4. **Optimistic UI:** Las inserciones (ej: nuevo paciente) se inyectan en el Store al instante sin esperar un nuevo `SELECT` de SQLite, logrando tiempos de respuesta de 0ms en la UI.
5. **UI Desacoplada:** Los componentes de `/ui` no conocen la estructura de la base de datos ni las rutas de la API, solo reciben `props` primitivas.
6. **Flujo Unificado de EHR:** La interfaz `PatientEhrView` consolida búsqueda, ficha y evoluciones en una sola vista, eliminando el context-switching entre tabs.
7. **Limpieza de Datos Sensibles:** El frontend nunca retiene la contraseña en estado de React. Se limpia inmediatamente después de `unlock_vault`.

---

## 🔒 Especificación de Seguridad (Zero-Knowledge & Vault)

### 1. Ciclo de Vida de las Claves (Cold Start)
El sistema no utiliza claves hardcodeadas. Las claves nacen y mueren en la RAM:
1. El usuario ingresa su contraseña en la UI.
2. React llama a `unlock_vault`. Rust verifica el hash en la tabla `users` (Argon2id).
3. La misma contraseña se usa como semilla para derivar 32 bytes mediante Argon2id en modo KDF (Llave Maestra). La sal del KDF es **única por usuario** (`vault_{user_id}.salt`) y se genera con `OsRng` en el primer login.
4. Se desbloquea el archivo `vault_{user_id}.bin`, el cual contiene la Llave Privada de Firma (Ed25519) cifrada con la Llave Maestra.
5. Ambas llaves se inyectan en el `CryptoState` y `SigningState` (RAM volátil de Rust).
6. Al cerrar sesión, React llama a `lock_vault`. Rust destruye los `Option` de la RAM. Las claves dejan de existir.

### 2. Aislamiento Multi-Usuario (Vaults)
Cada médico tiene su propio archivo de bóveda (`vault_{user_id}.bin`) y su propia sal de KDF (`vault_{user_id}.salt`). Las notas cifradas por el Dr. House no pueden ser descifradas por la Dra. Cameron (aislamiento criptográfico por defecto) sin necesidad de lógica de permisos compleja.

### 3. a. Patrón "Blob Cifrado" en Pacientes (Legacy)
Los datos demográficos del paciente se empaquetan en un JSON, se cifran como un solo bloque y se guardan en SQLite. Solo el Nombre y los Índices Ciegos se mantienen separados para rendimiento del listado.

### 3. b. Patrón "Blob Cifrado" en Entidades (Template v1)
Los datos demográficos se definen dinámicamente en `config/schema.json`. Cada campo se cifra individualmente con **AES-GCM-256 + nonce único de 12 bytes** (`OsRng`), se serializan en un JSON estructurado y se guardan en `enc_data_blob`. El campo marcado como `blind_index: true` genera un **Índice Ciego** (SHA-256 + sal derivada de master_key) para búsqueda exacta sin exponer el dato real.

### 4. Cifrado Clínico Granular (SOAP) y Desidentificación (EAV)
* **Texto (SOAP, Nombres):** Cifrado independiente por campo con un **Nonce único de 12 bytes** (`OsRng`) por inserción.
* **Métricas (EAV):** Variables numéricas (`value_num`) almacenadas en texto plano desidentificado para permitir cálculos matemáticos y gráficos instantáneos (<1ms) sin comprometer la identidad del paciente. Notas opcionales van cifradas.

### 5. Firma Digital y No-Repudio (Auditoría)
Cada vez que un médico guarda una nota, el Core concatena los campos según `signature_payload_order` del template, calcula un Hash SHA-256 y lo firma usando su Llave Privada (Ed25519) en RAM. La firma se guarda en la base de datos. Al leer la nota, el sistema utiliza la Llave Pública del médico para verificar matemáticamente que el documento no ha sido alterado desde su creación.

### 6. Prevención de Duplicados (Blind Index)
Hash determinista **SHA-256** (DNI + sal derivada de master_key) para rechazar registros duplicados mediante `UNIQUE` en SQLite **sin conocer la identidad real del paciente**.

---

## 🧪 Tests Automatizados

El Core cuenta con **18 tests unitarios** que cubren:

| Módulo | Tests | Cobertura |
|--------|-------|-----------|
| `crypto.rs` | 14 | Cifrado AES-GCM (roundtrip, nonces únicos, clave incorrecta, manipulación de ciphertext/nonce), Blind Index (determinismo, normalización de DNI, sal diferente por usuario), Firma Ed25519 (sign/verify, llave equivocada, documento alterado, firma alterada) |
| `vault.rs` | 4 | Creación de vault, desbloqueo exitoso, contraseña incorrecta, sal única por usuario (misma contraseña = diferente master key) |

```bash
cargo test  # 18 passed; 0 failed
```

---

## 🗺️ Mapa de Ruta del Desarrollo (Roadmap)

### 🟩 Fase 1: Almacenamiento Local y Autenticación (¡Completado!)
* [x] Entorno Tauri v2 + pnpm + React + Tailwind CSS.
* [x] Sistema de Vault Local (`vault.rs`) con sal única por usuario.
* [x] Refactorización a módulos con patrón `with_conn`.

### 🟩 Fase 2: Robustez del Core Local (¡Completado!)
* [x] Índice Ciego (Blind Index) con sal derivada de master_key.
* [x] Buscador Relacional Clínico.
* [x] Creación de Design System propio (`/ui`).

### 🟩 Fase 3: Métricas, Firmas Digitales y Arquitectura Frontend (¡Completado!)
* [x] Comandos de inserción/lectura para `patient_metrics`.
* [x] Cifrado selectivo (Numéricas en claro, notas cifradas).
* [x] Interfaz rápida de registro y listado de variables.
* [x] Migración del estado a **Zustand** (Stores de Auth, Patient y Registry).
* [x] Implementación de **Optimistic UI** y tipado estricto End-to-End.
* [x] Implementación de **Firma Digital Ed25519** para notas (No-Repudio).
* [x] Refactor a Patrón "Blob Cifrado" para datos demográficos de pacientes.
* [x] Arquitectura de Template experimental (v1) con tablas genéricas `entities` + `notes`.
* [x] Carga dinámica de `schema.json` y `note_templates/soap.json` empaquetados en el binario.
* [x] Comandos genéricos `create_entity`, `get_entity`, `find_entity_by_blind_index`, `list_entities`, `search_entities`.
* [x] Comandos genéricos `create_note`, `get_note` con firma Ed25519 y verificación dinámica por template.
* [x] IDs duales: `id` INTEGER local eficiente + `external_id` UUID global para sincronización futura.
* [x] **Interfaz clínica unificada (`PatientEhrView`):** Búsqueda de paciente, ficha con evoluciones, creación de notas SOAP con firma digital, verificación en tiempo real.
* [x] **18 tests unitarios** pasando en `crypto.rs` y `vault.rs`.
* [x] **Vault con sal única por usuario** (`vault_{user_id}.salt`).

### 🟧 Fase 4: Infraestructura y Sincronización Híbrida (Próximo paso)
* [ ] Despliegue de VPS con Dokploy y PocketBase.
* [ ] **Sync Engine (`tokio`):** Background worker que hace polling de `is_synced = 0` y empuja vía HTTPS.
* [ ] Implementar Blind Index para búsqueda de nombres en el servidor (Evitar descifrado masivo en RAM).

### 🟥 Fase 5: Teleconsulta Asíncrona (Multi-Usuario)
* [ ] Tabla `asynchronous_threads` y linkage paciente-usuario.
* [ ] Cifrado asimétrico o clave de sesión compartida.

### ⬜ Fase 6: Interoperabilidad (Futuro)
* [ ] Motor de exportación JSON y traducción a estándares FHIR (IA-Driven).

---

## 🚀 Comandos Útiles de Desarrollo

```bash
pnpm tauri dev      # Levanta el entorno (Rust + React HMR)
pnpm tauri build    # Compila en modo release para producción
cargo test          # Ejecuta los 18 tests unitarios del Core
cargo add <crate>   # Añade dependencias al backend (desde src-tauri)
```

---

## 🏗️ Arquitectura de Template (Experimental — v1)

Simplex Health Core evoluciona hacia un **sistema de template** que permite adaptar el core a diferentes especialidades médicas (medicina general, veterinaria, psicología) sin modificar código Rust.

### Principio del Template

> "El core no sabe si el paciente es humano o animal. Sabe que hay entidades, atributos, relaciones y cifrado. El resto es configuración."

### Estructura de Configuración

```
config/
├── schema.json              ← Define entidades y campos (ej: paciente humano)
├── note_templates/
│   └── soap.json           ← Define plantillas de notas clínicas
└── eav_attributes.json     ← Define métricas y variables (próximo)
```

### Tablas Genéricas (Conviven con Legacy)

| Tabla | Propósito | Reemplaza a |
|-------|-----------|-------------|
| `entities` | Entidades genéricas con blob cifrado + blind index | `patients` |
| `notes` | Notas genéricas con template_id + campos cifrados | `in_person_consultations` |
| `sync_queue` | Cola FIFO de sincronización | `is_synced` en tablas individuales |

### Flujo de Datos del Template

1. **Configuración:** `schema.json` define qué campos tiene una entidad, cuál genera el blind index, cuáles son requeridos.
2. **Validación:** El core lee `schema.json` al iniciar y valida los datos dinámicamente.
3. **Cifrado:** Cada campo se cifra individualmente con AES-GCM-256 + nonce único (OsRng).
4. **Almacenamiento:** Los campos cifrados se serializan en JSON y se guardan en `enc_data_blob`.
5. **Búsqueda:** El blind index (SHA-256 + sal derivada de master_key) permite búsqueda exacta sin exponer el dato real.

### Comandos Experimentales (Template v1)

| Comando | Tabla | Estado |
|---------|-------|--------|
| `create_entity` | `entities` | ✅ Funcional |
| `get_entity` | `entities` | ✅ Funcional |
| `find_entity_by_blind_index` | `entities` | ✅ Funcional |
| `list_entities` | `entities` | ✅ Funcional |
| `search_entities` | `entities` | ✅ Funcional |
| `create_note` | `notes` | ✅ Funcional + Firma Digital |
| `get_note` | `notes` | ✅ Funcional + Verificación de Firma |
| `get_notes_by_entity` | `notes` | ✅ Funcional (listado por paciente) |

### Convivencia con Legacy

Las tablas y comandos legacy (`patients`, `in_person_consultations`, etc.) siguen funcionando. La migración es gradual: las nuevas funcionalidades usan las tablas genéricas; las existentes mantienen las tablas fijas hasta su deprecación.

### IDs Duales: Local + Global

Cada entidad y nota tiene dos identificadores:

| Campo | Tipo | Propósito |
|-------|------|-----------|
| `id` | `INTEGER PRIMARY KEY AUTOINCREMENT` | Identificador local eficiente para índices y foreign keys en SQLite. |
| `external_id` | `TEXT UUID` | Identificador global único para sincronización entre dispositivos y servidor. |

---

## 🧩 Arquitectura de Plugins (Futuro)

El sistema no es una aplicación monolítica, sino un **Microkernel (Core) altamente seguro y optimizado**, diseñado para ser extendido mediante un sistema de Plugins aislados.

### Principios Fundamentales
* **Aislamiento de Errores (Resilience):** Si un plugin de IA falla, el médico sigue escribiendo notas.
* **Carga Bajo Demanda:** Los plugins se activan mediante `plugins.json`.
* **Aislamiento de Datos:** Los plugins gestionan sus propios archivos SQLite locales sin afectar la base de datos principal.
* **Cumplimiento Zero-Knowledge:** Los plugins **nunca** tienen acceso a las claves maestras.

### Categorías de Plugins Planeadas

| Categoría | Ejemplos | Modelo de negocio |
|-----------|----------|-------------------|
| **Terminología** | LOINC, SNOMED CT, ICD-10 | Suscripción anual |
| **Integración** | PAMI, OSDE, SISA | Por transacción o suscripción |
| **Exportación** | FHIR, HL7 v2, PDF | Licencia institucional |
| **IA Médica** | Sugeridor LOINC, OCR | Por uso o suscripción |

---

## 📋 Modelo de Negocio

* **Core:** Implementación básica económica (accesible)
* **Plugins:** Cobros por funcionalidad específica (monetización)

---

## 📄 Licencia

Privado — En evaluación para futura apertura selectiva del Core bajo AGPL-3.0.


---
