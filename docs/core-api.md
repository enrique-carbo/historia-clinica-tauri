# CoreApi — capa de servicio del Core

### v1 — 2026-11-21 | CoreApi y PluginHost implementados (2026-11-22)

> Capa de servicio en Rust puro (`src-tauri/src/core/`). Implementado:
> `fetch_filiation`, `entity_exists`, `fetch_professional`, `audit`,
> `wrap_with_datakey` / `unwrap_with_datakey`; el contrato
> `Module` / `ModuleContext`; y el `PluginHost` con `module_execute`,
> `is_module_available`, `list_modules` y activación/desactivación
> (`v002`: `modules` + `module_access`). Pendiente: el primer módulo
> (solicitudes). Alineado con Core mínimo: **no** reescribe los 38 commands
> del frontend.

---

## 1. Por qué angosto y sin `tauri::State`

El contrato pide una capa de servicio para que los módulos pidan datos del Core
sin abrir su DB ni ver la DataKey. Los commands siguen siendo la superficie IPC
del frontend; `src/core/` es la superficie de **servicio** del Core hacia
adentro.

Dos reglas de diseño evitan capas redundantes:

1. **Un módulo no necesita los 38 commands**: necesita proyecciones, auditoría y
   crypto opt-in.
2. **`src/core/` no importa `tauri`**: las funciones de servicio operan
   directamente sobre tipos de dominio (`&Connection`, `&[u8; 32]`,
   `&SchemaConfig`, `&SessionUser`). La implementación in-process de
   `ModuleContext` (en el `PluginHost`) es la única que toma los locks de
   `DbState`/`DataKey` desde Tauri y delega en `core::api::*`. Así no hay un
   struct `CoreApi<'a>` intermedio peleando con lifetimes de `tauri::State<'a, T>`
   y los tests son Rust puro con SQLite en memoria.

## 2. Ubicación y forma

```
src-tauri/src/core/
├── mod.rs      # pub mod api, dto, module, host
├── api.rs      # Funciones de servicio puras (sin dependencia de Tauri)
├── dto.rs      # Anchor/DTOs de dominio (Filiation*, Professional*, AuditEvent)
├── module.rs   # Contrato: Module, ModuleContext, ModuleDb, ModuleError
└── host.rs     # PluginHost: gate, DBs de módulo, activación/desactivación
```

El `ModuleContext` in-process vive en `commands/module_commands.rs` (es el
borde Tauri: toma los `State` y delega en `core::api`), por eso `core::host`
no importa `tauri`.

- **Cero cambios de comportamiento** en `types.rs`, `with_conn`, `get_key`,
  `get_data_key` ni en los 38 commands. `lib.rs` expone `pub mod core;`.
- **Dos movimientos con re-export** (call sites intactos): `record_audit` pasó
  a `core::api` y `License` a `core::dto`; `commands` los re-exporta.
- `InProcessModuleContext` (cuando se conecte `PluginHost`) guarda el actor de
  la sesión ya resuelto por `require_session()` y adquiere `&Connection` /
  `Zeroizing<[u8; 32]>` por llamada (nunca cachea la `DataKey`, para que
  `lock_vault` surta efecto inmediato).

## 3. Superficie v1 (`core::api`)

| Función | Firma conceptual e implementación |
|---|---|
| `fetch_filiation` | `(conn, data_key, schema, actor: &SessionUser, entity_id: i64) -> Result<FiliationDto, CommandError>`. **Scoping obligatorio**: si `actor.role == "paciente"`, verifica en `users.entity_id` que `entity_id` sea el propio; si no, devuelve `AccessDenied` (nunca `EntityNotFound`, §12). Descifra la entidad y proyecta `FiliationDto` |
| `entity_exists` | `(conn, actor: &SessionUser, entity_id: i64) -> Result<bool, CommandError>`. Mismo scoping para `paciente`; `SELECT 1 FROM entities WHERE id = ?1` para staff |
| `fetch_professional` | `(conn, data_key, user_id: &str) -> Result<ProfessionalDto, CommandError>`. Proyección acotada equivalente a `load_profile` (`full_name`, `profession`, `specialty`, `licenses`, `public_key_hex`) |
| `audit` | `(conn, actor_user_id: &str, event: &AuditEvent) -> Result<(), CommandError>`. Usa `event.action()` + `to_detail()` (sin PHI) y `core::api::record_audit` (movido acá; re-exportado por `commands`) |
| `wrap_with_datakey` / `unwrap_with_datakey` | `(data_key: &[u8; 32], bytes) -> Result<..., CommandError>` sobre `crypto::encrypt_bytes` / `decrypt_bytes` (formato `nonce‖ciphertext`; el plaintext vuelve en `Zeroizing`) |

**Fuera del primer corte**: `wrap_for_user` / `unwrap_for_user` (solo
teleconsulta) y `http_fetch` (allowlist, auditar host+path sin query — lo más
sensible; ningún módulo actual lo necesita). `fetch_entry_metadata`, para
verificar un `entry_id` de SOAP si algún módulo post-v1 lo requiere, se agrega
después con `default → Err(Unsupported)` (§4).

## 4. DTOs alineados con el código real y los perfiles Med/Vet

Los DTOs viven en `core::dto`, derivan `Serialize + Deserialize` desde el día 1
(Ruta 2 es un swap de transporte, §11) y son proyecciones acotadas: nunca
`enc_data_blob`.

### `FiliationDto` (agnóstico de Med/Vet, igual que `export/render.rs`)

```rust
pub struct FiliationFieldDto {
    pub key: String,
    pub label: String,
    pub value: String,
}

pub struct FiliationDto {
    pub entity_id: i64,
    pub full_name: String,
    pub document: String,
    pub fields: Vec<FiliationFieldDto>,
}
```

- `document`: se extrae usando `schema.get_blind_index_field("patient")`
  (helper ya existente en `config_schema.rs`).
- `full_name`: `nombre` + `apellido` (misma convención que `render.rs:175`).
- `fields`: pares `(key, label, value)` no vacíos en el orden canónico de
  `schema.field_labels("patient")`. Esto permite que una orden médica en Med
  disponga de `seguro_salud` / `afiliado` y en Vet de `especie` / `raza` /
  `responsable_*` **sin hardcodear listas de campos ni acoplar tests al
  contenido de `profiles/{med,vet}/entity_schema.json`** (regla de `AGENTS.md`).

### `ProfessionalDto` (alineado con `ProfessionalProfile`)

```rust
pub struct ProfessionalDto {
    pub user_id: String,
    pub full_name: String,
    pub profession: String,
    pub specialty: String,
    pub licenses: Vec<License>, // { number, jurisdiction }; la 1ª es la principal
    pub public_key_hex: String,
}
```

Incluye `profession` y `licenses: Vec<License>` (con jurisdicción), coincidiendo
con `ProfessionalProfile` y `ExportProfessional`.

## 5. Riesgos y mitigaciones

| Riesgo | Mitigación |
|---|---|
| `DataKey` cacheada → `lock_vault` no surte efecto | `core::api` recibe `&[u8; 32]` por llamada; `InProcessModuleContext` llama a `get_data_key()` en cada invocación y no guarda la clave |
| Scoping de `paciente` omitido en `fetch_filiation` | `fetch_filiation` recibe `&SessionUser` y valida `users.entity_id == Some(entity_id)` cuando `role == "paciente"`, devolviendo `AccessDenied` si no coincide |
| Mutex sync dentro de `ModuleContext` async | Ningún `MutexGuard` cruza un `.await`: lock, llamada síncrona a `core::api::*`, drop del guard y recién después retorno async |
| Segundo catálogo de errores | Reutilizar `CommandError` con códigos `ERR_*`; el frontend traduce con `formatError()` en `src/utils/errors.ts` |
| PHI en `audit_log` | `AuditEvent` enum cerrado; `to_detail()` testeado con fixtures que contengan nombres propios y `".md"` |

## 6. Testing y verificación

- Tests unitarios en `core::api` con SQLite en memoria y **schema fixture
  propio** (sin acoplar a `entity_schema.json`):
  - `fetch_filiation` para staff y scoping para `role == "paciente"` (propia
    entity → ok; ajena → `AccessDenied`).
  - `entity_exists` (existente, inexistente y scoping paciente).
  - `fetch_professional` con múltiples matrículas y profesión.
  - `audit` anti-PHI en todas las variantes de `AuditEvent`.
- Los 147 tests previos de Rust y los 38 commands quedan intactos.
- Baseline actualizado a **180 tests** en `AGENTS.md` y `README.md`;
  `cargo clippy --all-targets` sigue en **3 warnings**. No correr `cargo fmt`.

## 7. Cortes

1. ✅ `core/` (`mod.rs`, `dto.rs`, `api.rs`) con `fetch_filiation`,
   `entity_exists`, `fetch_professional` y `audit`.
2. ✅ Crypto opt-in: `wrap_with_datakey` / `unwrap_with_datakey` + tests.
3. ✅ Contrato de módulos: `core::module` (`Module`, `ModuleContext`,
   `ModuleRequest`/`ModuleResponse`/`ModuleDescriptor`/`ModuleError`).
4. ✅ `PluginHost` (`core::host`) + `ModuleContext` in-process + `module_execute`,
   `is_module_available`, `list_modules`, activación/desactivación y migración
   **v002** (`modules`/`module_access`). `PluginHost::default_modules()` vacío.
5. ⬜ Módulo de solicitudes (crate propio, `migrate()` y acciones).

**Decisión tomada** (`docs/module-contract.md` §14): **monorepo público** —
`core/` y los crates de módulo viven en el mismo repo (ya público). El paywall
es build-time, no secreto del source (§9).
