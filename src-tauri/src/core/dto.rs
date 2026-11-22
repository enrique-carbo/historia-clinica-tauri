use serde::{Deserialize, Serialize};

/// Una matrícula profesional: número + jurisdicción (provincia/colegio).
/// El orden del array es el orden de presentación — la primera es la
/// matrícula principal.
///
/// Vive en `core::dto` porque es parte de los DTOs que consumen los módulos;
/// `commands::professional_profile_commands` la re-exporta para no cambiar
/// los call sites existentes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct License {
    pub number: String,
    pub jurisdiction: String,
}

/// Rol del actor en el contrato de módulos. Cerrado: un rol de sesión
/// desconocido no se mapea y el gate lo rechaza (fail-closed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActorRole {
    Administrador,
    Asistente,
    Profesional,
    Paciente,
}

impl ActorRole {
    pub fn from_role(role: &str) -> Option<Self> {
        match role {
            "administrador" => Some(Self::Administrador),
            "asistente" => Some(Self::Asistente),
            "profesional" => Some(Self::Profesional),
            "paciente" => Some(Self::Paciente),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Administrador => "administrador",
            Self::Asistente => "asistente",
            Self::Profesional => "profesional",
            Self::Paciente => "paciente",
        }
    }
}

/// Actor derivado de la sesión. El módulo nunca recibe un `user_id` del
/// payload: `ModuleContext::actor()` sale de `require_session()` (§2.6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    pub user_id: String,
    pub role: ActorRole,
}

/// Un campo de filiación ya descifrado y etiquetado, en el orden canónico de
/// `SchemaConfig::field_labels`. Agnóstico de Med/Vet: incluye lo que cada
/// perfil defina (obra social/afiliado en Med, especie/raza/tutor en Vet).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FiliationFieldDto {
    pub key: String,
    pub label: String,
    pub value: String,
}

/// Proyección acotada de la ficha del paciente: nunca `enc_data_blob` ni
/// filas completas. `full_name` y `document` son atajos; `fields` es la
/// lista canónica ya etiquetada.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FiliationDto {
    pub entity_id: i64,
    pub full_name: String,
    pub document: String,
    pub fields: Vec<FiliationFieldDto>,
}

/// Proyección del profesional que firma una orden/documento. Alineada con
/// `ProfessionalProfile` y `ExportProfessional`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfessionalDto {
    pub user_id: String,
    pub full_name: String,
    pub profession: String,
    pub specialty: String,
    /// Misma estructura que `ProfessionalProfile`; la primera es la principal.
    pub licenses: Vec<License>,
    pub public_key_hex: String,
}

/// Eventos de auditoría de módulos: enum **cerrado** y sin texto libre.
/// Cada variante solo lleva metadatos no-PHI (ids, fingerprints, montos). Es
/// la garantía estructural contra PHI en `audit_log` (`module-contract.md` §4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditEvent {
    SolicitudIssued {
        subject_id: i64,
        solicitud_id: String,
        /// HMAC-SHA256 con clave derivada de la DataKey (`hmac-sha256:...`).
        fingerprint: String,
    },
    SolicitudCompleted {
        subject_id: i64,
        solicitud_id: String,
    },
    SolicitudTemplateSaved {
        template_id: String,
    },
    TeleconsultaMessageSent {
        entity_id: i64,
        conversation_id: String,
    },
    TeleconsultaPaymentConfirmed {
        entity_id: i64,
        conversation_id: String,
        amount_cents: i64,
    },
    ModuleActivated {
        module_id: String,
    },
    ModuleDeactivated {
        module_id: String,
    },
}

impl AuditEvent {
    /// Acción estable que se escribe en `audit_log.action`.
    pub fn action(&self) -> &'static str {
        match self {
            Self::SolicitudIssued { .. } => "solicitud_issued",
            Self::SolicitudCompleted { .. } => "solicitud_completed",
            Self::SolicitudTemplateSaved { .. } => "solicitud_template_saved",
            Self::TeleconsultaMessageSent { .. } => "teleconsulta_message_sent",
            Self::TeleconsultaPaymentConfirmed { .. } => "teleconsulta_payment_confirmed",
            Self::ModuleActivated { .. } => "module_activated",
            Self::ModuleDeactivated { .. } => "module_deactivated",
        }
    }

    /// Detalle para `audit_log.detail`: **solo** metadatos. Nunca nombres,
    /// documentos ni contenido clínico.
    pub fn to_detail(&self) -> String {
        match self {
            Self::SolicitudIssued {
                subject_id,
                solicitud_id,
                fingerprint,
            } => format!(
                "subject_id={} solicitud_id={} fingerprint={}",
                subject_id, solicitud_id, fingerprint
            ),
            Self::SolicitudCompleted {
                subject_id,
                solicitud_id,
            } => format!("subject_id={} solicitud_id={}", subject_id, solicitud_id),
            Self::SolicitudTemplateSaved { template_id } => {
                format!("template_id={}", template_id)
            }
            Self::TeleconsultaMessageSent {
                entity_id,
                conversation_id,
            } => format!("entity_id={} conversation_id={}", entity_id, conversation_id),
            Self::TeleconsultaPaymentConfirmed {
                entity_id,
                conversation_id,
                amount_cents,
            } => format!(
                "entity_id={} conversation_id={} amount_cents={}",
                entity_id, conversation_id, amount_cents
            ),
            Self::ModuleActivated { module_id } => format!("module_id={}", module_id),
            Self::ModuleDeactivated { module_id } => format!("module_id={}", module_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_role_maps_from_session_strings_fail_closed() {
        for role in [
            ActorRole::Administrador,
            ActorRole::Asistente,
            ActorRole::Profesional,
            ActorRole::Paciente,
        ] {
            assert_eq!(ActorRole::from_role(role.as_str()), Some(role));
        }
        assert_eq!(ActorRole::from_role("visitante"), None);
        assert_eq!(ActorRole::from_role(""), None);
    }

    #[test]
    fn audit_events_map_to_stable_actions_and_safe_details() {
        let cases = [
            (
                AuditEvent::SolicitudIssued {
                    subject_id: 7,
                    solicitud_id: "sol-1".to_string(),
                    fingerprint: "hmac-sha256:abc123".to_string(),
                },
                "solicitud_issued",
                "subject_id=7 solicitud_id=sol-1 fingerprint=hmac-sha256:abc123",
            ),
            (
                AuditEvent::SolicitudCompleted {
                    subject_id: 7,
                    solicitud_id: "sol-1".to_string(),
                },
                "solicitud_completed",
                "subject_id=7 solicitud_id=sol-1",
            ),
            (
                AuditEvent::SolicitudTemplateSaved {
                    template_id: "tpl-1".to_string(),
                },
                "solicitud_template_saved",
                "template_id=tpl-1",
            ),
            (
                AuditEvent::TeleconsultaMessageSent {
                    entity_id: 3,
                    conversation_id: "conv-9".to_string(),
                },
                "teleconsulta_message_sent",
                "entity_id=3 conversation_id=conv-9",
            ),
            (
                AuditEvent::TeleconsultaPaymentConfirmed {
                    entity_id: 3,
                    conversation_id: "conv-9".to_string(),
                    amount_cents: 250000,
                },
                "teleconsulta_payment_confirmed",
                "entity_id=3 conversation_id=conv-9 amount_cents=250000",
            ),
            (
                AuditEvent::ModuleActivated {
                    module_id: "solicitudes".to_string(),
                },
                "module_activated",
                "module_id=solicitudes",
            ),
            (
                AuditEvent::ModuleDeactivated {
                    module_id: "solicitudes".to_string(),
                },
                "module_deactivated",
                "module_id=solicitudes",
            ),
        ];

        for (event, action, detail) in cases {
            assert_eq!(event.action(), action);
            assert_eq!(event.to_detail(), detail);
            // Defensa anti-PHI: el detalle no puede contener nombres ni
            // extensiones de archivo exportado.
            assert!(!detail.contains("Pérez"));
            assert!(!detail.contains(".md"));
        }
    }

    #[test]
    fn audit_event_roundtrips_through_json() {
        let event = AuditEvent::SolicitudIssued {
            subject_id: 1,
            solicitud_id: "s".to_string(),
            fingerprint: "hmac-sha256:h".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: AuditEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, event);
    }
}
