
# 🩺 Simplex Health Core — Historia Clínica Digital

Aplicación de escritorio nativa orientada a la gestión de historias clínicas y telemedicina asíncrona con un enfoque **Local-First** y arquitectura de seguridad **Zero-Knowledge (Cifrado de Extremo a Extremo)**.

El sistema garantiza la máxima privacidad procesando y cifrando toda la información médica sensible directamente en la RAM del hardware local antes de persistirla en el disco o sincronizarla. El servidor remoto actuará en el futuro únicamente como un "tubo ciego" que almacena texto cifrado, sin capacidad de leer la información.

---

## ⚡ Stack Tecnológico

La arquitectura está dividida de forma estricta en dos capas de alto rendimiento:

* **Backend Nativo (Core - Rust):**
  * **Rust:** Motor principal encargado de la criptografía, seguridad y acceso a datos.
  * **Tauri v2:** Puente IPC multiplataforma de consumo ultra-bajo.
  * **SQLite (`rusqlite`):** Base de datos relacional embebida, garantizando autonomía total *offline*.
  * **AES-GCM-256:** Cifrado autenticado para texto médico (Campos SOAP, Nombres).
  * **Argon2id:** Hashing de contraseñas y derivación de claves (KDF).
* **Frontend (UI):**
  * **React + TypeScript:** Interfaz declarativa, tipada y reactiva.
  * **Tailwind CSS:** Framework de utilidades CSS para diseño rápido, consistente y responsivo.
  * **Vite:** Entorno de compilación rápida con HMR (Hot Module Replacement).
  * **pnpm:** Gestor de paquetes eficiente mediante *hard links*.

---

## 📂 Arquitectura del Proyecto

### Backend Core (Rust)
Diseño modular estricto para evitar dependencias circulares. El acceso a la base de datos y a la criptografía está aislado mediante patrones de diseño avanzados.

```text
src-tauri/src/
├── main.rs              # Punto de entrada al ejecutable nativo.
├── lib.rs               # Orquestador de Tauri, inyección de estados globales (DbState, CryptoState).
├── lib_types.rs         # Contenedores de estado seguro (Mutex<Option<T>>).
│
├── vault.rs             # Gestión de Bóvedas criptográficas por usuario (Cold Start).
├── auth.rs              # Hashing Argon2id para autenticación de usuarios.
├── crypto.rs            # Motor matemático puro (AES-GCM, Nonces OsRng, SHA-256 Blind Index).
├── database.rs          # Inicialización física de SQLite y esquemas relacionales.
│
└── commands/            # Controladores de API interna (Aduana de peticiones de React).
    ├── mod.rs           # Exportación plana y Helpers (`with_conn`, `get_key`).
    ├── auth_commands.rs # Endpoints de registro, login y desbloqueo de bóveda.
    ├── patient_commands.rs # Endpoints de gestión de pacientes (CRUD cifrado).
    ├── metric_commands.rs  # Endpoints de métricas clínicas (EAV y cifrado selectivo).
    └── soap_commands.rs     # Endpoints de notas médicas SOAP (Cifrado por campo).
```

### Frontend UI (React)
Implementación de un **Design System** propio basado en componentes atómicos reutilizables, separando la lógica de negocio de la presentación visual.

```text
src/components/
├── ui/                  # Design System Atómico (Sin lógica de negocio)
│   ├── Button.tsx        # Botones con variantes (Primary, Ghost, Danger) y estados de carga.
│   ├── Card.tsx          # Contenedores estandarizados con estados hover opcionales.
│   ├── Input.tsx         # Inputs de texto y contraseñas estilizados.
│   ├── Textarea.tsx      # Áreas de texto redimensionables.
│   └── Alert.tsx         # Alertas visuales contextuales (Error, Success, Info).
│
├── AuthBox.tsx           # Flujo de autenticación y creación de bóvedas.
├── PatientManager.tsx    # Admisión y padrón de pacientes.
├── PatientSelector.tsx   # Buscador relacional Zero-Knowledge con debounce.
├── SoapForm.tsx          # Redacción de evoluciones clínicas (Cifrado RAM -> Disco).
├── SoapHistory.tsx       # Lectura y visualización del historial (Disco -> RAM -> UI).
├── MetricQuickForm.tsx  # Registro rápido de variables clínicas numéricas.
└── MetricViewer.tsx     # Listado crudo de métricas desidentificadas.
```

### Principios de Diseño del Core
1. **Patrón `with_conn`:** Ningún comando saca la conexión a la base de datos de su `Mutex`. Se le inyecta un *closure* para garantizar que el lock se libere instantáneamente, evitando congelamientos de UI.
2. **Inyección de Dependencias:** Los comandos que requieren cifrado reciben `State<'_, CryptoState>`, extraen la llave maestra y si el Vault no fue desbloqueado, fallan de forma segura.
3. **UI Desacoplada:** Los componentes de `/ui` no conocen la estructura de la base de datos ni las rutas de la API, solo reciben `props` primitivas.

---

## 🔒 Especificación de Seguridad (Zero-Knowledge & Vault)

### 1. Ciclo de Vida de la Clave Maestra (Cold Start)
El sistema no utiliza claves hardcodeadas. La clave de cifrado nace y muere en la RAM:
1. El usuario ingresa su contraseña en la UI.
2. Rust verifica el hash en la tabla `users` (Argon2id).
3. La misma contraseña se usa como semilla para derivar 32 bytes mediante Argon2id en modo KDF.
4. Esos 32 bytes se inyectan en el `CryptoState` (RAM volátil).
5. Al cerrar la app, la RAM se destruye. La clave deja de existir.

### 2. Aislamiento Multi-Usuario (Vaults)
Cada médico tiene su propio archivo de bóveda (`vault_{user_id}.bin`). Las notas cifradas por el Dr. House no pueden ser descifradas por la Dra. Cameron (aislamiento criptográfico por defecto) sin necesidad de lógica de permisos compleja.

### 3. Cifrado Clínico Granular (SOAP) y Desidentificación (EAV)
* **Texto (SOAP, Nombres):** Cifrado independiente por campo con un **Nonce único de 12 bytes** (`OsRng`) por inserción.
* **Métricas (EAV):** Variables numéricas (`value_num`) almacenadas en texto plano desidentificado para permitir cálculos matemáticos y gráficos instantáneos (<1ms) sin comprometer la identidad del paciente. Notas opcionales van cifradas.

### 4. Prevención de Duplicados (Blind Index)
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

### 🟨 Fase 3: Métricas Clínicas (EAV Local) (¡Completado!)
* [x] Comandos de inserción/lectura para `patient_metrics`.
* [x] Cifrado selectivo (Numéricas en claro, notas cifradas).
* [x] Interfaz rápida de registro y listado de variables.

### 🟧 Fase 4: Infraestructura y Sincronización Híbrida (Próximo paso)
* [ ] Despliegue de VPS con Dokploy y PocketBase.
* [ ] **Sync Engine (`tokio`):** Background worker que hace polling de `is_synced = 0` y empuja vía HTTPS.

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
