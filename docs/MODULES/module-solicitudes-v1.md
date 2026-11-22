# 🧩 Módulo: Solicitud de Estudios Complementarios

### v1 — 2026-11-21 | Modelo v2: checklist + diagnóstico + notas (2026-11-22)

> **Estado**: backend (`src-tauri/src/modules/solicitudes/`) y UI
> (`src/components/SolicitudesPanel.tsx`) operativos.
>
> **Ruta 1**: se compila como módulo interno del crate `fichio-care` con la
> feature `module-solicitudes` (default on). `cargo build --no-default-features`
> produce el binario Free sin el módulo (paywall build-time). Cuando `core`
> se extraiga a un crate propio, el módulo se mueve a `modules/solicitudes`
> sin cambiar el trait `Module`.

---

## 1. Alcance

Un **template** es un catálogo de estudios: un título + una lista ordenada
de estudios que el profesional pide seguido.

| Template | Estudios (ítems) |
|---|---|
| Laboratorio general | Hemograma completo, Glucemia, Colesterol total, Hepatograma, Orina completa… |
| Radiografía tórax | Radiografía de tórax frente y perfil |
| Micológico de uñas | Examen directo, Cultivo micológico de uñas de pies |
| Urocultivo | Recuento de colonias, Identificación de germen, Antibiograma |

Al emitir una **solicitud**, el profesional:

1. elige un template (o completa a mano),
2. **tilda los estudios** que pide (checklist),
3. completa opcionalmente **diagnóstico/indicación** y **notas**.

| En alcance (v1) | Fuera de alcance |
|---|---|
| Templates por profesional (título + checklist de estudios) | Reimpresión: no existe; si hace falta se da de alta una solicitud nueva |
| Checklist, diagnóstico/indicación y notas por solicitud | Resultados, informes o archivos clínicos en la DB del módulo |
| Filiación y profesional desde `ModuleContext` | Copiar esos datos a la DB del módulo |
| Render Markdown + HTML imprimible (guardar PDF vía diálogo del sistema) | Firma Ed25519 y auditoría de emisión (decisión pendiente) |
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
-- Catálogo/config del profesional: sin datos de paciente.
CREATE TABLE templates (
    id            TEXT PRIMARY KEY,
    owner_user_id TEXT NOT NULL,   -- actor de la sesión; sin FK (otra DB)
    name          TEXT NOT NULL,   -- "Laboratorio general"
    items_json    TEXT NOT NULL,   -- ["Hemograma completo", "Glucemia", ...] en orden
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);

-- Orden emitida: snapshot de los ítems tildados + diagnóstico + notas.
CREATE TABLE solicitudes (
    id                 TEXT PRIMARY KEY,
    entity_id          INTEGER NOT NULL,  -- referencia blanda al Core
    created_by_user_id TEXT NOT NULL,     -- actor de la sesión
    study_name         TEXT NOT NULL,     -- snapshot del título del template
    template_id        TEXT,              -- procedencia opcional, sin FK
    items_enc          BLOB NOT NULL,     -- nonce‖ciphertext (DataKey), JSON ["..."]
    diagnostico_enc    BLOB,              -- texto cifrado (PHI), opcional
    notas_enc          BLOB,              -- texto cifrado (PHI), opcional
    created_at         TEXT NOT NULL,
    status             TEXT NOT NULL DEFAULT 'issued'
        CHECK (status IN ('draft', 'issued'))
);
```

**Regla de oro**: la solicitud es **autocontenida**. Guarda los ítems
tildados + diagnóstico + notas (cifrados con la DataKey). El template es solo
un atajo para completarla. Editar o borrar un template después no afecta a
las solicitudes existentes. Tampoco hay reimpresión: si se necesita el
documento, se da de alta una solicitud nueva. `template_id` es solo metadata
de procedencia, sin FK.

**Schema del módulo**: `PRAGMA user_version` propio. La v2 (checklist)
reemplaza a la v1 pre-release (campos tipados): `migrate()` resetea
`templates`/`solicitudes` al detectar `user_version < 2`. En dev también se
puede borrar `fichio-solicitudes.db` con la app cerrada.

No se guarda filiación ni datos del profesional: se piden al Core al
renderizar.

## 4. Acciones (`module_execute`)

| `action` | Payload | Efecto |
|---|---|---|
| `template.create` | `{ name, items: string[] }` | Alta (dueño = actor); normaliza/deduplica ítems |
| `template.update` | `{ id, name, items }` | Solo del dueño |
| `template.delete` | `{ id }` | Solo del dueño; no toca solicitudes |
| `template.list` | — | Templates del actor, con `items` |
| `solicitud.create` | `{ template_id?, study_name?, items: string[], diagnostico?, notas? }` | Valida entidad, exige ≥1 ítem; si hay template, cada ítem debe pertenecer; cifra y audita |
| `solicitud.list` | `{ entity_id? }` | Metadatos del actor (sin contenido clínico) |
| `solicitud.get` | `{ id }` | Metadatos + `items` + `diagnostico` + `notas` descifrados |
| `solicitud.render` | `{ id }` | `{ markdown, html }` con encabezado + diagnóstico + checklist + notas |

- `items` se normaliza: trim, se descartan vacíos y se deduplica preservando
  el orden. Si hay `template_id`, la solicitud guarda los ítems en el **orden
  canónico del template** (no en el orden en que se tildaron).
- Sin template, `study_name` + ítems libres (lista manual).
- Diagnóstico y notas conservan los saltos de línea (Markdown y HTML con
  `white-space: pre-wrap`).
- No hay acción de resultado: la SOAP es la fuente de verdad clínica. Marcar
  una solicitud como completada con un `entry_id` queda para post-v1.

## 5. Seguridad y PHI

- `items_enc`, `diagnostico_enc` y `notas_enc` contienen datos clínicos
  (PHI): se cifran con la DataKey (`ctx.wrap_with_datakey` /
  `unwrap_with_datakey`). Los templates no contienen datos de paciente y
  quedan en claro.
- `AuditEvent` es enum cerrado y sin texto libre (§4): `SolicitudIssued` lleva
  `sha256` del material `{ items, diagnostico, notas }` (nunca el contenido) y
  `SolicitudTemplateSaved` solo el id.
- Actor, rol y `entity_id` salen de `require_session()`; el `entity_id` del
  payload es referencia blanda y `ctx.entity_exists` aplica el scoping.
- Render del documento en Rust puro (sin I/O), como `export/render.rs`. El
  guardado/share respeta mobile: `navigator.share` como fallback del diálogo
  de impresión.

## 6. Pendientes (no bloquean v1)

- **PDF nativo** (opcional): hoy se usa el diálogo de impresión del sistema
  («Guardar como PDF»); una librería PDF embebida queda como mejora.
- **Firma/auditoría de emisión** del documento: copiar el patrón de
  `export_emitted` o no.
- **Sync de fase 4**: outbox propia prevista desde el primer `migrate()` (§13).
