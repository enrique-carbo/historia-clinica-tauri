# 🧩 Módulo: Solicitud de Estudios Complementarios

### v1 — 2026-11-21 | Backend implementado 2026-11-22

> **Estado**: backend operativo (`src-tauri/src/modules/solicitudes/`):
> migración propia, templates por profesional, alta/listado/consulta de
> solicitudes y render Markdown. Pendiente: PDF + UI (siguiente corte).
>
> **Ruta 1**: se compila como módulo interno del crate `fichio-care` con la
> feature `module-solicitudes` (default on). `cargo build --no-default-features`
> produce el binario Free sin el módulo (paywall build-time). Cuando `core`
> se extraiga a un crate propio, el módulo se mueve a `modules/solicitudes`
> sin cambiar el trait `Module`.

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
| Solicitudes con valores cifrados y estado | Resultados, informes o archivos clínicos en la DB del módulo |
| Filiación y profesional desde `ModuleContext` | Copiar esos datos a la DB del módulo |
| Render del documento (Markdown hoy; PDF en el siguiente corte) | Firma Ed25519 y auditoría de emisión (decisión pendiente) |
| DB propia `fichio-solicitudes.db` | Integración con PACS/laboratorio (`ctx.http_fetch`, post-v1) |

## 2. Qué vive dónde

| Pieza | Hogar | Por qué |
|---|---|---|
| `templates`, `solicitudes` | DB del módulo `fichio-solicitudes.db` | Operativa del módulo con `migrate()` propio; `create_backup` ya respalda todas las `fichio-*.db` (§7, §9) |
| Filiación del paciente | `ModuleContext.fetch_filiation` | Proyección descifrada por el Core; el módulo no lee `entities` ni ve la DataKey |
| Profesional (nombre, matrícula) | `ModuleContext.fetch_professional` | Ídem; el actor se deriva de la sesión, nunca del payload |
| Resultado del estudio | `entries` del Core (`create_entry`) | Firma, inmutabilidad, EHR y export gratis: el módulo no crea entries (§1, §13) |
| Registro de activación/acceso | `modules`/`module_access` del Core (v002) | Gobernanza del host; no es dato del módulo |
| UI | `module_execute` + `is_module_available` | Un solo command para módulos (§5) |

## 3. Modelo de datos

```sql
CREATE TABLE templates (
    id            TEXT PRIMARY KEY,
    owner_user_id TEXT NOT NULL,        -- actor de la sesión; sin FK (otra DB)
    name          TEXT NOT NULL,        -- "Laboratorio", "Rx tórax", ...
    fields_json   TEXT NOT NULL,        -- shape igual a config/entry_templates/*.json
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);

CREATE TABLE solicitudes (
    id                 TEXT PRIMARY KEY,
    entity_id          INTEGER NOT NULL,  -- referencia blanda al Core
    created_by_user_id TEXT NOT NULL,     -- actor de la sesión
    study_name         TEXT NOT NULL,     -- copiado del template o escrito a mano
    template_id        TEXT,              -- procedencia opcional, sin FK
    values_enc         BLOB NOT NULL,     -- nonce‖ciphertext (DataKey), JSON de valores
    created_at         TEXT NOT NULL,
    status             TEXT NOT NULL DEFAULT 'issued'
        CHECK (status IN ('draft', 'issued'))
);
```

**Regla de oro**: la solicitud es **autocontenida**. Guarda el nombre del
estudio y los valores (cifrados con la DataKey vía `ctx.wrap_with_datakey`).
El template es solo un atajo para completarla. Editar o borrar un template
después no afecta a las solicitudes existentes, así que no necesitan versión
ni snapshot. Tampoco hay reimpresión: si se necesita el documento, se da de
alta una solicitud nueva (copiando los datos de la anterior si hace falta).
`template_id` es solo metadata de procedencia, sin FK justamente para que
borrar un template no rompa el histórico.

No se guarda filiación ni datos del profesional: se piden al Core al
renderizar.

## 4. Acciones (`module_execute`)

| `action` | Efecto | Estado |
|---|---|---|
| `template.create` | Alta (dueño = actor de la sesión); valida `name` + `fields` | ✅ |
| `template.update` | Solo del dueño; valida la nueva definición | ✅ |
| `template.delete` | Solo del dueño; no toca solicitudes | ✅ |
| `template.list` | Solo los templates del actor | ✅ |
| `solicitud.create` | Valida `entity_id` (vía Core), template y required; cifra valores; audita `SolicitudIssued` | ✅ |
| `solicitud.list` | Metadatos del actor, filtrable por `entity_id` (sin valores) | ✅ |
| `solicitud.get` | Metadatos + valores descifrados | ✅ |
| `solicitud.render` | Documento Markdown con datos del Core + template | ✅ (PDF en el siguiente corte) |

La forma de `fields` es la misma de `config/entry_templates/*.json`:
`{"name","label","type" (text/textarea/select/date),"required","options"}`.

No hay acción de resultado: la SOAP es la fuente de verdad clínica. Marcar una
solicitud como completada con un `entry_id` queda para post-v1 y requiere
extender `ModuleContext` con `fetch_entry_metadata` (aditivo, `default →
Err(Unsupported)`, §4).

## 5. Seguridad y PHI

- `values_enc` puede llevar indicación o sospecha diagnóstica → PHI cifrada con
  la DataKey (`ctx.wrap_with_datakey` / `unwrap_with_datakey`). Los templates
  no contienen datos de paciente y quedan en claro.
- `AuditEvent` es enum cerrado y sin texto libre (§4): `SolicitudIssued` lleva
  `sha256` del JSON de valores (nunca el contenido) y `SolicitudTemplateSaved`
  solo el id.
- Actor, rol y `entity_id` salen de `require_session()`; el `entity_id` del
  payload es referencia blanda y `ctx.entity_exists` aplica el scoping.
- Render del documento en Rust puro (sin I/O), como `export/render.rs`. El
  guardado/share respetará mobile: sharesheet donde `revealItemInDir` no aplica.

## 6. Pendientes (no bloquean el backend)

- **PDF + UI** (siguiente corte): formateador PDF, pantalla de templates, alta de
  solicitud y flujo imprimir/descargar con fallback mobile.
- **Firma/auditoría de emisión** del documento: copiar el patrón de
  `export_emitted` o no.
- **Sync de fase 4**: outbox propia prevista desde el primer `migrate()` (§13).
