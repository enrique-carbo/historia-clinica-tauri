# ARQUITECTURA DEL CORE ENTITY ENTRY
Aquí tenés el archivo .md completamente unificado, estructurado y actualizado, integrando tus especificaciones JSON, el diseño relacional simplificado [2] y las reglas de negocio estrictas que acordamos.
Este documento funciona como la especificación técnica final para guiar el proceso de refactorización de tu base de datos y validadores en Rust.
------------------------------

# Clinical Core: Sistema de Historia Clínica Electrónica Local-First## 📌 Visión del Proyecto**Clinical Core** es una arquitectura de software de alta eficiencia, robusta y soberana diseñada para sistemas de gestión de historias clínicas electrónicas (**HCE**) multi-dominio (medicina humana, veterinaria, odontología, etc.).

Bajo la filosofía **Local-First**, el sistema opera de forma 100% independiente en la máquina local del profesional de la salud. Esto garantiza disponibilidad offline absoluta, privacidad de datos de grado militar y latencia cero, delegando la sincronización multi-dispositivo a procesos asíncronos y cifrados en segundo plano.
---## ⚖️ Las Reglas Estrictas del Core (Inquebrantables)Para garantizar que el sistema sea eterno, inmune a corrupciones y legalmente invulnerable, el desarrollo debe respetar estrictamente las siguientes reglas arquitectónicas:

### 1. Inmutabilidad Absoluta de las Entradas (`Append-Only`)
* **Regla:** Ninguna fila de la tabla `entries` de la base de datos del Core puede ser editada (`UPDATE`) ni eliminada (`DELETE`) bajo ninguna circunstancia operativa clínica.
* **Manejo de Errores:** Si un profesional comete un error en una nota SOAP, una medicación o una alergia, el error **not se borra**. Se debe generar una **nueva `Entry`** de enmienda o corrección que se enlaza al ID del registro erróneo mediante su payload.* **Impacto en Sincronización:** Al no existir ediciones, los conflictos de sincronización offline desaparecen por completo. La base de datos es una línea de tiempo acumulativa de verdades históricas.
### 2. Aislamiento Total de Plugins (Frontera Rígida de Datos)* **Regla:** Las tablas `entities` y `entries` pertenecen **exclusivamente** al Core y solo almacenan actores del sistema y hechos médicos duros/validados.
* **Manejo de Datos de Plugins:** Cualquier funcionalidad extendida (Agenda, Telemedicina, Facturación, Gestión de Stock) tiene prohibido crear tablas dentro de `core_clinical.db`. Cada plugin debe inicializar y gestionar **su propio archivo SQLite independiente** (ej: `scheduler.db`, `telemed.db`).
* **Flujo de Interacción:** Un plugin de telemedicina gestiona el chat informal (textos, audios, adjuntos) pura y exclusivamente dentro de su propia base de datos (`telemed.db`). Al finalizar la teleconsulta, el plugin debe obligar al profesional a redactar un resumen clínico estructurado (ej: SOAP) que se enviará al Core como una `Entry` inmutable. El Core jamás procesa ni almacena tráfico de mensajería informal.
### 3. Soberanía Criptográfica Absoluta (Zero-Knowledge)* **Regla:** El desarrollador/servidor central no posee, no conoce y no puede recuperar jamás la llave de acceso a los datos clínicos de ninguna instalación.* **Frase Semilla Local:** Cada instalación genera una única clave global a partir de una frase semilla de **3 o 6 palabras** (estándar mnemónico). Esta frase es la llave maestra para descifrar el archivo local mediante **SQLCipher (AES-256)**.* **Responsabilidad del Usuario:** Se entrena explícitamente al cliente para que grabe estas palabras en una **placa de metal** física (resistente a incendios/inundaciones). Si la computadora se rompe y el usuario pierde el metal, los datos son matemáticamente irrecuperables. No existe el soporte de "recuperar contraseña".

### 4. Firma Criptográfica Obligatoria por Entidad (`Non-Repudiation`)
* **Regla:** Cada `Entity` de tipo usuario (médico, enfermero, asistente o incluso el paciente/tutor en un consentimiento) posee su propio par de claves criptográficas asimétricas.
* **El Acto de Firmar:** Cada nueva `Entry` enviada al Core debe incluir obligatoriamente una firma digital (`signature`) generada en el backend de Rust (usando algoritmos como **Ed25519**) combinando la clave privada del autor con el hash del payload. Esto garantiza ante auditorías legales que el registro no fue alterado y determina de forma inapelable quién lo creó.
---## 🗺️ Estrategia de Refactorización de Datos
El sistema evoluciona de un modelo tradicional multi-tabla sobre-acoplado (`users`, `professional_profiles`, `medical_history`, `notes`) hacia un modelo unificado relacional-dinámico de dos tablas. La estructura variable del negocio se inyecta por esquemas JSON.
```text
ANTES (Modelo Acoplado V1)            AHORA (Core Unificado Append-Only)
┌───────────────────────┐             ┌──────────────────────────────────┐
│ users                 │             │ entities                         │
├───────────────────────┤             ├──────────────────────────────────┤
│ professional_profiles │ ───═══───>  │ id (UUIDv4)                      │
├───────────────────────┤             │ type (PATIENT, USER, CLINIC)     │
│ entities (v1)         │             │ name                             │
└───────────────────────┘             │ attributes (JSON cifrado)        │
                                      │ blind_index                      │
┌───────────────────────┐             └──────────────────────────────────┘
│ notes (SOAP)          │             ┌──────────────────────────────────┐
├───────────────────────┤             │ entries                          │
│ medical_history       │ ───═══───>  ├──────────────────────────────────┤
└───────────────────────┘             │ id (UUIDv4)                      │
                                      │ category (SOAP_NOTE, ALLERGY...) │
                                      │ payload (JSON inmutable)         │
                                      │ signature (Ed25519)              │
                                      └──────────────────────────────────┘
```
---
## 🗄️ Esquema de la Base de Datos Central Refactorizada (`core_clinical.db`)

El Core se compone única y exclusivamente de estas dos tablas relacionales y polimórficas. Reemplaza por completo el diseño de bases de datos anterior.
```sql
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL; -- Activado para alta concurrencia local

-- Tabla unificada de Actores (Pacientes, Médicos, Clínicas)
CREATE TABLE IF NOT EXISTS entities (
    id TEXT PRIMARY KEY NOT NULL,          -- UUIDv4 generado en el cliente
    type TEXT NOT NULL CHECK(type IN ('PATIENT', 'USER', 'CLINIC')),
    name TEXT NOT NULL,                    -- Nombre legible para búsquedas rápidas en UI
    attributes TEXT NOT NULL DEFAULT '{}',  -- JSON cifrado con campos del esquema dinámico
    blind_index TEXT UNIQUE,               -- Hash para búsquedas indexadas (ej: DNI, Microchip)
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (json_valid(attributes))
);

-- Tabla unificada de Hechos Clínicos Inmutables (SOAP, Alergias, Antecedentes)
CREATE TABLE IF NOT EXISTS entries (
    id TEXT PRIMARY KEY NOT NULL,          -- UUIDv4
    category TEXT NOT NULL CHECK(category IN ('SOAP_NOTE', 'MEDICATION', 'ALLERGY', 'CONDITION')),
    subject_id TEXT NOT NULL,              -- FK al Paciente (Entity)
    author_id TEXT NOT NULL,               -- FK al Profesional/Asistente (Entity)
    title TEXT NOT NULL,                   -- Resumen legible (ej: "Alergia a Penicilina")
    status TEXT NOT NULL CHECK(status IN ('ACTIVE', 'RESOLVED', 'COMPLETED')),
    timestamp TEXT NOT NULL,               -- Fecha y hora del hecho médico
    payload TEXT NOT NULL DEFAULT '{}',    -- JSON estructurado inmutable según NoteTemplate
    signature TEXT NOT NULL,               -- Firma criptográfica Ed25519 del autor
    updated_at TEXT NOT NULL,              -- Control de tiempo de inserción local
    is_synced INTEGER NOT NULL DEFAULT 0 CHECK(is_synced IN (0, 1)), -- Cola offline
    
    FOREIGN KEY (subject_id) REFERENCES entities(id) ON DELETE RESTRICT,
    FOREIGN KEY (author_id) REFERENCES entities(id) ON DELETE RESTRICT,
    CHECK (json_valid(payload))
);

-- Índices de Rendimiento Clínico Local
CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(type);
CREATE INDEX IF NOT EXISTS idx_entries_timeline ON entries(subject_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_entries_sync_queue ON entries(is_synced) WHERE is_synced = 0;
```
---## 💻 Mapeo y Modelado en el Backend (Rust)
Los tipos de datos de Rust reflejan la nueva estructura del Core. El payload variable de las entradas y los atributos variables de las entidades son validados dinámicamente mediante la configuración JSON.
```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use serde_json::Value;

// --- MODELADO DE ENTIDADES ---
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Entity {
    pub id: String,
    pub r#type: String,                    // "PATIENT", "USER", "CLINIC"
    pub name: String,
    pub attributes: HashMap<String, Value>, // Campos dinámicos del JSON validados
    pub blind_index: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// --- MODELADO DE ENTRADAS CLÍNICAS (INMUTABLES) ---
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Entry {
    pub id: String,
    pub category: String,                  // "SOAP_NOTE", "MEDICATION", "ALLERGY", "CONDITION"
    pub subject_id: String,                // ID del paciente
    pub author_id: String,                 // ID del usuario firmante
    pub title: String,
    pub status: String,                    // "ACTIVE", "RESOLVED", "COMPLETED"
    pub timestamp: String,
    pub payload: HashMap<String, Value>,    // Contenido dinámico del Template (ej: SOAP)
    pub signature: String,                 // Firma Ed25519
    pub updated_at: String,
}
```
### Flujo de Validación de Esquemas RefactorizadoSe utiliza la configuración JSON de plantillas y campos para validar los datos dinámicos antes de impactar en SQLite.

```rust
impl SchemaConfig {

/// Valida los atributos dinámicos de una Entity contra el JSON de configuración
pub fn validate_dynamic_entity(
&self,
entity_type: &str,
data: &HashMap<String, Value>,
) -> Result<(), String> {
let schema = self
.entities
.get(entity_type)
.ok_or_else(|| format!("Tipo de entidad desconocido: {}", entity_type))?;
for field in &schema.fields {
if field.required {
let is_empty = match data.get(&field.name) {
Some(Value::String(s)) => s.trim().is_empty(),
Some(Value::Null) | None => true,
_ => false,
};
if is_empty {
return Err(format!("Campo requerido faltante: {}", field.label));
}
}
}
Ok(())
}
}
```
------------------------------
## 🔌 Ecosistema de Extensiones (Plugins de Tauri)
El Core expone la instancia de la base de datos principal y hooks de ciclo de vida. Los plugins de Tauri extienden el sistema creando bases de datos aisladas e interceptando IDs del Core.
## Catálogo Inicial de Plugins Planificados:

   1. tauri-plugin-scheduler (Agenda & Turnos):
   * Persistencia: scheduler.db independiente.
      * Funcionamiento: Administra calendarios y turnos utilizando el id de las entidades paciente del Core.
   2. tauri-plugin-telemed (Mensajería asíncrona):
   * Persistencia: telemed.db independiente.
      * Funcionamiento: Almacena logs de chat informales de forma aislada. Al finalizar la consulta, interactúa con la UI para forzar al profesional a guardar una Entry tipo SOAP_NOTE en el Core de la HCE.
   3. tauri-plugin-clinical-exporter (Generador de Informes):
   * Persistencia: Sin persistencia (Stateless).
      * Funcionamiento: Lee cronológicamente las Entries del Core y renderiza de forma nativa archivos en formato Markdown (.md) o PDFs firmados.
   
## Comunicación Cross-Database mediante SQLite (ATTACH DATABASE):
Cuando un plugin requiera realizar consultas de rendimiento cruzado (ej: estadísticas de turnos uniendo datos dinámicos del paciente), utilizará la vinculación en caliente de SQLite desde Rust:
rust // Ejecutado dentro del contexto de inicialización de un Plugin específico rusqlite::query("ATTACH DATABASE 'core_clinical.db' AS core_db") .execute(&plugin_connection) .ok(); 

***
