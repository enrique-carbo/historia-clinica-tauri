# CoreApi — capa de servicio del Core

### v1 — 2026-11-21 | Diseño, no implementación

> Prerequisite de la Ruta 1 (`docs/module-contract.md` §14): hoy `CoreApi`
> tiene 0 referencias en el código. Es el servicio sobre el que se
> implementa `ModuleContext` in-process (§4). Alineado con el principio de
> Core mínimo: **no** implica reescribir los 38 commands de `lib.rs`.

---

## 1. Por qué angosto

El contrato pide "refactorizar `commands/` en una capa de servicio", pero el
requisito real es más chico: que los módulos puedan pedir datos del Core sin
abrir su DB ni ver la DataKey. Los commands siguen siendo la superficie IPC
del frontend; `CoreApi` es la superficie de **servicio** del Core hacia adentro.

Un módulo no necesita los 38 commands. Necesita proyecciones, auditoría y
crypto opt-in. Empezar por eso evita un refactor grande y no agrega nada
"por si acaso".

## 2. Ubicación y forma

```
src-tauri/src/core/
├── mod.rs      # mod api; mod dto;
├── api.rs      # CoreApi<'a> + CoreHandles
└── dto.rs      # FiliationDto, ProfessionalDto, AuditEvent
```

- `CoreApi<'a>` **guarda los `State<'a, T>` por valor** (`DbState`, `DataKey`,
  `SessionState`, `CryptoState`, `SchemaConfig`). Son newtypes baratos sobre
  `&T` e implementan `Clone`. No cambia `types.rs`, no agrega `app.manage`,
  no modifica `lib.rs` salvo por `mod core;`.
- Se construye por llamada, igual desde un command que desde `module_execute`.
- **Cero cambios en `with_conn`, `get_key` ni `get_data_key`**: los métodos
  de `CoreApi` los llaman con los mismos `&State`/`&DataKey` de siempre.
  Cambiar sus firmas a `&DbState`/`&CryptoState` habría tocado helpers usados
  por los 38 commands solo para ganar comodidad en los tests — se descartó.
- Para testear sin Tauri, la lógica se separa en **funciones puras**
  (`project_filiation(conn: &Connection, key: &[u8; 32], schema: &SchemaConfig,
  entity_id) -> Result<FiliationDto, CommandError>`, etc.). El método de
  `CoreApi` solo adquiere el state y delega; el test llama a la función pura
  con una DB en memoria. `State` tiene el campo privado y no se puede
  construir a mano en un test.

## 3. Superficie v1

| Método | Implementación |
|---|---|
| `fetch_filiation(entity_id)` | Extraer de `get_entity` un helper `load_entity(...) -> EntityRecord`, y proyectar solo los campos del DTO. **Un solo camino de descifrado** |
| `entity_exists(id)` | `SELECT 1` vía `with_conn` |
| `fetch_professional(user_id)` | Proyección del perfil profesional del command existente |
| `audit(actor, AuditEvent)` | Reutilizar `record_audit()` → `to_detail()` sin PHI |
| `wrap_with_datakey` / `unwrap_with_datakey` | `get_data_key` + `crypto::*` (sumar cuando un módulo lo pida) |

**Fuera del primer corte**: `wrap_for_user` / `unwrap_for_user` (solo
teleconsulta) y `http_fetch` (allowlist, auditar host+path sin query — lo más
sensible; ningún módulo actual lo necesita). `fetch_entry_metadata`, para
verificar el `entry_id` de la SOAP en solicitudes, se agrega después con
`default → Err(Unsupported)` (§4).

## 4. DTOs y perfiles

Los DTOs van en `core::api::dto`, derivan `Serialize + Deserialize` desde el
día 1 (Ruta 2 es un swap de transporte, §11) y son proyecciones acotadas:
nunca `enc_data_blob` ni filas completas.

`FiliationDto { entity_id, full_name, document, birth_date, sex }` (§4). El
problema: los nombres de campo viven en `profiles/{med,vet}/entity_schema.json`
y **no coinciden** — Med no tiene `sexo`, Vet sí; el document es `dni` en
ambos. Reglas:

- `document` se resuelve con `schema.get_blind_index_field()` (helper ya
  existe), no con un string hardcodeado.
- `full_name` = `nombre` + `apellido`; `birth_date` = `fecha_nacimiento` si
  está; `sex` = `sexo` si está (`None` en Med).
- Un **test de invariante estructural** sobre ambos schemas falla en CI si un
  perfil renombra estos campos. No se acoplan tests al contenido del schema
  más allá de esa invariante (regla del repo).

## 5. Riesgos y mitigaciones

| Riesgo | Mitigación |
|---|---|
| `CoreApi` guarde la DataKey → `lock_vault` no surte efecto | El struct no tiene campos de clave: `get_data_key()` en cada llamada. Test unlock → fetch → lock → fetch falla |
| Mutex sync dentro de `ModuleContext` async | Ningún `MutexGuard` a través de un `.await`: DB síncrona, guard suelto antes del await |
| Se salte el gate §5 | `CoreApi` recibe `&SessionUser` ya resuelto por `require_session()`; `role: String` → `ActorRole` enum con fail-closed |
| Segundo catálogo de errores | Reutilizar `CommandError` con `ERR_*`; `ModuleError` mapea a esos códigos y el frontend agrega las entradas en `src/utils/errors.ts` |
| PHI en el audit | `AuditEvent` enum cerrado, `to_detail()` con test anti-PHI (nombre y `".md"` en la fixture) |
| `too_many_arguments` en clippy | Agrupar los states en `CoreHandles` (los 3 warnings permitidos ya están ocupados) |
| Ventana entre chequear sesión y key | Re-chequear en el punto de uso; aceptado en v1 y documentado, como los límites de zeroize |

## 6. Testing y verificación

- Tests nuevos sobre las **funciones puras** con DB en memoria: proyección de
  filiación en ambos perfiles, `entity_exists`, `audit` anti-PHI, roundtrip
  crypto, vault-locked y rol desconocido.
- Los tests de los 38 commands quedan intactos: `CoreApi` es aditivo y no
  modifica ningún helper de `commands/mod.rs`.
- Al implementar, sube el baseline Rust (hoy 147) → actualizar `AGENTS.md` y
  README. `cargo clippy --all-targets` debe seguir en 3 warnings. No correr
  `cargo fmt`.

## 7. Primer slice

1. Crear `core/` con `mod.rs`, `dto.rs` y `api.rs`, sin tocar `commands/`.
2. Funciones puras de proyección + cuatro métodos: `fetch_filiation`,
   `entity_exists`, `fetch_professional`, `audit`, con sus tests.
3. Crypto y `http_fetch` en cortes posteriores, igual de angostos.

**Decisión tomada** (`docs/module-contract.md` §14): **monorepo público** —
`core/` y los crates de módulo viven en el mismo repo (ya público). El
paywall es build-time, no secreto del source (§9). No bloquea este slice: el
`core/` de `src-tauri/src/core/` es la capa de servicio; los módulos se
agregan como crates hermanos cuando se implemente el primero.
