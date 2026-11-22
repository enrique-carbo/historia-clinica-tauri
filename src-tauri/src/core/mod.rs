//! Capa de servicio del Core (`docs/core-api.md`).
//!
//! Rust puro: **no** importa `tauri`. La implementación in-process de
//! `ModuleContext` (en el `PluginHost`) toma los locks de `DbState`/`DataKey`
//! y delega acá. Mantener este módulo libre de Tauri es lo que hace que sus
//! tests corran con SQLite en memoria y sin `MockRuntime`.

pub mod api;
pub mod dto;
