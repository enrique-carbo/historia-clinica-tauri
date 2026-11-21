# 🧩 Módulo: Solicitud de Estudios Complementarios

### v1 — 2026-11-21 | Borrador alineado con `docs/module-contract.md` (Ruta 1)

> **Diseño, no implementación.** Requiere el prerequisite `CoreApi` +
> `PluginHost` (`docs/module-contract.md` §14), que hoy no existe en el código.

---

## 1. Alcance

Generar solicitudes de estudios complementarios (radiología, laboratorio,
cultivos, etc.) con templates creados por cada profesional, e imprimir o
descargar el documento. El resultado **no** lo registra el módulo: se anota en
la nota SOAP que el profesional escribe en el Core.

| En alcance (v1) | Fuera de alcance |
|---|---|
| Templates por profesional con campos dinámicos | Reimpresión: no existe; si hace falta se da de alta una solicitud nueva |
| Solicitudes completas a mano o desde un template propio | Reconstruir documentos antiguos a partir del template |
| Solicitudes con valores cargados y estado | Resultados, informes o archivos clínicos en la DB del módulo |
| Filiación y profesional desde `ModuleContext` | Copiar esos datos a la DB del módulo |
| Render del documento para imprimir/descargar | Firma Ed25519 y auditoría de emisión (decisión pendiente) |
| DB propia `fichio-solicitudes.db` | Integración con PACS/laboratorio (`ctx.http_fetch`, post-v1) |

## 2. Qué vive dónde

| Pieza | Hogar | Por qué |
|---|---|---|
| `templates`, `solicitudes` | DB del módulo `fichio-solicitudes.db` | Operativa del módulo con `migrate()` propio; `create_backup` ya respalda todas las `fichio-*.db` (§7, §9) |
| Filiación del paciente | `ModuleContext.fetch_filiation` | Proyección descifrada por el Core; el módulo no lee `entities` ni ve la DataKey |
| Profesional (nombre, matrícula) | `ModuleContext.fetch_professional` | Ídem; el actor se deriva de la sesión, nunca del payload |
| Resultado del estudio | `entries` del Core (`create_entry`) | Firma, inmutabilidad, EHR y export gratis: el módulo no crea entries (§1, §13) |
| UI | `module_execute` + `is_module_available` | Un solo command para módulos (§5) |

## 3. Modelo de datos

```sql
CREATE TABLE templates (
    id            TEXT PRIMARY KEY,
    owner_user_id TEXT NOT NULL,        -- actor de la sesión; sin FK (otra DB)
    name          TEXT NOT NULL,        -- "Laboratorio", "Rx tórax", ...
    fields_json   TEXT NOT NULL,        -- misma shape que config/entry_templates/*.json
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);

CREATE TABLE solicitudes (
    id                 TEXT PRIMARY KEY,
    entity_id          INTEGER NOT NULL,  -- referencia blanda al Core
    created_by_user_id TEXT NOT NULL,     -- actor de la sesión
    study_name         TEXT NOT NULL,     -- copiado del template o escrito a mano
    template_id        TEXT,              -- procedencia opcional, sin FK (ver abajo)
    values_json        TEXT NOT NULL,     -- valores cargados (ver §5)
    created_at         TEXT NOT NULL,
    status             TEXT NOT NULL DEFAULT 'issued'
        CHECK (status IN ('draft', 'issued'))
);
```

**Regla de oro**: la solicitud es **autocontenida**. Guarda el nombre del
estudio y los valores cargados; el template es solo un atajo para completarla.
Editar o borrar un template después no afecta a las solicitudes existentes,
así que no necesitan versión ni snapshot. Tampoco hay reimpresión: si se
necesita el documento, se da de alta una solicitud nueva (copiando los datos
de la anterior si hace falta). `template_id` es solo metadata de procedencia,
sin FK justamente para que borrar un template no rompa el histórico.

No se guarda filiación ni datos del profesional: se piden al Core al
renderizar.

## 4. Acciones (`module_execute`)

| `action` | Efecto |
|---|---|
| `template.create` / `template.update` / `template.delete` | CRUD simple; dueño = actor de la sesión |
| `template.list` | Solo los templates cuyo `owner_user_id` es el actor |
| `solicitud.create` | Con `template_id`: copia `name` y precarga los campos. Sin él: `study_name` + `values` directos. En ambos casos guarda los valores en la solicitud |
| `solicitud.list` | Histórico de estudios solicitados, filtrado por `entity_id`/actor |
| `solicitud.render` | Devuelve el documento para imprimir/descargar desde la solicitud, no desde el template |

No hay acción de resultado: la SOAP es la fuente de verdad clínica. Marcar una
solicitud como completada con un `entry_id` queda para post-v1 y requiere
extender `ModuleContext` con `fetch_entry_metadata` (aditivo, `default →
Err(Unsupported)`, §4).

## 5. Seguridad y PHI

- `values_json` puede llevar indicación o sospecha diagnóstica → PHI: envolver
  con `ctx.wrap_with_datakey` / `unwrap_with_datakey` (§4). Los templates no
  contienen datos de paciente y quedan en claro.
- `AuditEvent` es enum cerrado y sin texto libre (§4). El alta de template y la
  emisión de solicitud son variantes nuevas, con `sha256`, revisadas anti-PHI en PR.
- Actor, rol y `entity_id` salen de `require_session()`; el `entity_id` del
  payload es referencia blanda y el Core hace el scoping (§2.6).
- Render del documento en Rust puro (sin I/O), como `export/render.rs`. El
  guardado/share respeta mobile: sharesheet donde `revealItemInDir` no aplica.

## 6. Pendientes (no bloquean el diseño)

- **Prerequisite**: `CoreApi` angosto (diseño en `docs/core-api.md`) + trait
  `Module` / `PluginHost` (contrato §14), sin tocar los commands del frontend.
- **Formato de salida**: PDF y librería a elegir; hoy no hay dependencia de PDF.
- **Firma/auditoría de emisión**: copiar el patrón de `export_emitted` o no.
- **Sync de fase 4**: outbox propia prevista desde el primer `migrate()` (§13).
