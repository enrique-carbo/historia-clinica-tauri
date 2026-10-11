# AGENTS.md

Instrucciones para agentes de código. opencode las carga automáticamente en
cada sesión de este proyecto (también las leen otros agentes — estándar
`AGENTS.md`). Mantené este archivo actualizado cuando cambien los baselines.
El "porqué" del diseño vive en `README.md` y `docs/` — acá va lo operativo.

## Verificación (antes de dar por terminado un cambio)

```bash
cd src-tauri
cargo test                    # baseline: 101 tests, 0 fallas
cargo clippy --all-targets    # baseline: 3 warnings, 0 errores
cd .. && npx tsc --noEmit     # baseline: sin errores
```

- Los 3 warnings de clippy son `too_many_arguments` aceptados en
  `unlock_vault`, `seed_recovery` y `create_entry` (los `State<...>` los
  inyecta Tauri). **No agregar warnings nuevos.**
- **NO correr `cargo fmt`**: el repo no está formateado; fmt reescribiría
  archivos que no tocaste y arruina el diff.
- `package.json` no tiene scripts de lint/test (solo
  dev/build/preview/tauri). Gestor de paquetes: pnpm. Backend: `cargo`
  desde `src-tauri/`.

## Estructura

- `src-tauri/` — backend Rust (Tauri 2), crate `historia_clinica_app_lib`.
  - `src/commands/*.rs` — comandos Tauri. Un command nuevo **debe
    registrarse en `src-tauri/src/lib.rs`** (invoke_handler): si no, el
    `invoke` del frontend falla en runtime y nada lo detecta al compilar.
  - `src/db/migrations/mod.rs` — catálogo de migraciones.
  - `src/export/render.rs` — render del Markdown exportado (puro, sin I/O).
  - `src/security/` — vault, crypto, seed, backup, data_key.
  - `config/entity_schema.json` — **fuente única** del schema de entities
    (filiación del paciente). La consume el backend (`from_embedded()` en
    `db/config_schema.rs`, vía `include_str!`) y el frontend
    (`src/utils/schema.ts`, que exporta `PATIENT_FIELDS` y
    `FLAT_PATIENT_FIELDS`). Los labels del export y de la ficha EHR se
    derivan del schema — **no agregar listas hardcodeadas de campos**.
    Groups (`type: "group"`, ej. responsable): la UI los renderiza como
    sección, pero el storage guarda los subcampos **aplanados con prefijo
    `{group}_{campo}`** (ej. `responsable_nombre_completo`); los
    `required` de un grupo solo aplican si el grupo tiene algún dato
    ("nada o todo"). **No recrear una copia en `src/`**: el watcher de
    Vite ignora `src-tauri/`, así que el archivo vive acá para que Tauri
    lo detecte y reinicie cargo al cambiar.
- `src/` — React + TS. Vistas en `src/components/views/`, stores en
  `src/stores/`.
- `config/entry_templates/*.json` — labels y orden canónicos de entries.

## Reglas del proyecto

- **Sesión**: el `user_id` nunca se confía en lo que manda el frontend.
  Derivar con `require_session()` / `require_admin()` (definidos en
  `src-tauri/src/commands/admin_commands.rs`).
- **Auditoría**: `record_audit()` guarda solo nombres de campos y
  metadatos, **nunca valores** — los datos van cifrados en reposo y
  `audit_log` es texto plano.
- **Migraciones**: append-only. Nunca editar una versión ya aplicada.
  Excepción en dev: podés editar `v001_baseline` solo si borrás la base
  después (una DB nueva arranca en `user_version = 0`).
- **Zeroize**: claves en `Zeroizing<...>`; deref coercion mantiene los
  call sites (`&key` → `&[u8; 32]`), usá `&mut *x` para `fill_bytes` /
  `hash_password_into`. La sección "Límites de zeroize" del README es la
  verdad oficial: no sobrevender limpieza de RAM.
- **Firmas**: `signed_hash()` (en `entry_commands.rs`) es la única fuente
  de verdad. `SignedMaterial` tiene orden de campos fijo y hay que
  verificar cada campo de la fila (payload, created_at, id incluidos).
- **Mapas serializados**: si un mapa se convierte en bytes que otro
  componente lee, compara o firma, usar BTreeMap. Ojo: hoy la firma no
  depende del orden de claves (el payload se firma como hex crudo) — el
  riesgo real es re-serialización divergente, no el nonce AES-GCM.

## Docs

- `README.md` — seguridad, esquema, roles, tests, export (fuente del porqué).
- `docs/estrategia_encriptacion.md` — diseño cripto por capas.
- `docs/fase4-diseno-sync.md` — diseño de sync (fase 4) y deuda pendiente.
- `docs/2026-julio8-documentacion-maestra.md` — snapshot histórico; puede
  estar desactualizado respecto del esquema actual.
