# 🧩 Módulo: Teleconsulta Asíncrona

### Versión 5.0 — 2026-10-17 | Alineado con `docs/module-contract.md` (Ruta 1)

> **v5.0 sustituye a v4.0**: la arquitectura de datos cambia. El chat ya no
> vive namespaced en la DB del Core — vive en la **DB propia del módulo**
> (`fichio-teleconsulta.db`, outbox propia, `migrate()` propio), y el módulo
> habla con el Core solo por `ModuleContext` vía `module_execute`. El
> alcance queda **chat + pago (P2)**; la SOAP la escribe el profesional en
> el Core a mano (sin automatismos). v4.0 y anteriores quedan en el
> historial de git.

---

## 1. Visión y alcance

Un canal de comunicación **único y asíncrono** entre el profesional y un
paciente (usuario con `role: 'paciente'`), con workflow de cobro eventual y
cierre clínico a cargo del profesional en el Core.

| En alcance (v1) | Fuera de alcance |
|---|---|
| Un thread de conversación por paciente; ambos lados responden | Video / llamadas en tiempo real |
| Mensajes de texto + foto de comprobante | Multi-thread, bitácora PGHD, signos vitales |
| Pago eventual: QR/transferencia con **confirmación manual** (hitp P2) | Pasarela de pago, webhooks, integraciones bancarias |
| Cierre: **el profesional escribe la entry SOAP en el Core** con el flujo normal (`create_entry`) | El módulo creando entries o referenciándolas automáticamente (`note_reference` → post-v1) |
| Usuario paciente: vault propio, password propia, pairing propio | Paciente viendo su propio EHR (depende del re-wrap, post-v1) |
| Sync del canal por cola propia del módulo (fase 4) | Realtime (suscripciones PocketBase — v2 del módulo) |

**Principio rector (heredado, reforzado):** el módulo es *operativa*; lo
clínico es *Core* — y lo clínico lo escribe **el profesional en el Core**,
no el módulo. El historial del paciente nunca se duplica en datos del
módulo. Y el rol `paciente` **nunca** tiene DataKey (§4): esa separación es
estructural, no una lista de permisos. El paciente **no ve el EHR ni las
entries** del Core.

---

## 2. Qué vive dónde

| Pieza | Hogar | Por qué |
|---|---|---|
| `threads`, `messages`, `payments`, `thread_keys`, outbox | **DB del módulo** `fichio-teleconsulta.db` | El contrato de módulos: datasets operativos en DB propia con `migrate()` propio; atomicidad de outbox dentro del propio archivo; el engine de fase 4 itera las colas de los módulos (`module-contract.md` §13) |
| Usuario paciente | `users` (rol `'paciente'`, ya en el CHECK de v001) | Identidad uniforme: vault, login, sesión, `require_session`, `change_password`, pairing, audit — todo construido y testeado en el Core |
| Vínculo ficha ↔ usuario | `users.entity_id` (FK `entities`, NULL en staff) — **Core** | Creado por `grant_patient_access(entity_id, …)` desde la ficha (alta lazy): sin usuarios fantasma. Gate admin+asistente; temp password (v1) |
| Nota SOAP | `entries` del Core (`create_entry`) — **escrita por el profesional** | Firma Ed25519, inmutabilidad, EHR, export `.md`, verificación, sync — todo gratis. El módulo no crea entries (`module-contract.md` §13) |
| Llaves de thread | Tablas del módulo (wraps) | Wraps vía `ModuleContext` (`wrap_with_datakey` oficina, `wrap_for_user` paciente); el módulo **nunca ve claves** |
| Datasets de referencia (futuro: SNOMED…) | DB propia del módulo | No es sync: artefactos versionados |
| UI | Componentes del módulo (`module_execute` + `is_module_available`) | `module-contract.md` §9 — modo médico y modo paciente |

**Relación con el Core**: el módulo no abre `fichio-core.db`, no llama
commands de Core ni escribe sus tablas — su única superficie es
`ModuleContext` (`fetch_filiation`, `actor`, `audit`, crypto, `http_fetch`).

---

## 3. Modelo de datos (DB del módulo)

```sql
-- Un canal por paciente. Mutables → LWW (blob cifrado + updated_at).
CREATE TABLE threads (
    id              TEXT PRIMARY KEY,            -- UUID
    patient_user_id TEXT NOT NULL,               -- usuario role 'paciente' (sin FK: DB distinta)
    patient_entity_id INTEGER,                   -- sujeto clínico del SOAP (desde users.entity_id al abrir)
    assigned_to_user_id TEXT,                    -- staff: profesional a cargo
    enc_data_blob   BLOB NOT NULL,               -- status, subject (cifrados, DataKey)
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,               -- toca en cada cambio (LWW)
    closed_at       TEXT
);

-- Mensajes: hechos inmutables → unión por id (como entries).
CREATE TABLE messages (
    id              TEXT PRIMARY KEY,            -- UUID
    thread_id       TEXT NOT NULL,
    sender_type     TEXT NOT NULL CHECK(sender_type IN ('patient', 'doctor', 'system')),
    sender_user_id  TEXT NOT NULL,               -- desde require_session (ambos lados)
    kind            TEXT NOT NULL CHECK(kind IN ('text', 'voucher_photo')),
    content_ciphertext BLOB NOT NULL,            -- con thread key
    content_nonce   BLOB NOT NULL,
    created_at      TEXT NOT NULL,
    FOREIGN KEY(thread_id) REFERENCES threads(id)
);

-- Pago: dato OPERATIVO (no clínico). LWW. Paciente se entera por
-- mensaje 'system' en el canal, no leyendo esta tabla.
CREATE TABLE payments (
    id              TEXT PRIMARY KEY,
    thread_id       TEXT NOT NULL,
    enc_data_blob   BLOB NOT NULL,               -- amount_ars, method, status, reference (DataKey)
    confirmed_by_user_id TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,               -- LWW
    FOREIGN KEY(thread_id) REFERENCES threads(id)
);

-- Llave de conversación por thread, envuelta dos veces.
CREATE TABLE thread_keys (
    thread_id            TEXT PRIMARY KEY,
    wrap_doctor_cipher   BLOB NOT NULL,          -- thread key con DataKey (oficina)
    wrap_doctor_nonce    BLOB NOT NULL,
    wrap_patient_cipher  BLOB NOT NULL,          -- thread key con master de la vault del paciente
    wrap_patient_nonce   BLOB NOT NULL,
    created_at           TEXT NOT NULL,
    FOREIGN KEY(thread_id) REFERENCES threads(id)
);

-- Outbox propia del módulo (se llena desde P4; misma transacción que la
-- escritura correspondiente dentro de esta DB).
CREATE TABLE sync_queue (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    collection   TEXT NOT NULL,                  -- threads | messages | payments | thread_keys
    external_id  TEXT NOT NULL,
    op           TEXT NOT NULL CHECK(op IN ('upsert','delete')),
    origin_device TEXT NOT NULL,
    created_at   TEXT NOT NULL
);
-- No hay patient_creds: la vault del paciente ES su credencial.
```

Notas:

- **Sin FKs hacia `users`/`entities`**: SQLite no cruza archivos — la
  integridad referencial hacia el Core la valida el módulo (vía
  `ModuleContext`) y el Core la garantiza de origen (`grant_patient_access`).
- `status` del thread (`open`, `waiting_payment`, `in_consultation`,
  `closed`) va **dentro de `enc_data_blob`**: el servidor no ve ni el estado
  del flujo ni el asunto.
- El servidor ve solo: UUIDs, timestamps, `updated` (LWW), ciphertexts y
  `origin_device`. Residual aceptado: volumen/timing correlaciona mensajes
  con el mismo `thread_id` (mismo caso que fase 4 §5.1).
- `patient_entity_id` es el puente a lo clínico: la SOAP se crea con
  `subject_id` = esa entity desde el Core (flujo normal del profesional).

---

## 4. Cripto: thread keys y ausencia estructural de DataKey

Cadena de descifrado por rol:

```text
Médico:   sesión → DataKey → unwrap wrap_doctor (ctx) → thread key → mensajes
Paciente: password → su vault (archivo propio) → master → unwrap wrap_patient (ctx) → thread key
```

Operaciones **vía `ModuleContext`** (`wrap_with_datakey`/`unwrap_with_datakey`
para oficina; `wrap_for_user`/`unwrap_for_user` para el wrap de paciente) —
el módulo pide wraps, nunca recibe claves. El Core gate-audita cada
operación (v1: `wrap_for_user` solo acepta targets `role: 'paciente'`).

**Por qué el paciente jamás tiene DataKey** (propiedad estructural, sin
lista de permisos):

1. El wrap personal `.data_key.{user_id}` de un paciente **nunca se crea**
   (solo se crea para staff, vía seed o `admin_reset_password`).
2. El paciente no conoce la **seed** del consultorio (bootstrap de
   `resolve_data_key`).
3. → `resolve_data_key` falla con `SEED_REQUIRED`; el flujo de login del
   paciente inyecta solo master + firma y la DataKey **no entra jamás en su
   RAM**.

Efecto cascada: todos los commands clínicos (entities, entries, export)
fallan naturalmente para un paciente — requieren DataKey en RAM. La defensa
es la arquitectura, no el ACL.

**Prerrequisitos de Core (estado)**:

- `unlock_vault` consciente del rol (paciente: solo master + firma) —
  **resuelto (2026-10-11)**: `unlock_vault_core` con
  `UnlockedKeys { data_key: Option<...> }`.
- `change_password` sin rewrap y `admin_reset_password` sin wrap para
  paciente — **resueltos (2026-10-11)**.
- `grant_patient_access` (`users.entity_id`, alta lazy) — **resuelto
  (2026-10-11)**.
- `require_session` en los commands de atribución — **resuelto
  (2026-10-11)**.
- **`wrap_for_user`/`unwrap_for_user` en `ModuleContext`** — **nuevo, P0
  del módulo** (lo que hizo `wrap_with_datakey` decidido en v1 del
  contrato).

Rotación:

- **`change_password` del paciente**: el vault re-envuelve la **misma
  master** → los wraps de thread siguen válidos; no hay que tocar nada.
- **`admin_reset_password`** (olvidó password / robo de dispositivo): el
  vault se regenera con master **nueva** → el admin re-envuelve
  `wrap_patient_cipher` en todos los threads del paciente (blobs LWW, vía
  `wrap_for_user`). **Trust model explícito: el administrador del
  consultorio puede leer el canal del paciente.** Documentado; se mitiga
  con que el reset queda en `audit_log`.

---

## 5. Flujo completo

```text
1. Alta del paciente (una vez, modo médico)
   admin_create_user(role: 'paciente') + password temporal → audit_log.
   El paciente instala la app → login → unlock (su vault se crea en SU
   dispositivo en el primer unlock) → change_password con la suya.
   ⚠ Abrir un thread exige que el paciente ya activó su cuenta (vault
   existe); si no, el módulo responde "el paciente aún no activó".

2. Canal
   Staff/paciente inicia consulta (asunto opcional) → module_execute
   "open_thread" → thread + thread key (wraps vía ModuleContext)
   → mensajes async ambos lados ("send_message": texto o foto).
   El sync de fase 4 (polling 60 s) transporta la cola propia del módulo.

3. Pago (eventual, P2)
   Módulo renderiza QR/alias estático del profesional → paciente paga
   externamente → envía foto del comprobante (voucher_photo)
   → profesional confirma → payments.status = confirmed (LWW)
   → mensaje 'system': "Pago confirmado".

4. Cierre clínico
   El profesional abre el formulario del Core (create_entry normal:
   subject = entity, author = profesional de sesión, firmada, inmutable).
   El módulo NO lo crea ni lo copia; como máximo guarda un estado de
   thread ("nota creada") dentro de enc_data_blob.
```

La SOAP aparece solo en el EHR del paciente (y en su export `.md`): el
módulo no guarda copia ni referencia automática (v1).

---

## 6. Sync (mapeo a fase 4)

| Dato | Mecanismo | Regla |
|---|---|---|
| `messages` | outbox propia del módulo + pull | **Unión por `id`** (inmutables, como entries) |
| `threads` | outbox propia + pull | LWW con `updated` del servidor (blob + `updated_at`) |
| `payments` | outbox propia + pull | LWW |
| `thread_keys` (wraps) | outbox propia + pull | LWW por blob (rotaciones/reset) |
| Nota SOAP | — | **No es dato del módulo**: entra y sync-ea como `entries` por el Core |
| Vault del paciente | pairing (fase 4 §5.3) | Su dispositivo recibe su vault vía pairing normal |

- Colecciones en el relé (mismo PocketBase, device token y pairing de
  fase 4): `sync_plugin_teleconsulta_{messages,threads,payments}` + wraps
  en `files` con namespace `teleconsulta:*`.
- La outbox del módulo se escribe **en la misma transacción** que la
  escritura en las tablas del módulo (mismo archivo); el engine de fase 4
  itera las colas de los módulos activos (`module-contract.md` §13).
- **Orden de bootstrap**: colecciones del módulo **después de `entities`**
  (los threads referencian fichas por `external_id`), al final del orden de
  fase 4 §6.4.
- Reglas de falla heredadas: push con error → pull omitido (§4.1.4); eco
  `origin_device`; cursores por colección; `min_app_version` del Core
  gobierna también las colecciones del módulo.
- **ACL del relé**: el device token de un paciente solo puede pull-ear sus
  colecciones con scope acotado. Server rules por usuario — hito P4.

---

## 7. Seguridad

| Regla | Detalle |
|---|---|
| DataKey ausente para pacientes | Estructural: sin wrap personal y sin seed (§4). Commands clínicos fallan solos |
| El módulo nunca ve claves | Wraps/descifrado vía `ModuleContext` con la bóveda abierta; `wrap_for_user` gate-audita target no-paciente |
| El servidor es tubo ciego | Ciphertexts + UUIDs + timestamps; status/asunto/contenido/pago van cifrados |
| Gate antes que rol de dominio | `module_execute`: descriptor → active → rol ∈ `allowed_roles` → `module_access` (`module-contract.md` §5) |
| Actor solo de la sesión | `require_session` en todos los actions; `sender_user_id` y `entity_id` de pacientes derivados del Core, jamás del payload |
| `fetch_filiation` scoping Core-side | Actor `paciente` solo resuelve su propia entity (garantizado en `ModuleContext`, no en el módulo) |
| Participación en el thread | El módulo valida en `handle`: el actor es `patient_user_id` del thread o staff del consultorio (`module-contract.md` §12) |
| Atribución | Ambos lados: `sender_user_id` desde la sesión; hecho clínico firmado = entry SOAP escrita por el profesional en el Core |
| Reset por admin | Trust model: el admin puede derivar la master nueva y leer el canal; queda en `audit_log` |
| Pago | Flujo externo (banca); el módulo solo rendering de QR estático + registro operativo. Sin datos de tarjeta/cuenta en la app |
| Auditoría | `AuditEvent` cerrado del Core (`TeleconsultaMessageSent`, `TeleconsultaPaymentConfirmed`) — sin PHI |

---

## 8. IPC con el Core

- El módulo corre en el binario (feature flag) y habla **solo** por el
  trait `Module` + `ModuleContext` (`module-contract.md` §3–4), invocado
  desde el frontend con **un único command** `module_execute`.
- **Actions** (strings, no commands tipados): `open_thread`,
  `send_message`, `get_thread`, `list_messages`, `confirm_payment` (P2).
  Ningún `tc_*` se registra en `lib.rs`.
- **Prerrequisitos Core** (§4): todos los P0 de 2026-10-11 resueltos +
  **`wrap_for_user`/`unwrap_for_user`** (nuevo, trabajo del P1 del
  módulo). `create_entry` lo invoca el profesional desde el frontend en el
  flujo Core normal — el módulo no lo necesita.
- Gate (`module-contract.md`): `allowed_roles` del descriptor =
  `[Administrador, Asistente, Profesional, Paciente]`; scope =
  `InstallationAndProfessional` para staff; el `module_access` aplica solo
  a staff (los pacientes pasan por rol + validación de participación).

---

## 9. Hitos

| # | Hito | Contenido | Tamaño |
|---|---|---|---|
| P0 | Prerequisitos Core | **✅ resuelto (2026-10-11)**: `unlock_vault` role-aware, `change_password` sin rewrap, `grant_patient_access`, reset sin wrap para paciente, `require_session` y gates no-clínicos. **+ nuevo**: `wrap_for_user`/`unwrap_for_user` en `ModuleContext` | M |
| P1 | Canal local | DB del módulo (`migrate()` propio), thread keys vía `ModuleContext`, mensajes, dispatch por `module_execute` (paciente simulado con su password) | M |
| P2 | Pago | QR/alias configurable, `voucher_photo`, confirmación manual, mensaje system | S |
| P3 | Cierre SOAP | El profesional escribe la entry en el Core (flujo normal); el módulo solo actualiza estado del thread | S |
| P4 | Sync | Cola propia del módulo + iteração del engine de fase 4 (requiere fase 4 M1–M3); bootstrap después de entities; ACL de relé por usuario | M |
| P5 | Modo paciente | Vista acotada (canal + pago, sin resto del Core), activación de cuenta en primera sesión, pairing. **Ojo**: `App.tsx` muestra `SeedPhraseSetup` a cualquier rol si la seed no está en localStorage — en el teléfono del paciente hay que saltearlo por rol (nunca ve la frase) | M |

**Dependencia**: P0 del contrato (`CoreApi` + trait `Module` + `PluginHost`)
es prerequisite de todo lo demás. P4 requiere fase 4 M1–M3.

---

## 10. Preguntas abiertas

- `note_reference` (mensaje que apunta a la entry del Core): ¿v1 o
  post-v1? v5.0 lo deja fuera — el estado "nota creada" en el thread
  alcanza.
- QR: v1 **estático** (alias/CVU configurable del profesional). ¿Monto
  dinámico en v2?
- Límite de tamaño de `voucher_photo` (¿compresión a WebP en el Core?,
  límite ~5 MB documentado).
- ¿Un solo thread abierto por paciente a la vez? (v1: sí, simplifica).
- Notificaciones: v1 = indicador/polling; v2 = realtime PocketBase.
- Compartir la SOAP / el EHR con el paciente: post-v1 con re-wrap
  por-paciente.
- Multi-dispositivo del paciente: el pairing normal de fase 4 lo cubre;
  validar el orden vault-via-pairing vs. password elegida en el phone.

---

## 11. Delta contra versiones anteriores

| v3.0 | v4.0 | v5.0 (actual) |
|---|---|---|
| Tablas en la DB del Core (`plugin_teleconsulta_*`) | igual v3 | **DB del módulo** `fichio-teleconsulta.db`, tablas sin prefijo, outbox propia (contrato de módulos) |
| Outbox `sync_queue` del Core | igual v3 | Outbox propia del módulo; engine de fase 4 itera módulos |
| `create_entry` vía plugin (`tc_create_sap_note`) | igual v3 | **El profesional escribe la SOAP en el Core a mano**; sin `note_reference` automático (pregunta abierta) |
| Trait `Plugin` + misma `Connection` (pre-contracto, v3/v4) | igual v3 | Trait `Module` + `ModuleContext` + `module_execute` (`module-contract.md`) |
| Identidad del paciente: ad-hoc / `patient_creds` | Usuario `role: 'paciente'` | igual v4 (P0 resuelto) |
| Sync propia del plugin | Primitivas de fase 4 | Cola propia del módulo + iteração fase 4 |
| Sin pago | Workflow de pago QR/manual | igual v4 (hitp P2), datos en la DB del módulo |
