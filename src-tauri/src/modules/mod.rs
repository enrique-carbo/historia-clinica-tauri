//! Módulos compilados en el binario (Ruta 1, `docs/module-contract.md`).
//!
//! En Ruta 1 el módulo vive como módulo interno del crate `fichio-care`
//! (paywall build-time por feature de Cargo). Cuando `core` se extraiga a un
//! crate propio, se mueve a `modules/solicitudes` sin cambiar el trait
//! `Module` (§14).

#[cfg(feature = "module-solicitudes")]
pub mod solicitudes;

use crate::core::module::Module;

/// Registro de composición de los módulos compilados.
pub fn compiled_modules() -> Vec<Box<dyn Module>> {
    #[cfg(feature = "module-solicitudes")]
    {
        vec![Box::new(solicitudes::SolicitudesModule)]
    }
    #[cfg(not(feature = "module-solicitudes"))]
    {
        Vec::new()
    }
}
