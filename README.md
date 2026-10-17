# 🩺 Fichio Care — Historia Clínica Digital

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](./LICENSE)

Aplicación de escritorio nativa orientada a la gestión de historias clínicas y telemedicina asíncrona con un enfoque **Local-First** y arquitectura de seguridad **Zero-Knowledge Parcial** (Cifrado de Extremo a Extremo).

El sistema garantiza la máxima privacidad procesando y cifrando toda la información médica sensible directamente en la RAM del hardware local antes de persistirla en el disco o sincronizarla. El servidor remoto actuará en el futuro únicamente como un "tubo ciego" que almacena texto cifrado, sin capacidad de leer la información.

## ⚡ Stack Tecnológico

La arquitectura está dividida de forma estricta en dos capas de alto rendimiento:

### Backend Nativo (Core - Rust)
- **Rust**: Motor principal encargado de la criptografía, seguridad y acceso a datos.
- **Tauri v2**: Puente IPC multiplataforma de consumo ultra-bajo (plugins: `opener` para revelar archivos, `dialog` para elegir carpetas de respaldo).
- **SQLite (`rusqlite`)**: Base de datos relacional embebida (acceso directo sin plugins de Tauri para mantener el patrón `with_conn`), garantizando autonomía total offline.
- **AES-GCM-256**: Cifrado autenticado para texto médico (entries, nombres, perfil) y para envolver la DataKey en disco.
- **Ed25519 (Dalek)**: Firma digital asimétrica para garantizar el No-Repudio y validez legal de las notas médicas.
- **SHA-256**: Hashing determinista para Índices Ciegos, huellas digitales de documentos y hash del export.
- **Argon2id**: Passwords de usuario y derivación de la Seed Phrase (master wrap).
- **Argon2id**: Hashing de contraseñas y derivación de claves (KDF) — incluye el envoltorio de la DataKey.
- **Seed Phrase**: Frase mnemotécnica de 6 palabras en papel para bootstrap y recovery de la DataKey — y para el **break-glass** de recuperación de la contraseña del administrador (auditado). No se persiste en disco.
- **Respaldos locales**: carpeta `respaldo-{timestamp}/` con `manifest.json` firmado (SHA-256 por archivo + firma Ed25519 del admin); todo el contenido ya va cifrado en reposo.

### Frontend (UI)
- **React 19 + TypeScript (Strict)**: Interfaz declarativa con tipado extremo a extremo (los structs de Rust se espejan en TS).
- **Zustand**: Gestor de estado global ultraligero. Maneja sesiones de seguridad y datos clínicos sin boilerplate ni props-drilling.
- **Tailwind CSS v4**: Framework de utilidades CSS para diseño rápido, consistente y responsivo.
- **Vite 7**: Entorno de compilación rápida con HMR (Hot Module Replacement).
- **pnpm**: Gestor de paquetes eficiente mediante hard links.

## 📂 Arquitectura del Proyecto

### Backend Core (Rust)

```text
src-tauri/src/
 ├── main.rs / lib.rs      # Entry + orquestador Tauri (estados: DbState, CryptoState, SigningState, DataKey)
 ├── types.rs              # Contenedores de estado seguro (Mutex<Option<T>>)
 ├── security/             # auth (Argon2id), crypto (AES-GCM/Ed25519/blind index), data_key (wrap), seed (mnemonic), vault, backup
 ├── db/                   # config_schema (schema embebido), database (WAL), migrations/ (append-only, PRAGMA user_version)
 ├── export/render.rs      # Snapshot tipado + render puro a Markdown
 └── commands/             # admin, auth, backup, entity, entry, export, professional_profile, seed (aduana IPC)
```

### Frontend UI (React)

```text
src/
 ├── stores/               # Zustand: auth, entity, entry, navigation, seed, patient
 ├── components/
 │   ├── ui/               # Design System atómico (Button, Card, Drawer…)
 │   ├── layouts/          # DashboardLayout (logout seguro + seed)
 │   └── views/            # AdminView (Usuarios/Admisión/Auditoría/Seed/Respaldos), MedicoView (Perfil/Paciente/EHR/Entries+export), PacienteView
 └── (raíz)                # PatientEhrView, Entity, EntryTimeline/Card/Form, SeedPhraseSetup, AuthBox
```

> 🎨 Las reglas visuales del frontend (tokens de color, tipografía, radios, estados interactivos, semántica de color y estrategia de temas claro/oscuro) están definidas en **[DESIGN.md](./DESIGN.md)**.

## Principios de Diseño del Core & Frontend

- **Patrón `with_conn`**: Ningún comando saca la conexión a la base de datos de su `Mutex`. Se le inyecta un closure para garantizar que el lock se libere instantáneamente, evitando congelamientos de UI.
- **Autorización en Backend (`SessionState`)**: Ningún command sensible confía en el frontend. `login_user` registra la sesión activa en Rust y `require_admin()` (en `admin_commands.rs`) exige rol `administrador` antes de tocar usuarios, contraseñas, la semilla o los respaldos. `lock_vault` limpia la sesión.
- **Inyección de Dependencias**: Los comandos que requieren cifrado reciben `State<'_, DataKey>` o `State<'_, SigningState>`, extraen la llave y fallan de forma segura si no está inicializada.
- **Estado Reactivo Seguro (Zustand)**: El Frontend jamás almacena contraseñas ni claves de cifrado en JavaScript. Los Stores solo contienen booleanos de estado y datos descifrados para renderizar.
- **Store Unificado de Paciente**: `usePatientStore` es la fuente única de verdad para la selección de paciente. `PatientEhrView`, `Entity`, `EntryForm` y `EntryTimeline` consumen el mismo estado, garantizando sincronización instantánea sin props-drilling.
- **Entries Append-Only**: Las entries son inmutables una vez creadas. No existen operaciones de update ni delete, garantizando integridad y no-repudio. La corrección se hace con nuevas entries (ej: "Corrección de...").
- **Flujo Unificado de EHR**: La interfaz `PatientEhrView` consolida búsqueda, ficha y evoluciones en una sola vista, eliminando el context-switching entre tabs.
- **Limpieza de Datos Sensibles**: El frontend limpia las referencias de la contraseña y la frase en React apenas se usan (tras `unlock_vault` o `seed_recovery`). En Rust, las llaves viven en `Zeroizing<[u8; 32]>` y las contraseñas/frases en `Zeroizing<String>`: al liberarse (unlock, `lock_vault`, fin de cada command) los bytes se **ponen a cero con `zeroize`** en vez de quedar como residuo en el heap. `lock_vault` además descarta los `Option` de `CryptoState`, `SigningState` y `DataKey` y limpia la `SessionState`. Ver `docs/estrategia_encriptacion.md` §4.7 "Límites de zeroize" — esto es *best-effort*, no borrado criptográfico garantizado.
- **DataKey Nunca en Claro**: La llave de cifrado de datos médicos no se persiste en texto plano. Se envuelve con Argon2id(password) por usuario (fast path) y con Argon2id(seed, salt de dominio `fichio/seed-kdf/v1`) en el master wrap (bootstrap/recovery). La seed vive en papel, no en disco.
- **Seed Phrase en Papel**: La frase semilla (6 palabras) no se persiste en `localStorage`. Solo el flag `isSeedConfigured` se guarda; la frase se muestra una vez en `SeedPhraseSetup` para anotarla. Se tipea únicamente en bootstrap de usuarios nuevos o recovery.

## 🔒 Seguridad (Zero-Knowledge Parcial)

> **Alcance del Zero-Knowledge:** protege el **disco** — sin la contraseña o la frase semilla, la base local es incomprensible para alguien con acceso físico. **No** aísla a los co-usuarios de la misma instalación: la DataKey es compartida por diseño, así que todos los usuarios staff con sesión abierta descifran los mismos datos clínicos. Entre usuarios el control es de autorización, no criptográfico. El rol `paciente` es la excepción estructural: no tiene DataKey.

Resumen (detalle completo en **[docs/estrategia_encriptacion.md](./docs/estrategia_encriptacion.md)**):

- **Claves en RAM**: nacen en `unlock_vault` y mueren en `lock_vault` (cada `Zeroizing` se pone a cero al caer). La DataKey no existe al arrancar la app. El borrado es *best-effort* — límites honestos en §4.7 del doc.
- **Vaults por usuario** (`vault_{user_id}.bin`): llave privada Ed25519 de firma, una por profesional.
- **DataKey por instalación**: cifra todo lo clínico; nunca se persiste en claro — wrap personal con Argon2id(password) (fast path) y master wrap `.data_key.master` con Argon2id(seed, sal de dominio `fichio/seed-kdf/v1`).
- **Blind index**: HMAC-SHA256 con clave derivada de la DataKey sobre el DNI normalizado — búsqueda exacta sin exponer el dato y sin sal que respaldar/sincronizar.
- **Firma Ed25519 por entry**: sobre el JSON canónico de **todos** los campos de la fila (bytes persistidos incluidos); el histórico `signing_keys` permite verificar por el timestamp de la entry aunque roten claves.
- **Frase semilla** (6 palabras, en papel, nunca en disco): bootstrap y recovery del master wrap; break-glass `seed_recovery` (audita) cuando se olvidó todo.

## 👥 Roles y Autorización

| | `administrador` | `asistente` (ex-`admin`) | `profesional` | `paciente` |
|---|---|---|---|---|
| Admisión de pacientes (Entity) | ✅ | ✅ | — | — |
| Vista clínica (EHR, entries, export .md) | — | — | ✅ | — |
| Alta de cuentas de usuario | ✅ | ❌ | ❌ | ❌ |
| Otorgar acceso de paciente (desde su ficha) | ✅ | ✅ | ❌ | ❌ |
| Baja lógica (`is_active`) / reactivación | ✅ | ❌ | ❌ | ❌ |
| Reset de contraseña de otros | ✅ (vault regenerado + auditoría) | ❌ | ❌ | ❌ |
| Rotar la frase semilla | ✅ (con verificación + auditoría) | ❌ | ❌ | ❌ |
| Ver auditoría | ✅ | ❌ | ❌ | ❌ |
| Cambiar la propia contraseña | ✅ (con la previa — sin tocar firma) | ✅ | ✅ | ✅ |

- **Alta de cuentas**: el administrador desde su panel (`admin_create_user`); y admin + asistente otorgan la **cuenta de paciente** desde la ficha (`grant_patient_access` — alta lazy, un solo acceso activo por ficha, ligada con `users.entity_id`). El registro público solo existe cuando la instalación **no tiene usuarios** y siempre crea un `administrador` (bootstrap) — después queda cerrado.
- **Baja lógica**: `is_active = 0` bloquea el próximo login; la fila y el historial permanecen. No se puede desactivar a uno mismo ni dejar la instalación sin administradores activos.
- **Tres caminos para cambiar una contraseña**: (1) el usuario con su contraseña previa (`change_password` — conserva la llave de firma), (2) el administrador sin la previa (`admin_reset_password` — regenera el vault con keypair nuevo; el histórico `signing_keys` mantiene verificadas las entries anteriores; si el target es paciente **no hay wrap que re-envolver**), (3) el **break-glass con frase semilla** (`seed_recovery` desde el login — verifica la posesión de la frase contra `.data_key.master`, restablece la contraseña del administrador activo más antiguo y queda en `audit_log`; la DataKey no cambia).
- **Reparto deliberado de confidencialidad**: la DataKey es única por instalación, así que cualquier usuario logueado **de staff** ya accede a los mismos datos cifrados (el filtrado es de UI por rol). El rol `paciente` es la excepción por diseño: inicia sesión con username/contraseña pero **no tiene DataKey** (ausencia estructural) — todos los commands clínicos le fallan. El rol `administrador` añade poder sobre **cuentas y credenciales**, no sobre los datos clínicos — y cada uso de ese poder queda en `audit_log`.

## 🗄️ Base de Datos y Templates

> Esquema completo (tablas, SQL de `entries`, arquitectura de templates):
> **[docs/esquema-db.md](./docs/esquema-db.md)**.

- **Migraciones append-only** con `PRAGMA user_version` (transacción por migración, guard de downgrade).
- **Único sistema clínico**: la tabla `entries` — inmutables (sin `update`/`delete`), payload cifrado con DataKey, firma Ed25519; todo lo legacy (`medical_history`, `notes`) fue eliminado.
- **Filiación** en `entities` (cifrada campo a campo); usuarios/roles en `users`; histórico de claves en `signing_keys`; auditoría en `audit_log` (solo metadata, nunca valores).
- **Templates**: `profiles/{med,vet}/entity_schema.json` (fuente única de labels y orden — form, ficha EHR y export) + `config/entry_templates/*.json` (formularios de entries), todo embebido en build time, offline e inmutable en runtime.

## 📤 Exportación de Historia Clínica (Markdown)

La historia completa de un paciente se exporta a un archivo `.md` legible y versionable. El diseño es **stateless respecto a claves y de confianza mínima**: el snapshot (paciente + perfil + entries ya descifrados) se arma y se renderiza **dentro de Rust** y nunca cruza el IPC — el frontend solo manda el `subject_id` y recibe `{ path, filename, entries_count }`.

### Flujo

```text
MedicoView (tab Entries) ─ "Exportar .md"
  │
  └─ export_history(subject_id)          ← único roundtrip IPC
         1. build_export_snapshot: descifra paciente + perfil + TODAS las
            entries (sesión activa; error claro si la bóveda está cerrada).
         2. render puro a Markdown (labels/orden desde entity_schema.json)
         3. firma Ed25519 del documento completo (clave de la sesión) +
            bloque "Firma digital" con fingerprint de la pública
         4. escritura en <app_local_data_dir>/exports/ + auditoría
            export_emitted (solo metadata: subject_id, entries, sha256 —
            sin filename: correlación con el archivo por hash del contenido)
         → { path, filename, entries_count, sha256 } → banner "✓ firmado ·
           Revelar en carpeta"
```

### Decisiones de diseño

| Aspecto | Decisión | Razón |
|---|---|---|
| Destino | Carpeta fija `exports/` de la app | Sin diálogos en el flujo de export — compatible con el futuro build mobile (`plugin-dialog` se usa solo en Respaldos) |
| Nombre de archivo | `HistoriaClinica_Apellido_Nombre_YYYY-MM-DD_HHMMSS.md` (sanitizado, único por timestamp) | Sin conflictos de sobrescritura ni diálogos |
| Campos por categoría | Labels canónicos de `config/entry_templates/*.json` (orden y nombre) | El `.md` es espejo del formulario clínico |
| Orden | Cronológico ascendente | Lectura continua de la evolución |
| Firma | Check ✓/✗ por entry + **firma Ed25519 del documento completo** (bloque final con firma y fingerprint de la pública) | Integridad + atribución + no-repudio del archivo entregado; la firma cubre el contenido hasta el footer de hash |
| Auditoría | Evento `export_emitted` en `audit_log` con subject_id, entries y sha256 (solo metadata, **sin filename**: el nombre del archivo lleva apellido+nombre del paciente y el audit es texto plano; correlación = hash del contenido) | Constancia de emisión trazable al profesional, sin PHI en claro |
| Integridad | SHA-256 del contenido en el footer + aviso de confidencialidad | Permite detectar alteraciones del archivo exportado |
| "Revelar en carpeta" | `revealItemInDir` de `@tauri-apps/plugin-opener` | Ya instalado y concedido vía `opener:default` (sin cambios de capability) |

El único command registrado es `export_history` (en `lib.rs`), la lógica vive en `commands/export_commands.rs` y el render puro en `export/render.rs` (sin estado, sin I/O → 7 tests unitarios directos). Los `Export*` no llevan serde: al no cruzar IPC, el compilador hace cumplir que el snapshot queda en Rust.

## 🧪 Tests Automatizados

- **Core (Rust)**: 138 tests — `cargo test` desde `src-tauri/` (crypto, vault, seed, data_key, backup, migraciones y todos los commands end-to-end con DB real y `tauri::test`).
- **Frontend**: 73 tests — `pnpm test` (Node >= 22.12; vitest + Testing Library, co-located en `src/**/*.test.{ts,tsx}`, `@tauri-apps/api/core` mockeado con `vi.mock`).

Baselines exactos y checklist de verificación: **`AGENTS.md` §Verificación**. Los tests usan **fixtures propios** — no se acoplan al contenido de `entity_schema.json` (rompió la CI una vez); solo invariantes estructurales.

## 🔄 Versiones y actualizaciones

**Fuente única de versión**: `src-tauri/tauri.conf.json`. Tras un bump ahí,
sincronizar las otras dos copias (`package.json`, `src-tauri/Cargo.toml`):

```bash
pnpm version:sync     # escribe package.json + Cargo.toml
pnpm version:check    # falla si las 3 no coinciden (para CI/verificación)
```

**Update hoy (manual)**: no hay auto-updater todavía — el update es
publicar el instalador nuevo y que el profesional lo instale **encima**.
Eso no toca los datos: la base vive en `app_local_data_dir` y el
`identifier` no cambia entre versiones. Las migraciones (append-only,
`user_version`) corren solas en el primer arranque posterior.

- **Antes de cada actualización: respaldar** (`Respaldos` → archivo
  firmado). Si una migración posterior a un release rompiera algo, el
  rollback es **restaurar el backup con la versión anterior** — no existe
  downgrade de esquema (regla de migraciones append-only).
- Las claves (DataKey, seed, firmas) **no cambian con un update**: la
  seed no se regenera nunca al actualizar.

**Hacia el auto-updater (pendiente, cuando haya ~3+ instalaciones)**:
`tauri-plugin-updater` con artefactos firmados (clave propia, guardada en
CI + copia offline) y endpoints **separados por perfil**
(`med/latest.json` / `vet/latest.json` — son productos distintos).
Primer rollout: notificar y abrir la página de release (ya existe
`opener`), no auto-instalar en silencio. **Solo desktop**: en mobile los
updates los dan las tiendas (Play Store / App Store) — el patrón
unificado es "notificar + abrir enlace". Ver `docs/release.md`.

## 🗺️ Roadmap

Detalle histórico y checklists por fase: **[docs/roadmap.md](./docs/roadmap.md)**.

- ✅ **Fases 1–3.11** — Core local completo: vault, cifrado por capas, Entity-Entry firmado (append-only), roles + auditoría, respaldos firmados, migraciones append-only, export `.md` firmado.
- 🟧 **Fase 4** (próximo): infraestructura y sync híbrida (VPS + PocketBase + outbox `sync_queue`) — `docs/fase4-diseno-sync.md`.
- 🟥 **Fase 5**: teleconsulta — módulo de chat + pago; diseño v5 en `docs/plugin-teleconsulta-v1.md` sobre el contrato de módulos (`docs/module-contract.md`); prerequisite: refactor `commands/` → `CoreApi`.
- ⬜ **Fase 6**: interoperabilidad (export FHIR — futuro).
- 📱 **Mobile**: solo tras la Fase 4 (sync es el prerequisito); un core, N targets; updates vía tiendas — `docs/roadmap.md` §"Futuro: Mobile".

## 🚀 Comandos Útiles de Desarrollo

```bash
pnpm tauri dev          # Levanta el entorno (Rust + React HMR)
pnpm tauri build        # Compila en modo release para producción
npx tsc --noEmit        # Verifica TypeScript sin generar archivos
pnpm test               # Ejecuta los 73 tests unitarios del frontend (vitest)
pnpm test:watch         # Vitest en modo watch
cargo check             # Verifica compilación Rust
cargo test --lib        # Ejecuta los 138 tests unitarios del Core
cargo add <crate>       # Añade dependencias al backend (desde src-tauri)
```

## 📜 Licencia

[AGPL-3.0-or-later](./LICENSE) — Copyright (C) 2026 Fichio Care contributors.

- **Sin garantía** (sección "NO WARRANTY" de la AGPL): software clínico, úselo bajo
  su responsabilidad profesional y con los respaldos correspondientes.
- **Uso en red cubierto**: quien modifique este código y lo ofrezca como servicio
  (ej. un SaaS de historias clínicas derivado de este Core) debe publicar sus
  cambios bajo la misma licencia. Vender **soporte o hosting** del servicio
  propio está permitido — el modelo es open source de verdad, no "source-available".
- El copyright holder puede otorgar **licencias comerciales** aparte (dual
  licensing) a quien necesite usar el código sin las obligaciones de la AGPL.
