# 🧩 Arquitectura de Plugins (Microkernel)

Este documento describe la arquitectura de extensibilidad del Simplex Health Core. El sistema no es una aplicación monolítica, sino un **Microkernel (Core) altamente seguro y optimizado**, diseñado para ser extendido mediante un sistema de Plugins aislados.

---

## 🎯 Visión General

El sistema separa radicalmente dos responsabilidades:
1. **El Núcleo (Core):** Encargado exclusivamente de la criptografía (Zero-Knowledge), la gestión de la base de datos relacional local (SQLite), el ciclo de vida de las Bóvedas (Vaults) y la autenticación.
2. **Los Plugins:** Módulos independientes que añaden funcionalidades específicas (Terminología médica, exportación a estándares, integración con obras sociales, IA) sin contaminar el núcleo.

### Principios Fundamentales
* **Aislamiento de Errores (Resilience):** Si un plugin de IA falla o se cuelga, el médico debe poder seguir escribiendo notas SOAP y guardándolas localmente sin interrupciones.
* **Carga Bajo Demanda:** Los plugins se activan mediante un archivo de configuración (`plugins.json`). El núcleo no carga código innecesario en RAM.
* **Aislamiento de Datos:** Los plugins que requieren grandes conjuntos de datos (ej. SNOMED CT, LOINC) gestionan sus propios archivos SQLite locales (`data/snomed.db`) sin afectar el rendimiento de la base de datos principal de pacientes.
* **Cumplimiento Zero-Knowledge:** Los plugins **nunca** tienen acceso a las claves maestras. Si un plugin trae datos externos (ej. un nombre de PAMI), el Core los recibe, los cifra y los persiste.

---

## 🏗️ Arquitectura del Sistema

```text
┌─────────────────────────────────────────────────────────────────┐
│                     NÚCLEO DE LA HCE (Core)                     │
│  (Rust + Tauri + SQLite | Zero-Knowledge & Cifrado E2E)         │
└────────────────────────┬────────────────────────────────────────┘
                         │ (IPC Seguro)
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                    PLUGIN MANAGER (Rust)                        │
│  • Carga dinámica • Ciclo de vida • Aislamiento de errores      │
└────────────────────────┬────────────────────────────────────────┘
                         │
     ┌───────────────────┼───────────────────┬───────────────────┐
     ▼                   ▼                   ▼                   ▼
┌─────────────┐   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐
│ Terminología│   │  HL7 / FHIR │   │ Integración │   │    IA / OCR │
│ (LOINC,SNOMED)│   │ (Exportación)│   │ (PAMI, OSDE)│   │ (Sugerencias)│
└─────────────┘   └─────────────┘   └─────────────┘   └─────────────┘
```

---

## 🦀 Diseño Técnico en Rust

El sistema de plugins se basa en el uso de *Traits* de Rust y programación asíncrona (`async-trait`), garantizando que las operaciones de red o de base de datos de un plugin no congelen la UI de Tauri.

### 1. Definición del Trait `Plugin`

Todo plugin debe implementar este trait base, definiendo su ciclo de vida y su método de ejecución principal:

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: String,
    pub nombre: String,
    pub version: String,
    pub categoria: PluginCategory,
    pub activo: bool,
}

#[async_trait]
pub trait Plugin: Send + Sync {
    fn metadata(&self) -> PluginMetadata;

    // Ciclo de vida
    async fn inicializar(&mut self, config: HashMap<String, String>) -> Result<(), String>;
    async fn detener(&mut self) -> Result<(), String>;

    // Comunicación estandarizada (Contrato)
    async fn ejecutar(&self, input: PluginInput) -> Result<PluginOutput, String>;
}
```

### 2. Contrato de Datos (Input / Output)
Para evitar que el Core conozca la lógica interna de cada plugin, la comunicación se realiza mediante un contrato JSON flexible:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInput {
    pub accion: String, // ej: "buscar", "exportar", "validar"
    pub datos: serde_json::Value, // Payload dinámico
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginOutput {
    pub exito: bool,
    pub datos: Option<serde_json::Value>,
    pub errores: Vec<String>,
}
```

---

## ⚙️ Configuración (`plugins.json`)

El `PluginManager` lee un archivo JSON al iniciar la aplicación para determinar qué plugins cargar en memoria y qué configuraciones específicas necesitan (ej. tokens de API).

```json
{
  "plugins": [
    {
      "id": "loinc",
      "activo": true,
      "db_path": "data/loinc.db"
    },
    {
      "id": "pami_integracion",
      "activo": true,
      "config": {
        "api_url": "https://api.pami.gov.ar/v1",
        "api_token": "${PAMI_API_TOKEN}"
      }
    }
  ]
}
```

---

## 🔒 Seguridad y Límites del Plugin

1. **Sin Acceso a Claves:** Los plugins operan en un espacio aislado. Si el plugin de PAMI obtiene el domicilio de un afiliado, lo devuelve al Core de Rust. El Core lo cifra con AES-GCM-256 y lo guarda en SQLite. El plugin nunca ve la llave maestra.
2. **Firmas Digitales (Futuro):** En la Fase 3, un plugin de "Firma Digital" escuchará los eventos de guardado de notas SOAP. Tomará el hash de la nota, lo firmará con la clave privada del médico (que sí está en el Core) y devolverá la firma para ser persistida.
3. **Sandboxing de Red:** Los plugins de integración externa solo pueden realizar peticiones HTTP mediante el cliente del Core, permitiendo auditar y limitar las URLs a las que intentan conectarse.

---

## 🗺️ Categorías de Plugins Planeadas

| Categoría | Ejemplos | Caso de Uso |
|-----------|----------|-------------|
| **Terminología** | LOINC, SNOMED CT, ICD-10 | Estandarizar notas SOAP y diagnósticos para interoperabilidad. |
| **Integración** | PAMI, OSDE, SISA | Consultar cobertura, verificar afiliados, validar recetas online. |
| **Exportación** | FHIR, HL7 v2, PDF | Compartir historias clínicas con otros sistemas o imprimir. |
| **IA Médica** | Sugeridor LOINC, OCR | Asistir al médico escribiendo, digitalizando papel o alertando riesgos. |


---
