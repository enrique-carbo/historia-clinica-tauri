// Error estructurado de los commands de Tauri.
//
// Contrato con el frontend: el `invoke` rechaza con
// `{ code, params? }` — el texto para el usuario NO viaja desde Rust.
// `src/utils/errors.ts` traduce el código con su diccionario (i18n-ready,
// ver docs/i18n.md Fase 2a). Los errores internos (DB, cripto, locks)
// conservan el detalle técnico en `params["detail"]`, que el frontend
// muestra como fallback pero no se traduce.
//
// `From<String>` es la clave para migrar sin tocar los ~59
// `map_err(|e| e.to_string())?` existentes: el `?` convierte solo.
use std::collections::BTreeMap;
use std::fmt;

// --- Infraestructura ---
pub const ERR_INTERNAL: &str = "ERR_INTERNAL";
pub const ERR_STORAGE_PATH: &str = "ERR_STORAGE_PATH";
pub const ERR_DB_NOT_INITIALIZED: &str = "ERR_DB_NOT_INITIALIZED";

// --- Sesión y roles ---
pub const ERR_SESSION_REQUIRED: &str = "ERR_SESSION_REQUIRED";
pub const ERR_ADMIN_REQUIRED: &str = "ERR_ADMIN_REQUIRED";
pub const ERR_ADMIN_OR_ASSISTANT_REQUIRED: &str = "ERR_ADMIN_OR_ASSISTANT_REQUIRED";

// --- Autenticación ---
pub const ERR_REGISTRATION_CLOSED: &str = "ERR_REGISTRATION_CLOSED";
pub const ERR_USERNAME_TAKEN: &str = "ERR_USERNAME_TAKEN";
pub const ERR_USER_DISABLED: &str = "ERR_USER_DISABLED";
pub const ERR_WRONG_PASSWORD: &str = "ERR_WRONG_PASSWORD";
pub const ERR_USER_NOT_FOUND: &str = "ERR_USER_NOT_FOUND";
pub const ERR_PASSWORD_TOO_SHORT: &str = "ERR_PASSWORD_TOO_SHORT";
pub const ERR_PASSWORD_SAME_AS_CURRENT: &str = "ERR_PASSWORD_SAME_AS_CURRENT";

// --- Administración de usuarios ---
pub const ERR_USERNAME_EMPTY: &str = "ERR_USERNAME_EMPTY";
pub const ERR_INVALID_ROLE: &str = "ERR_INVALID_ROLE";
pub const ERR_SELF_DEACTIVATE: &str = "ERR_SELF_DEACTIVATE";
pub const ERR_LAST_ADMIN: &str = "ERR_LAST_ADMIN";
pub const ERR_OWN_PASSWORD_RESET: &str = "ERR_OWN_PASSWORD_RESET";

// --- Bóveda y semilla ---
pub const ERR_VAULT_LOCKED: &str = "ERR_VAULT_LOCKED";
pub const ERR_SIGNING_KEY_MISSING: &str = "ERR_SIGNING_KEY_MISSING";
pub const ERR_SEED_EMPTY: &str = "ERR_SEED_EMPTY";
pub const ERR_SEED_NOT_CONFIGURED: &str = "ERR_SEED_NOT_CONFIGURED";
pub const ERR_SEED_INVALID: &str = "ERR_SEED_INVALID";
pub const ERR_SEED_MALFORMED: &str = "ERR_SEED_MALFORMED";
pub const ERR_NO_ACTIVE_ADMIN: &str = "ERR_NO_ACTIVE_ADMIN";

// --- Fichas y accesos ---
pub const ERR_ENTITY_NOT_FOUND: &str = "ERR_ENTITY_NOT_FOUND";
pub const ERR_NOT_PATIENT_ENTITY: &str = "ERR_NOT_PATIENT_ENTITY";
pub const ERR_PATIENT_ACCESS_EXISTS: &str = "ERR_PATIENT_ACCESS_EXISTS";
pub const ERR_BLIND_INDEX_FIELD_MISSING: &str = "ERR_BLIND_INDEX_FIELD_MISSING";

// --- Entries ---
pub const ERR_INVALID_CATEGORY: &str = "ERR_INVALID_CATEGORY";
pub const ERR_INVALID_STATUS: &str = "ERR_INVALID_STATUS";
pub const ERR_PROFILE_INCOMPLETE: &str = "ERR_PROFILE_INCOMPLETE";

// --- Perfil profesional ---
pub const ERR_FOREIGN_PROFILE: &str = "ERR_FOREIGN_PROFILE";

// --- Export y respaldos ---
pub const ERR_EXPORT_EMPTY: &str = "ERR_EXPORT_EMPTY";
pub const ERR_BACKUP_SIGNATURE_INVALID: &str = "ERR_BACKUP_SIGNATURE_INVALID";
pub const ERR_BACKUP_UNVERIFIED: &str = "ERR_BACKUP_UNVERIFIED";

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CommandError {
    pub code: &'static str,
    /// Parámetros para la interpolación del texto (ej. `min` en
    /// `ERR_PASSWORD_TOO_SHORT`). BTreeMap por convención del repo:
    /// los mapas serializados deben ser deterministas.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, String>,
}

impl CommandError {
    pub fn new(code: &'static str) -> Self {
        Self {
            code,
            params: BTreeMap::new(),
        }
    }

    /// Agrega/sobrescribe un parámetro de interpolación.
    pub fn with_param(mut self, key: &str, value: impl Into<String>) -> Self {
        self.params.insert(key.to_string(), value.into());
        self
    }

    /// Error no clasificado (DB, IO, cripto, lock): el detalle técnico se
    /// conserva para diagnóstico, pero el frontend lo trata como fallback.
    pub fn internal(detail: impl Into<String>) -> Self {
        Self::new(ERR_INTERNAL).with_param("detail", detail)
    }

    /// `true` si el detalle es el marcador `SEED_REQUIRED` que emite
    /// `data_key::resolve_data_key` — el frontend pide la frase semilla.
    pub fn is_seed_required(detail: &str) -> bool {
        detail == crate::security::data_key::SEED_REQUIRED
    }
}

/// Los helpers internos (`get_key`, `get_data_key`, `record_audit`, las capas
/// de seguridad y los errores de `rusqlite` convertidos con `to_string`) siguen
/// devolviendo `String`: al propagarse con `?` se convierten en `ERR_INTERNAL`
/// sin tocar sus call sites.
impl From<String> for CommandError {
    fn from(detail: String) -> Self {
        Self::internal(detail)
    }
}

impl From<&str> for CommandError {
    fn from(detail: &str) -> Self {
        Self::internal(detail.to_string())
    }
}

/// Formato para logs y tests: `CODE` + detalle/params si existen.
impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code)?;
        let mut first = true;
        for (k, v) in &self.params {
            if first {
                write!(f, " (")?;
                first = false;
            } else {
                write!(f, ", ")?;
            }
            write!(f, "{}={}", k, v)?;
        }
        if !first {
            write!(f, ")")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_interno_conserva_el_detalle() {
        let err: CommandError = "Lock poisoned".to_string().into();
        assert_eq!(err.code, ERR_INTERNAL);
        assert_eq!(err.params.get("detail").map(String::as_str), Some("Lock poisoned"));
    }

    #[test]
    fn codigo_con_parametros_se_interpola_en_el_frontend() {
        let err = CommandError::new(ERR_PASSWORD_TOO_SHORT).with_param("min", "8");
        assert_eq!(err.code, ERR_PASSWORD_TOO_SHORT);
        assert_eq!(err.params.get("min").map(String::as_str), Some("8"));
        assert_eq!(err.to_string(), "ERR_PASSWORD_TOO_SHORT (min=8)");
    }

    #[test]
    fn codigo_sin_params_serializa_sin_el_campo() {
        let json = serde_json::to_string(&CommandError::new(ERR_ADMIN_REQUIRED)).unwrap();
        assert_eq!(json, r#"{"code":"ERR_ADMIN_REQUIRED"}"#);
    }

    #[test]
    fn seed_required_es_reconocible_sin_ser_err_internal() {
        let err = CommandError::new(crate::security::data_key::SEED_REQUIRED);
        assert_eq!(err.code, "SEED_REQUIRED");
        assert!(CommandError::is_seed_required("SEED_REQUIRED"));
        assert!(!CommandError::is_seed_required("SEED_REQUIRED: algo"));
    }
}
