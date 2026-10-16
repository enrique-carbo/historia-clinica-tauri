# AGENTS.md

Instrucciones para agentes de código. opencode las carga automáticamente en
cada sesión de este proyecto (también las leen otros agentes — estándar
`AGENTS.md`). Mantené este archivo actualizado cuando cambien los baselines.
El "porqué" del diseño vive en `README.md` y `docs/` — acá va lo operativo.

## Verificación (antes de dar por terminado un cambio)

```bash
cd src-tauri
cargo test                    # baseline: 135 tests, 0 fallas
cargo clippy --all-targets    # baseline: 3 warnings, 0 errores
cd .. && npx tsc --noEmit     # baseline: sin errores
pnpm test                     # baseline: 61 tests, 0 fallas (vitest; Node >= 22.12)
```

- Los 3 warnings de clippy son `too_many_arguments` aceptados en
  `unlock_vault`, `seed_recovery` y `create_entry` (los `State<...>` los
  inyecta Tauri). **No agregar warnings nuevos.**
- **NO correr `cargo fmt`**: el repo no está formateado; fmt reescribiría
  archivos que no tocaste y arruina el diff.
- Gestor de paquetes: pnpm. Backend: `cargo` desde `src-tauri/`.
  Scripts npm: `dev/build/preview/tauri` y `test/test:watch` (vitest).
  **Vitest 5 requiere Node >= 22.12** (el default del repo es nvm v22;
  la CI usa node 22). El repo fija eso con `.nvmrc` + `engines` en
  `package.json`: con Node 18 jsdom 30 revienta (`ERR_REQUIRE_ESM`) y
  los tests de `dateUtils` fallan por ICU viejo.
- Tests frontend: co-located `src/**/*.test.ts`; `@tauri-apps/api/core`
  se mockea con `vi.mock`. Igual que en Rust: los tests de lógica usan
  **fixtures propios** — no acoplarlos al contenido de
  `entity_schema.json` (rompió la CI una vez); solo tests de invariantes
  estructurales.

## Estructura

- `src-tauri/` — backend Rust (Tauri 2), crate `fichio_care_lib`.
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

- **Core mínimo**: funcional pero mínimo — nada de features "por si acaso"
  ni placeholders en el Core. Lo no-esencial (teleconsulta, métricas, FHIR,
  terminología, OS, IA) va a **plugins** cuando haga falta. Ver
  `docs/PLUGIN_ARCHITECTURE.md` (Principio Rector) para la lista
  Core/no-Core.
- **Productos Med/Vet**: un core, dos perfiles de build mínimos
  (identifier + branding + `entity_schema.json`); el Core es agnóstico
  de especie. Pedidos de clientes específicos → config o plugin, nunca
  forks ni `#ifdefs` en Core (ver `docs/PLUGIN_ARCHITECTURE.md`,
  "Perfiles de producto").
- **Sesión**: el `user_id` nunca se confía en lo que manda el frontend.
  Derivar con `require_session()` / `require_admin()` /
  `require_admin_or_assistant()` (definidos en
  `src-tauri/src/commands/admin_commands.rs`).
- **Auditoría**: `record_audit()` guarda solo nombres de campos y
  metadatos, **nunca valores** — los datos van cifrados en reposo y
  `audit_log` es texto plano.
- **Wrap de paciente**: `save_user_wrap()` **nunca** para un `user_id` con
  `role: 'paciente'` — el paciente no tiene wrap personal (ausencia
  estructural de la DataKey: unlock, change_password y admin_reset la
  respetan; los tests de `auth_commands`/`admin_commands` la clavan).
  `save_user_wrap` no conoce roles: **cualquier command nuevo que escriba
  wraps debe verificar el rol antes**.
- **Blind index**: `generate_blind_index(dni, &data_key)` es HMAC-SHA256
  con clave derivada de la DataKey (label `fichio/blind-index/v1`).
  **No existe `.installation_salt`** — no crearla, no sincronizarla, no
  respaldarla: un índice solo es verificable (no invertible) con la
  DataKey.
- **Migraciones**: append-only. Nunca editar una versión ya aplicada.
  Excepción en dev: podés editar `v001_baseline` solo si borrás la base
  después (una DB nueva arranca en `user_version = 0`).
- **Zeroize**: claves en `Zeroizing<...>`; deref coercion mantiene los
  call sites (`&key` → `&[u8; 32]`), usá `&mut *x` para `fill_bytes` /
  `hash_password_into`. La sección "Límites de zeroize" del README es la
  verdad oficial: no sobrevender limpieza de RAM.
- **Firmas**: `signed_hash()` (en `entry_commands.rs`) es la única fuente
  de verdad del material de las entries. `SignedMaterial` tiene orden de
  campos fijo y hay que verificar cada campo de la fila (payload,
  created_at, id incluidos). El **manifest de respaldos** se firma con
  su propio material canónico (`ManifestSignable` en
  `security/backup.rs` — no reusar `signed_hash`); la verificación
  ancla la pública en el histórico local `signing_keys` vía
  `public_key_at`, **nunca** en la clave embebida en el propio
  manifest. Firma inválida = restore bloqueado; no verificable (legado
  o firmante ajeno) = solo con confirmación explícita.
- **Mapas serializados**: si un mapa se convierte en bytes que otro
  componente lee, compara o firma, usar BTreeMap. Ojo: hoy la firma no
  depende del orden de claves (el payload se firma como hex crudo) — el
  riesgo real es re-serialización divergente, no el nonce AES-GCM.

## Docs

- `README.md` — seguridad, esquema, roles, tests, export (fuente del porqué).
- `docs/estrategia_encriptacion.md` — diseño cripto por capas.
- `docs/fase4-diseno-sync.md` — diseño de sync (fase 4) y deuda pendiente.
- `docs/plugin-teleconsulta-v1.md` — diseño del plugin de teleconsulta (v4,
  paciente como usuario `role: 'paciente'`; alcance mínimo: canal + pago +
  SOAP como entry del Core).
