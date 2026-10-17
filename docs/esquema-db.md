# 🗄️ Esquema de Base de Datos y Templates

> Extraído del README (2026-10-17). Fuente operativa del catálogo de
> migraciones: `src-tauri/src/db/migrations/mod.rs`.

## Estructura de la Base de Datos

**Versionado**: el esquema se migra con `db/migrations/` usando `PRAGMA user_version`. Cada migración corre en su propia transacción (append-only), y una base creada por una versión *más nueva* de la app se rechaza en el arranque.

### Tablas Principales
| Tabla | Propósito | Relación |
|---|---|---|
| `users` | Autenticación y roles: `administrador` (gestión técnica), `asistente` (recepción), `profesional` (cualquier terapeuta que crea entries: médico, enfermero, psicólogo, kinesiólogo, odontólogo…), `paciente` — con `entity_id` (vínculo a la ficha cuando es paciente), `is_active` (baja lógica) y `last_login_at` | PK `id TEXT` |
| `professional_profiles` | Perfil profesional + llave pública Ed25519 vigente | FK → `users(id)` |
| `signing_keys` | **Histórico de llaves de firma** (`valid_from`/`valid_to`, `valid_to NULL` = vigente) — preserva la verificación de entries tras resets/rotaciones | FK → `users(id)` |
| `audit_log` | Registro de acciones sensibles (altas, bajas, resets, rotación de seed) | FK → `users(id)` ×2 (actor, target) |
| `entities` | Entidades genéricas (filiación de pacientes) | PK `id INTEGER` |
| `entries` | Entries clínicas inmutables (append-only) — único sistema clínico | FK → `entities(id)`, FK → `users(id)` |
| `entity_keys` | Claves de datos por entidad-usuario (telemedicina) | FK → `entities(id)` |
| `sync_queue` | Cola FIFO de sincronización | Reemplaza `is_synced` |

> **Nota**: Las tablas legacy `medical_history` y `notes` fueron eliminadas del esquema. Todo el dato clínico vive en `entries` (categorías `SOAP_NOTE`, `CONDITION`, `ALLERGY`, `MEDICATION`). En dev, borrá `historia_clinica.db` para limpiar tablas obsoletas.

### Tabla `entries` (Entity-Entry Paradigm — única tabla clínica)
```sql
CREATE TABLE entries (
    id TEXT PRIMARY KEY NOT NULL,
    category TEXT NOT NULL CHECK(category IN ('SOAP_NOTE', 'MEDICATION', 'ALLERGY', 'CONDITION')),
    subject_id INTEGER NOT NULL,            -- FK → entities(id)
    author_id TEXT NOT NULL,                -- FK → users(id)
    title TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('ACTIVE', 'RESOLVED', 'COMPLETED')),
    timestamp TEXT NOT NULL,
    payload BLOB NOT NULL,                  -- Cifrado con DataKey (AES-GCM-256)
    signature TEXT NOT NULL,                -- Firma Ed25519
    created_at TEXT NOT NULL,
    is_synced INTEGER NOT NULL DEFAULT 0 CHECK(is_synced IN (0, 1)),
    FOREIGN KEY(subject_id) REFERENCES entities(id) ON DELETE RESTRICT,
    FOREIGN KEY(author_id) REFERENCES users(id) ON DELETE RESTRICT
);

CREATE INDEX idx_entries_subject ON entries(subject_id, timestamp DESC);
CREATE INDEX idx_entries_category ON entries(category);
CREATE INDEX idx_entries_sync ON entries(is_synced) WHERE is_synced = 0;
```

**Nota**: Las entries son **inmutables**. No existen `update_entry` ni `delete_entry`.

## Arquitectura de Template (v1)

*"El core no sabe si el paciente es humano o animal. Sabe que hay entidades, atributos, relaciones y cifrado. El resto es configuración."*

### Archivos de Configuración Empaquetados
| Archivo | Propósito | Consumidor |
|---|---|---|
| `profiles/{med,vet}/entity_schema.json` (fuente única por perfil) | Define entidades, campos de filiación y groups (ej. Responsable - Tutor) | `entity_commands.rs` (validación + blind index), `export_commands.rs` (labels del .md), `src/utils/schema.ts` → `Entity.tsx` + `PatientEhrView.tsx` |
| `config/entry_templates/soap.json` | Template de entry SOAP | `EntryForm.tsx` |
| `config/entry_templates/allergy.json` | Template de alergia | `EntryForm.tsx` |
| `config/entry_templates/medication.json` | Template de medicación | `EntryForm.tsx` |
| `config/entry_templates/condition.json` | Template de condición médica | `EntryForm.tsx` |

`entity_schema.json` se empaqueta en el binario con `include_str!` desde OUT_DIR (`config_schema.rs::from_embedded()`; `build.rs` copia `profiles/{FICHIO_PROFILE}/entity_schema.json`, default `med`) y el frontend consume los mismos perfiles vía `src/utils/schema.ts` (selección por `VITE_FICHIO_PROFILE`) — es la fuente única: labels y orden del export Markdown y del grid de contacto del EHR se derivan del schema, así que agregar un campo al JSON lo propaga a form, ficha EHR y export sin tocar código. Los groups (`type: "group"`) se renderizan como sección en el form pero se guardan aplanados con prefijo `{group}_{campo}` (ej. `responsable_nombre_completo`), con validación "nada o todo". Los `entry_templates/` se embeben en el bundle de Vite con `import.meta.glob(..., { eager: true })`: `EntryForm.tsx` los descubre del disco en build time, valida el shape (`category`, `label`, `fields`) y arma el diccionario de templates. Todo funciona offline y es inmutable en runtime.

**Paradigma Entity-Entry**: El core no conoce el significado clínico de cada campo. Simplemente almacena `entries` cifradas con su categoría y las presenta según el template JSON correspondiente. Agregar nuevos tipos de entries solo requiere crear un nuevo archivo JSON en `config/entry_templates/` (y sumar la categoría al tipo `EntryCategory` y al CHECK de SQLite).
