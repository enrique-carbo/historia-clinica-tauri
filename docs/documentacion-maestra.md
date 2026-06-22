# Documentación Maestra de Referencia: Ecosistema Rust, Tauri v2 y Frontend

Este archivo concentra todos los enlaces oficiales, manuales y guías de arquitectura necesarios para el desarrollo del Gestor de Finanzas Personales. Actúa como el índice central de conocimiento para consultas de sintaxis, buenas prácticas y configuración del entorno.

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

## 🔌 SECTION 2: Arquitectura y Ecosistema de Plugins en Tauri v2

La v2 de Tauri delega las capacidades del sistema en plugins independientes. Estos enlaces cubren su funcionamiento y el sistema de permisos de seguridad (*capabilities*).

* **Tauri v2 Plugins Overview:** https://v2.tauri.app/plugin/
  * *Uso:* Entender el ciclo de vida de los plugins y cómo exponen comandos hacia el frontend.
* **Tauri Plugin SQL (SQLite):** https://github.com/tauri-apps/plugins-workspace/tree/v2/plugins/sql
  * *Uso:* Configuración de la base de datos embebida local para guardar transacciones y categorías.
* **Tauri Plugin Store:** https://github.com/tauri-apps/plugins-workspace/tree/v2/plugins/store
  * *Uso:* Almacenamiento rápido persistente de tipo Clave-Valor para configuraciones de usuario (Modo oscuro, moneda por defecto).
* **Tauri Plugin Notification:** https://github.com/tauri-apps/plugins-workspace/tree/v2/plugins/notification
  * *Uso:* Disparar alertas nativas del sistema operativo cuando se superen presupuestos financieros.
* **Tauri Plugin Dialog:** https://github.com/tauri-apps/plugins-workspace/tree/v2/plugins/dialog
  * *Uso:* Apertura de ventanas nativas para confirmar acciones críticas o guardar archivos de exportación (CSV).

---

## 🦀 SECTION 3: Fundamentos y Libros de Aprendizaje de Rust

Recursos canónicos e interactivos para dominar el manejo de memoria en Rust, el sistema de tipos y la gestión de errores.

* **The Rust Programming Language (Versión Interactiva de Brown University):** https://rust-book.cs.brown.edu/
  * *Uso:* Ideal para extraer analogías didácticas y explicaciones visuales sobre *Ownership*, *Borrow Checker* y referencias mutables.
* **The Rust Programming Language (Libro Oficial Estable):** https://doc.rust-lang.org/stable/book/
  * *Uso:* Manual de referencia estándar para sintaxis, colecciones (`Vector`, `HashMap`), enums complejos y control de errores (`Result`, `Option`).
* **Rust por Ejemplo (Rust by Example):** https://doc.rust-lang.org/rust-by-example/
  * *Uso:* Ejemplos de código directos e idiomáticos listos para aplicar en lógica financiera.
* **Standard Library Docs (std):** https://doc.rust-lang.org/std/
  * *Uso:* Consulta de APIs nativas de Rust para estructuras de datos y utilidades básicas.

---

## 🏗️ SECTION 4: Estructura de Aplicaciones y Buenas Prácticas (La Trilogía Lborb)

Guías de trinchera para organizar el backend de Rust con estándares profesionales de la industria, manejo de errores en producción y formateo.

* **The Little Book of Rust Books (Convenciones Oficiales):** https://lborb.github.io/book/official.html
  * *Uso:* Organización estándar de módulos, nomenclatura de funciones y estructura idiomática de crates.
* **The Little Book of Rust Books (Herramientas No Oficiales):** https://lborb.github.io/book/unofficial.html
  * *Uso:* Configuración de linters avanzados (`clippy`) y formateadores (`rustfmt`) para mantener el código de los alumnos impecable.
* **The Little Book of Rust Books (Estructura de Aplicaciones):** https://lborb.github.io/book/applications.html
  * *Uso:* Patrones de diseño para binarios ejecutables, aislamiento de la lógica de negocio y configuración de un sistema de registros/logs (esencial para diagnosticar fallas en la base de datos).

---

## 🌐 SECTION 5: Frontend, Estilos y Bundlers

Documentación del ecosistema web que correrá dentro de la WebView de Tauri para dar vida a la interfaz visual.

* **Vite.js Documentation:** https://vite.dev
  * *Uso:* Configuración del servidor de desarrollo rápido y variables de entorno del cliente.
* **Tailwind CSS Docs:** https://tailwindcss.com/docs
  * *Uso:* Estilado ágil, componentes responsivos y maquetación de la UI minimalista del Gestor de Finanzas.

---

## 📋 SECTION 6: Instrucciones de Contexto para el Modelo de IA (Prompting)

Instrucciones explícitas para cuando el usuario pida generar contenido utilizando este archivo como fuente:
1. Al responder sobre conceptos core de Rust (como Ownership), prioriza el estilo pedagógico interactivo de la fuente de la **Universidad de Brown (Section 3)**.
2. Al diseñar la arquitectura del directorio `src-tauri`, sigue rigurosamente los patrones de la guía de **Estructura de Aplicaciones (Section 4)**, separando la lógica de negocio de los comandos de Tauri.
3. Asegúrate de verificar siempre las especificaciones de la **v2 de Tauri (Section 1 y 2)**, ya que los comandos y la inicialización de la app cambiaron respecto a la v1.