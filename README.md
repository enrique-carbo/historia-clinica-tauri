# 🩺 Historia Clinica Digital  — Core App

Aplicación de escritorio nativa orientada a la gestión de historias clínicas y métricas de salud con un enfoque **Local-First** y arquitectura de seguridad **Zero-Knowledge (Cifrado de Extremo a Extremo)**.

El sistema garantiza la máxima privacidad del paciente procesando y cifrando toda la información médica sensible directamente en el hardware local (RAM) antes de persistirla en el disco rígido o sincronizarla con la nube.

---

## ⚡ Stack Tecnológico

La arquitectura está dividida de forma estricta en dos mundos de alto rendimiento:

* **Backend Nativo (Core):**
    * **Rust:** Motor principal encargado de la seguridad, criptografía y acceso a hardware.
    * **Tauri v2:** Puente de comunicación asíncrono e IPC (Inter-Process Communication) multiplataforma de consumo ultra-bajo.
    * **SQLite (Rusqlite):** Base de datos relacional integrada directamente en el binario (`bundled`), garantizando autonomía total *offline*.
* **Frontend (UI):**
    * **React + TypeScript:** Interfaz de usuario declarativa, tipada y reactiva.
    * **Vite:** Entorno de compilación rápida con HMR (Hot Module Replacement).
    * **pnpm:** Gestor de paquetes eficiente mediante enlaces rígidos (*hard links*) para optimizar el almacenamiento en disco.

---

## 📂 Arquitectura del Proyecto (Backend Core)

El backend de Rust implementa un diseño modular y desacoplado para evitar dependencias circulares y asegurar la escalabilidad:

```text
src-tauri/src/
├── main.rs          # Punto de entrada al ejecutable nativo.
├── lib.rs           # Orquestador del ciclo de vida de Tauri y registro de estados/endpoints.
├── lib_types.rs     # Estructuras de datos globales compartidas (ej. DbState).
├── commands.rs      # Controlador de API interna (Aduana de peticiones de React).
├── crypto.rs        # Módulo matemático puro de cifrado (AES-GCM-256) e índices ciegos.
├── database.rs      # Inicialización física, esquemas relacionales y queries locales (SQLite).
└── auth.rs          # Criptografía de autenticación nativa y manejo de hashes para contraseñas.

```

---

## 🔒 Especificación de Seguridad (Zero-Knowledge & Auth)

### 1. Cifrado Clínico SOAP

Cada bloque del registro clínico **SOAP** (Subjetivo, Objetivo, Análisis, Plan) y los datos filiatorios del paciente se procesan de forma **independiente e inmutable**:

* **Algoritmo:** `AES-GCM-256` (Advanced Encryption Standard con Galois/Counter Mode) para cifrado autenticado.
* **Vectores de Inicialización (Nonce):** Se generan **12 bytes aleatorios criptográficamente seguros (`rand`) por cada campo** en cada inserción. Esto evita ataques de análisis de patrones en el disco.

### 2. Control de Acceso (Auth)

* **Algoritmo:** `Argon2id` (estándar moderno ganador del Password Hashing Competition).
* Los hashes se calculan de forma aislada en la RAM al registrarse y se verifican contra SQLite protegiendo el sistema contra ataques de temporización (*timing attacks*).

### 3. Prevención de Duplicados (Blind Indexing)

Para evitar la duplicación de pacientes sin comprometer su privacidad ante el servidor remoto, se genera un **Índice Ciego**:

* Se toma el documento de identidad (DNI/Pasaporte), se normaliza y se le aplica un hash determinista **SHA-256** combinado con una sal secreta del sistema (`BLIND_INDEX_SALT`).
* La base de datos local y la nube pueden rechazar registros duplicados mediante una restricción `UNIQUE` sobre este hash, **sin conocer jamás la identidad real del paciente**.

---

## 🗺️ Mapa de Ruta del Desarrollo (Roadmap)

### 🟩 Fase 1: Almacenamiento Local Seguro (¡Completado!)

* [x] Inicialización del entorno híbrido con Tauri v2 y pnpm.
* [x] Modularización del Core en Rust (`lib.rs`, `commands.rs`, `crypto.rs`, `database.rs`).
* [x] Integración de dependencias criptográficas (`aes-gcm`, `rand@0.8`, `hex`).
* [x] Implementación del puente de persistencia autónoma con SQLite local.
* [x] Modularización de componentes React (`SoapForm`, `SoapHistory`) con flujo reactivo de lectura/escritura cifrada.

### 🟩 Fase 2: Robustez del Core Local y Relaciones (¡Completado!)

* [x] **Sistema de Llaves (Auth):** Autenticación criptográfica local con roles (`admin`, `medico`, `paciente`) utilizando `Argon2id`.
* [x] **Índice Ciego (Blind Index):** Control de unicidad de pacientes mediante hashing determinista SHA-256 para prevenir duplicados en entornos Zero-Knowledge.
* [x] **Buscador Relacional Clínico:** Componente selector que permite al médico buscar pacientes descifrados en RAM y abrir/vincular consultas SOAP sincrónicas de forma dinámica por su UUID.

### 🟨 Fase 3: Métricas y Sincronización Híbrida (Próximos pasos)

* [ ] **Métricas Clínicas (EAV):** Desarrollar la tabla Entity-Attribute-Value para registrar variables vitales numéricas (Peso, Presión Arterial) y notas opcionales encriptadas para gráficos temporales.
* [ ] **PocketBase Sync Engine:** Crear el background worker en Rust que escanee los registros locales con `is_synced = 0` y los transmita cifrados a la base de datos remota mediante HTTP seguro.

---

## 🚀 Comandos Útiles de Desarrollo

Para levantar el entorno de desarrollo local (compila Rust en segundo plano e inicia el HMR de React):

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

---
