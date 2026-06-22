# 🩺 Historia Clínica Digital — Simplex Health Core

Aplicación de escritorio nativa orientada a la gestión de historias clínicas y telemedicina asíncrona con un enfoque **Local-First** y arquitectura de seguridad **Zero-Knowledge (Cifrado de Extremo a Extremo)**.

El sistema garantiza la máxima privacidad procesando y cifrando toda la información médica sensible directamente en la RAM del hardware local antes de persistirla en el disco o sincronizarla. El servidor remoto (PocketBase) actúa únicamente como un "tubo ciego" que almacena texto cifrado, sin capacidad de leer la información.

---

## ⚡ Stack Tecnológico

La arquitectura está dividida de forma estricta en dos capas de alto rendimiento:

* **Backend Nativo (Core - Rust):**
  * **Rust:** Motor principal encargado de la criptografía, seguridad y acceso a datos.
  * **Tauri v2:** Puente IPC (Inter-Process Communication) multiplataforma de consumo ultra-bajo.
  * **SQLite (`rusqlite`):** Base de datos relacional embebida, garantizando autonomía total *offline*.
  * **AES-GCM-256:** Cifrado autenticado para texto médico (Campos SOAP, Nombres).
  * **Argon2id:** Hashing de contraseñas y derivación de claves (KDF).
* **Frontend (UI):**
  * **React + TypeScript:** Interfaz declarativa, tipada y reactiva.
  * **Vite:** Entorno de compilación rápida con HMR (Hot Module Replacement).
  * **pnpm:** Gestor de paquetes eficiente mediante *hard links*.

---

## 📂 Arquitectura del Proyecto (Backend Core)

El backend implementa un diseño modular estricto para evitar dependencias circulares. El acceso a la base de datos y a la criptografía está aislado mediante patrones de diseño avanzados.

```text
src-tauri/src/
├── main.rs              # Punto de entrada al ejecutable nativo.
├── lib.rs               # Orquestador de Tauri, inyección de estados globales (DbState, CryptoState).
├── lib_types.rs         # Contenedores de estado seguro (Mutex<Option<T>>).
│
├── vault.rs             # [FASE 1] Gestión de Bóvedas criptográficas por usuario (Cold Start).
├── auth.rs              # [FASE 1] Hashing Argon2id para autenticación de usuarios.
├── crypto.rs            # Motor matemático puro (AES-GCM, Nonces, SHA-256 Blind Index).
├── database.rs          # Inicialización física de SQLite y esquemas relacionales.
│
└── commands/            # Controladores de API interna (Aduana de peticiones de React).
    ├── mod.rs           # Exportación plana y Helpers (`with_conn`, `get_key`).
    ├── auth_commands.rs # Endpoints de registro, login y desbloqueo de bóveda.
    ├── patient_commands.rs # Endpoints de gestión de pacientes (CRUD cifrado).
    └── soap_commands.rs     # Endpoints de notas médicas SOAP (Cifrado por campo).
```

### Principios de Diseño del Core
1. **Patrón `with_conn`:** Ningún comando saca la conexión a la base de datos de su `Mutex`. Se le inyecta un *closure* para garantizar que el lock se libere instantáneamente después de la query, evitando congelamientos de UI.
2. **Inyección de Dependencias:** Los comandos que requieren cifrado reciben `State<'_, CryptoState>`, extraen la llave maestra y si el Vault no fue desbloqueado, fallan de forma segura.

---

## 🔒 Especificación de Seguridad (Zero-Knowledge & Vault)

### 1. Ciclo de Vida de la Clave Maestra (Cold Start)
El sistema no utiliza claves hardcodeadas. La clave de cifrado de toda la aplicación nace y muere en la RAM:
1. El usuario ingresa su contraseña en la UI.
2. Rust verifica el hash en la tabla `users` (Argon2id).
3. La misma contraseña se usa como semilla para derivar 32 bytes mediante Argon2id en modo KDF.
4. Esos 32 bytes se inyectan en el `CryptoState` (RAM volátil).
5. Al cerrar la app, la RAM se destruye. La clave deja de existir.

### 2. Aislamiento Multi-Usuario (Vaults)
En entornos de policonsultorio, cada médico tiene su propio archivo de bóveda (`vault_{user_id}.bin`).
* Si el Dr. House cifra una nota, utiliza su llave derivada.
* Si la Dra. Cameron intenta leer esa misma nota, el descifrado fallará (Error AES-GCM), aislando las consultas de forma criptográfica por defecto.

### 3. Cifrado Clínico Granular (SOAP)
Cada bloque del registro clínico (Subjetivo, Objetivo, Análisis, Plan) se cifra de forma independiente con un **Nonce único de 12 bytes** generado por `OsRng`. Esto evita ataques de análisis de patrones en el disco duro.

### 4. Prevención de Duplicados (Blind Index)
Se genera un hash determinista **SHA-256** combinando el DNI del paciente y una `BLIND_INDEX_SALT`. La base de datos puede rechazar duplicados mediante `UNIQUE` sobre este hash, **sin conocer jamás la identidad real del paciente**.

---

## 🗺️ Mapa de Ruta del Desarrollo (Roadmap)

### 🟩 Fase 1: Almacenamiento Local y Autenticación (¡Completado!)
* [x] Inicialización del entorno híbrido (Tauri v2 + pnpm + React).
* [x] Integración de criptografía (`aes-gcm`, `rand`, `argon2`, `hex`).
* [x] Sistema de registro y login local con roles (`admin`, `medico`, `paciente`).
* [x] **Sistema de Vault Local (`vault.rs`):** Derivación de clave maestra desde la contraseña del usuario y archivo validador `.bin`.
* [x] **Refactorización Arquitectónica:** Migración de `commands.rs` monolítico a módulos separados con patrón `with_conn` para control estricto de Mutex.
* [x] Eliminación completa de claves de cifrado hardcodeadas (`MOCK_MASTER_KEY`).

### 🟩 Fase 2: Robustez del Core Local y Relaciones (¡Completado!)
* [x] Índice Ciego (Blind Index) para control de unicidad Zero-Knowledge.
* [x] Buscador relacional clínico (Descifrado en RAM y filtrado en Rust).
* [x] Flujo completo de prueba: Crear paciente -> Redactar nota SOAP -> Cerrar app -> Reabrir -> Descifrar exitoso.

### 🟨 Fase 3: Métricas Clínicas (EAV Local) (Próximo paso)
* [ ] Comandos de inserción/lectura para la tabla `patient_metrics` (Entity-Attribute-Value).
* [ ] Lógica de cifrado selectivo: Valores numéricos (`REAL`) en claro para gráficos, notas opcionales (`TEXT`) cifradas con la llave del `CryptoState`.
* [ ] Integración de librería de gráficos en React (ej. Recharts) para mostrar evolución de peso/presión.

### 🟧 Fase 4: Infraestructura y Sincronización Híbrida
* [ ] Despliegue de VPS con Dokploy y PocketBase.
* [ ] Configuración de colecciones "Ciegas" en PocketBase (campos llamados `ciphertext` y `nonce`).
* [ ] **Sync Engine (`tokio`):** Background worker en Rust que hace polling de registros `is_synced = 0`, los empuja vía HTTPS (`reqwest`) y actualiza el flag local a `1`.

### 🟥 Fase 5: Teleconsulta Asíncrona (Multi-Usuario)
* [ ] Creación de tabla `asynchronous_threads` (Hilos de conversación por motivo médico).
* [ ] Flujo de linkage: Vincular un usuario de rol `paciente` con un ID de la tabla `patients`.
* [ ] **El gran desafío criptográfico:** Implementar cifrado asimétrico (o clave de sesión compartida) para que el médico y el paciente puedan desencriptar el mismo hilo desde dispositivos distintos.

### ⬜ Fase 6: Interoperabilidad (Futuro)
* [ ] Motor de exportación en Rust que consolide el historial en un "Paciente Index" JSON.
* [ ] Puente a estándares internacionales HL7 v2/v3 o FHIR (potencialmente asistido por IA local).

---

## 🚀 Comandos Útiles de Desarrollo

Para levantar el entorno (compila Rust en segundo plano e inicia HMR de React):
```bash
pnpm tauri dev
```

Para añadir dependencias al frontend:
```bash
pnpm add <nombre-paquete>
```

Para añadir dependencias al backend criptográfico (ejecutar dentro de `src-tauri`):
```bash
cargo add <nombre-crate>
```

Para compilar en modo release (optimizado para producción):
```bash
pnpm tauri build
```
