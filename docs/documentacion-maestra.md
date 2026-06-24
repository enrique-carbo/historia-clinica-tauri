# Documentación Maestra de Referencia: Ecosistema Rust, Tauri v2 y Criptografía Médica

Este archivo concentra todos los enlaces oficiales, manuales y guías de arquitectura necesarios para el desarrollo de **Simplex Health Core**. 
Actúa como el índice central de conocimiento para consultas de sintaxis, buenas prácticas, configuración del entorno y seguridad criptográfica.

---

## 🚀 SECTION 1: Inicio Rápido y Core de Tauri v2

Enlaces esenciales para entender la arquitectura base, la configuración de ventanas y la instalación del entorno según el sistema operativo.

* **Sitio Oficial de Tauri:** https://tauri.app
  * *Uso:* Visión general del framework y comunidad.
* **Tauri v2 Documentation Reference:** https://v2.tauri.app
  * *Uso:* Guías conceptuales, manejo de ventanas nativas y migración a la v2.
* **Tauri v2 - Quick Start Guide:** https://v2.tauri.app/start/
  * *Uso:* Requisitos previos del sistema (MSVC en Windows, Xcode en macOS, WebKit en Linux).
* **Tauri v2 - Creación de Proyectos:** https://v2.tauri.app/start/create-project/
  * *Uso:* Uso de la CLI `create-tauri-app` para inicializar el stack React + Vite + TypeScript.
* **Tauri CLI Reference:** https://v2.tauri.app/reference/cli/
  * *Uso:* Comandos de consola para desarrollo (`dev`) y compilación (`build`).

---

## 🔌 SECTION 2: Arquitectura Zero-Knowledge y Dependencias de Rust

Simplex Health Core NO delega la seguridad a plugins de Tauri. Utiliza crates de Rust puro para garantizar un control estricto sobre la memoria (patrón `with_conn`) y el cifrado en RAM. Solo se usan plugins de Tauri para capacidades no críticas del SO.

* **`rusqlite` (SQLite embebido):** https://github.com/rusqlite/rusqlite
  * *Uso:* Motor de base de datos local offline. Se usa directamente (no el plugin Tauri-SQL) para mantener la conexión envuelta en un `Mutex` y aplicar el patrón de diseño `with_conn` que evita congelar la UI.
* **`aes-gcm` (Cifrado Autenticado):** https://docs.rs/aes-gcm/latest/aes_gcm/
  * *Uso:* Implementación de AES-GCM-256 para cifrar campos clínicos SOAP y nombres de pacientes campo por campo con Nonces únicos (`OsRng`).
* **`argon2` (Hashing y KDF):** https://docs.rs/argon2/latest/argon2/
  * *Uso:* Derivación de claves (Argon2id) a partir de la contraseña del médico para generar la clave maestra del Vault, y verificación de hashes de autenticación.
* **Tauri Plugin Dialog:** https://github.com/tauri-apps/plugins-workspace/tree/v2/plugins/dialog
  * *Uso:* Apertura de ventanas nativas exclusivamente para confirmar acciones críticas (ej. eliminación de pacientes) o guardar archivos de exportación clínica (JSON/FHIR).
* **Tauri Plugin Notification:** https://github.com/tauri-apps/plugins-workspace/tree/v2/plugins/notification
  * *Uso:* Disparar alertas nativas del sistema operativo para recordatorios de telemedicina o alertas de métricas clínicas fuera de rango.

---

## 🦀 SECTION 3: Fundamentos y Libros de Aprendizaje de Rust

Recursos canónicos e interactivos para dominar el manejo de memoria en Rust, el sistema de tipos y la gestión de errores (crucial para no paniquear el sistema de seguridad).

* **The Rust Programming Language (Versión Interactiva de Brown University):** https://rust-book.cs.brown.edu/
  * *Uso:* Ideal para extraer analogías didácticas y explicaciones visuales sobre *Ownership*, *Borrow Checker* y referencias mutables (esencial para entender por qué la clave maestra muere en la RAM y no se copia).
* **The Rust Programming Language (Libro Oficial Estable):** https://doc.rust-lang.org/stable/book/
  * *Uso:* Manual de referencia estándar para sintaxis, colecciones (`Vector`, `HashMap` para el modelo EAV), enums complejos y control de errores (`Result`, `Option`).
* **Rust por Ejemplo (Rust by Example):** https://doc.rust-lang.org/rust-by-example/
  * *Uso:* Ejemplos de código directos e idiomáticos listos para aplicar en lógica de serialización/deserialización clínica.
* **Standard Library Docs (std):** https://doc.rust-lang.org/std/
  * *Uso:* Consulta de APIs nativas de Rust para estructuras de datos y utilidades criptográficas básicas.

---

## 🏗️ SECTION 4: Estructura de Aplicaciones y Buenas Prácticas (La Trilogía Lborb)

Guías de trinchera para organizar el backend de Rust con estándares profesionales de la industria, manejo de errores en producción y formateo.

* **The Little Book of Rust Books (Convenciones Oficiales):** https://lborb.github.io/book/official.html
  * *Uso:* Organización estándar de módulos (ej. separar `commands/` de `crypto.rs`), nomenclatura de funciones y estructura idiomática de crates.
* **The Little Book of Rust Books (Herramientas No Oficiales):** https://lborb.github.io/book/unofficial.html
  * *Uso:* Configuración de linters avanzados (`clippy`) y formateadores (`rustfmt`) para mantener el código de los módulos de seguridad impecable y libre de advertencias.
* **The Little Book of Rust Books (Estructura de Aplicaciones):** https://lborb.github.io/book/applications.html
  * *Uso:* Patrones de diseño para binarios ejecutables, aislamiento estricto de la lógica de negocio (Cold Start de Vaults) y configuración de un sistema de registros/logs (esencial para auditar fallos en la apertura de bóvedas).

---

## 🌐 SECTION 5: Frontend, Estilos y Bundlers

Documentación del ecosistema web que correrá dentro de la WebView de Tauri para dar vida a la interfaz visual desacoplada.

* **Vite.js Documentation:** https://vite.dev
  * *Uso:* Configuración del servidor de desarrollo rápido (HMR) y variables de entorno del cliente.
* **Tailwind CSS Docs:** https://tailwindcss.com/docs
  * *Uso:* Estilado ágil, componentes responsivos y maquetación del Design System atómico (`/ui`) para la interfaz clínica.
* **React + TypeScript Docs:** https://react.dev/learn
  * *Uso:* Gestión de estado de formularios clínicos (SoapForm, MetricQuickForm) y tipado estricto de los `props` que reciben los componentes visuales desde los comandos de Tauri.
* **Zustand (Gestor de Estado):** https://github.com/pmndrs/zustand
  * *Uso:* Gestión de estado global ultraligera y sin boilerplate. En Simplex Health se usa para mantener el estado de las sesiones de usuario (ej. isVaultUnlocked, activePatientId) y cachear respuestas asíncronas de los comandos de Tauri, evitando re-renders innecesarios en la UI médica.

---

## 📋 SECTION 6: Instrucciones de Contexto para el Modelo de IA (Prompting)

Instrucciones explícitas para cuando el modelo de IA genere código o arquitectura utilizando este archivo como fuente:
1. **NUNCA usar `tauri-plugin-sql`**: Siempre generar código usando `rusqlite` y respetando el patrón `with_conn` con `Mutex`.
2. Al responder sobre conceptos core de Rust (como Ownership), prioriza el estilo pedagógico interactivo de la fuente de la **Universidad de Brown (Section 3)**.
3. Al diseñar la arquitectura del directorio `src-tauri`, sigue rigurosamente los patrones de la guía de **Estructura de Aplicaciones (Section 4)**, aislando `crypto.rs`, `vault.rs` y `database.rs` de la carpeta `commands/`.
4. Asegúrate de verificar siempre las especificaciones de la **v2 de Tauri (Section 1)**, ya que los comandos y la inyección de estado (`State<'_, T>`) cambiaron respecto a la v1.
5. Cualquier lógica de frontend sugerida debe mantener el principio de **UI Desacoplada**: Los componentes en `/ui` no deben importar lógica de base de datos ni estructuras de Rust, solo recibir `props` primitivas de TypeScript.


