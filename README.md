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
├── vault.rs             # Gestión de Bóvedas criptográficas por usuario (Cold Start + Llave Privada de Firma).
├── auth.rs              # Hashing Argon2id para autenticación de usuarios.
├── crypto.rs            # Motor matemático puro (AES-GCM, Ed25519, Nonces OsRng, SHA-256 Blind Index).
├── database.rs          # Inicialización física de SQLite (WAL Mode activado) y esquemas relacionales.
│
└── commands/            # Controladores de API interna (Aduana de peticiones de React).
    ├── mod.rs           # Exportación plana y Helpers (`with_conn`, `get_key`).
    ├── auth_commands.rs # Endpoints de registro, login y desbloqueo de bóveda.
    ├── patient_commands.rs # Endpoints de gestión de pacientes (Blob Cifrado + Índices Ciegos).
    ├── metric_commands.rs  # Endpoints de métricas clínicas (EAV y cifrado selectivo).
    └── soap_commands.rs     # Endpoints de notas médicas SOAP (Cifrado por campo + Firma Digital).
```

### Frontend UI (React)
Implementación de un **Design System** propio basado en componentes atómicos, completamente desacoplados del estado de la aplicación mediante **Zustand**.

```text
src/
├── stores/                 # Capa de Estado Global (Zustand)
│   ├── useAuthStore.ts     # Estado de la Bóveda (Cold Start UI, cierre seguro de sesión).
│   ├── usePatientStore.ts  # Estado de la Consulta Activa (Historial SOAP, Métricas EAV).
│   └── usePatientRegistryStore.ts # Estado del Padrón (Optimistic UI al dar de alta).
│
├── components/
│   ├──layouts/
│   │   └── DashboardLayout.tsx
│   ├── ui/                 # Design System Atómico (Sin lógica de negocio ni llamadas a Tauri)
│   │   ├── Button.tsx      # Botones con variantes y estados de carga (isLoading).
│   │   ├── Card.tsx        # Contenedores estandarizados.
│   │   ├── Input.tsx       # Inputs de texto y contraseñas estilizados.
│   │   ├── Textarea.tsx    # Áreas de texto redimensionables.
│   │   └── Alert.tsx       # Alertas visuales contextuales.
│   ├── views/
│   │   ├── AdminView.tsx
│   │   ├── MedicoView.tsx
│   │   └── PacienteView.tsx
│   │
│   ├── AuthBox.tsx         # Flujo de autenticación (Desacoplado, actualiza Zustand directamente).
│   ├── PatientManager.tsx  # Admisión y padrón (Actualiza Registry Store vía Optimistic UI).
│   ├── PatientSelector.tsx # Buscador relacional con debounce (Event-Driven hacia los Stores).
│   ├── SoapForm.tsx        # Redacción SOAP (Cifrado RAM -> Disco + Firma en RAM).
│   ├── SoapHistory.tsx     # Historial (Verificación de firma criptográfica en tiempo real).
│   ├── MetricQuickForm.tsx # Registro rápido EAV (Reseteo inteligente de dependencias).
│   └── MetricViewer.tsx    # Listado de métricas desidentificadas (Lectura directa del Store).
```

### Principios de Diseño del Core & Frontend
1. **Patrón `with_conn`:** Ningún comando saca la conexión a la base de datos de su `Mutex`. Se le inyecta un *closure* para garantizar que el lock se libere instantáneamente, evitando congelamientos de UI.
2. **Inyección de Dependencias:** Los comandos que requieren cifrado reciben `State<'_, CryptoState>`, extraen la llave maestra y si el Vault no fue desbloqueado, fallan de forma segura.
3. **Estado Reactivo Seguro (Zustand):** El Frontend jamás almacena contraseñas ni claves de cifrado en JavaScript. Los Stores solo contienen booleanos de estado (ej: `isVaultUnlocked`) y datos descifrados para renderizar.
4. **Optimistic UI:** Las inserciones (ej: nuevo paciente) se inyectan en el Store al instante sin esperar un nuevo `SELECT` de SQLite, logrando tiempos de respuesta de 0ms en la UI.
5. **UI Desacoplada:** Los componentes de `/ui` no conocen la estructura de la base de datos ni las rutas de la API, solo reciben `props` primitivas.

---

## 🔒 Especificación de Seguridad (Zero-Knowledge & Vault)

### 1. Ciclo de Vida de las Claves (Cold Start)
El sistema no utiliza claves hardcodeadas. Las claves nacen y mueren en la RAM:
1. El usuario ingresa su contraseña en la UI.
2. React llama a `unlock_vault`. Rust verifica el hash en la tabla `users` (Argon2id).
3. La misma contraseña se usa como semilla para derivar 32 bytes mediante Argon2id en modo KDF (Llave Maestra).
4. Se desbloquea el archivo `vault_{user_id}.bin`, el cual contiene la Llave Privada de Firma (Ed25519) cifrada con la Llave Maestra.
5. Ambas llaves se inyectan en el `CryptoState` y `SigningState` (RAM volátil de Rust).
6. Al cerrar sesión, React llama a `lock_vault`. Rust destruye los `Option` de la RAM. Las claves dejan de existir.

### 2. Aislamiento Multi-Usuario (Vaults)
Cada médico tiene su propio archivo de bóveda (`vault_{user_id}.bin`). Las notas cifradas por el Dr. House no pueden ser descifradas por la Dra. Cameron (aislamiento criptográfico por defecto) sin necesidad de lógica de permisos compleja.

### 3. Patrón "Blob Cifrado" en Pacientes
Para evitar consultas lentas descifrando múltiples columnas, los datos demográficos del paciente (DNI, fecha de nacimiento, mail, dirección) se empaquetan en un JSON, se cifran como un solo bloque (`encrypted_data_blob`) y se guardan en SQLite. Solo el Nombre y los Índices Ciegos se mantienen separados para rendimiento del listado.

### 4. Cifrado Clínico Granular (SOAP) y Desidentificación (EAV)
* **Texto (SOAP, Nombres):** Cifrado independiente por campo con un **Nonce único de 12 bytes** (`OsRng`) por inserción.
* **Métricas (EAV):** Variables numéricas (`value_num`) almacenadas en texto plano desidentificado para permitir cálculos matemáticos y gráficos instantáneos (<1ms) sin comprometer la identidad del paciente. Notas opcionales van cifradas.

### 5. Firma Digital y No-Repudio (Auditoría)
Cada vez que un médico guarda una nota SOAP, el Core concatena el texto plano, calcula un Hash SHA-256 y lo firma usando su Llave Privada (Ed25519) en RAM. La firma se guarda en la base de datos. Al leer la nota, el sistema utiliza la Llave Pública del médico para verificar matemáticamente que el documento no ha sido alterado desde su creación.

### 6. Prevención de Duplicados (Blind Index)
Hash determinista **SHA-256** (DNI + `BLIND_INDEX_SALT`) para rechazar registros duplicados mediante `UNIQUE` en SQLite **sin conocer la identidad real del paciente**.

---

## 🗺️ Mapa de Ruta del Desarrollo (Roadmap)

### 🟩 Fase 1: Almacenamiento Local y Autenticación (¡Completado!)
* [x] Entorno Tauri v2 + pnpm + React + Tailwind CSS.
* [x] Sistema de Vault Local (`vault.rs`) y eliminación de claves hardcodeadas.
* [x] Refactorización a módulos con patrón `with_conn`.

### 🟩 Fase 2: Robustez del Core Local (¡Completado!)
* [x] Índice Ciego (Blind Index) y Buscador Relacional Clínico.
* [x] Creación de Design System propio (`/ui`).

### 🟩 Fase 3: Métricas, Firmas Digitales y Arquitectura Frontend (¡Completado!)
* [x] Comandos de inserción/lectura para `patient_metrics`.
* [x] Cifrado selectivo (Numéricas en claro, notas cifradas).
* [x] Interfaz rápida de registro y listado de variables.
* [x] Migración del estado a **Zustand** (Stores de Auth, Patient y Registry).
* [x] Implementación de **Optimistic UI** y tipado estricto End-to-End.
* [x] Implementación de **Firma Digital Ed25519** para notas SOAP (No-Repudio).
* [x] Refactor a Patrón "Blob Cifrado" para datos demográficos de pacientes.

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
cargo add <crate>   # Añade dependencias al backend (desde src-tauri)
```
