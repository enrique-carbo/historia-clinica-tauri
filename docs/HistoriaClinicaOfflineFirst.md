# 🩺 Simplex Health Core — Historia Clínica Digital

Aplicación de escritorio nativa orientada a la gestión de historias clínicas con un enfoque **Local-First** y arquitectura de seguridad **Zero-Knowledge (Cifrado de Extremo a Extremo)**.

El sistema garantiza la máxima privacidad del paciente procesando y cifrando toda la información médica sensible directamente en la RAM del hardware local antes de persistirla en el disco o sincronizarla. El servidor remoto actuará en el futuro únicamente como un "tubo ciego" que almacena texto cifrado.

> **Nota:** Para la visión completa del producto (Teleconsulta asíncrona, BIP-39, Backups en R2, Exportación FHIR), consultar el documento [VISION_Y_ARQUITECTURA_FUTURA.md](./VISION_Y_ARQUITECTURA_FUTURA.md).

---

## ⚡ Stack Tecnológico

* **Backend Nativo (Core):** Rust + Tauri v2 + SQLite (`rusqlite`).
* **Criptografía:** AES-GCM-256 (Cifrado de campos) + Argon2id (Hashing de Auth y KDF para llaves).
* **Frontend (UI):** React + TypeScript + Vite.

---

## 📂 Arquitectura del Proyecto (Estado Actual)

El backend implementa un diseño modular estricto. El acceso a la base de datos y la criptografía están aislados mediante patrones avanzados de Rust para evitar bloqueos en la UI.

```text
src-tauri/src/
├── main.rs              # Punto de entrada al ejecutable nativo.
├── lib.rs               # Orquestador de Tauri, inyección de estados (DbState, CryptoState).
├── lib_types.rs         # Contenedores de estado seguro (Mutex<Option<T>>).
│
├── vault.rs             # Gestión de Bóvedas criptográficas por usuario (Derivación KDF).
├── auth.rs              # Hashing Argon2id para autenticación de usuarios locales.
├── crypto.rs            # Motor matemático puro (AES-GCM, Nonces OsRng, SHA-256 Blind Index).
├── database.rs          # Inicialización física de SQLite y esquemas relacionales.
│
└── commands/            # Controladores de API interna (Puente con React).
    ├── mod.rs           # Exportación plana y Helpers de seguridad (`with_conn`, `get_key`).
    ├── auth_commands.rs # Endpoints: Registro, Login y Desbloqueo de Bóveda.
    ├── patient_commands.rs # Endpoints: Gestión de pacientes (CRUD totalmente cifrado).
    └── soap_commands.rs     # Endpoints: Notas médicas SOAP (Cifrado granular por campo).
```

---

## 🔒 Especificación de Seguridad (Implementada)

### 1. Ciclo de Vida de la Clave Maestra (Cold Start)
El sistema no utiliza claves hardcodeadas. La clave de cifrado nace y muere en la RAM:
1. El usuario ingresa su contraseña en React.
2. Rust verifica el hash en la tabla `users` (Argon2id).
3. La misma contraseña deriva 32 bytes (Argon2id en modo KDF) que se inyectan en el `CryptoState`.
4. Al cerrar la app, la RAM se destruye. La clave deja de existir.

### 2. Aislamiento Multi-Usuario (Vaults)
Cada médico tiene su propio archivo de bóveda (`vault_{user_id}.bin`). Las notas cifradas por el Dr. House no pueden ser descifradas por la Dra. Cameron (aislamiento criptográfico por defecto).

### 3. Cifrado Clínico Granular
Cada bloque del registro SOAP se cifra de forma independiente con un **Nonce único de 12 bytes** generado por `OsRng`.

### 4. Prevención de Duplicados (Blind Index)
Hash determinista **SHA-256** del DNI + una sal secreta. Permite rechazar duplicados mediante `UNIQUE` en SQLite sin conocer la identidad real del paciente.

---

## 🗺️ Mapa de Ruta del Desarrollo (Roadmap Real)

### 🟩 Fase 1: Almacenamiento Local y Autenticación (¡Completado!)
* [x] Entorno Tauri v2 + pnpm + React.
* [x] Sistema de registro y login local con roles (`admin`, `medico`, `paciente`).
* [x] **Sistema de Vault Local (`vault.rs`):** Derivación de clave maestra desde la contraseña y archivo validador `.bin`.
* [x] **Refactorización Arquitectónica:** Módulos separados y patrón `with_conn` para control estricto de Mutex.
* [x] Eliminación de claves de cifrado hardcodeadas.

### 🟩 Fase 2: Robustez del Core Local (¡Completado!)
* [x] Índice Ciego (Blind Index).
* [x] Buscador relacional clínico (Descifrado en RAM y filtrado en Rust).
* [x] Flujo completo: Crear paciente -> Redactar nota SOAP -> Cerrar app -> Reabrir -> Descifrar exitoso.

### 🟨 Fase 3: Métricas Clínicas (EAV Local) (En Progreso)
* [ ] Comandos de inserción/lectura para `patient_metrics` (Entity-Attribute-Value).
* [ ] Cifrado selectivo: Valores numéricos (`REAL`) en claro, notas opcionales (`TEXT`) cifradas.
* [ ] Integración de gráficos en React (ej. Recharts) para evolución de peso/presión.

### 🟧 Fase 4: Sincronización Híbrida (PocketBase)
* [ ] Despliegue de VPS con Dokploy y PocketBase.
* [ ] **Sync Engine (`tokio`):** Background worker que hace polling de `is_synced = 0` y empuja vía `reqwest`.

### 🟥 Fase 5: Teleconsulta Asíncrona
* [ ] Creación de tabla `asynchronous_threads`.
* [ ] Cifrado asimétrico o clave de sesión compartida entre médico y paciente.

---

## 🚀 Comandos de Desarrollo

```bash
pnpm tauri dev      # Levanta el entorno (Rust + React HMR)
pnpm tauri build    # Compila en modo release para producción
cargo add <crate>   # Añade dependencias al backend (desde src-tauri)
```
