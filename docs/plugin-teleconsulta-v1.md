# 🧩 Plugin: Teleconsulta Asíncrona

## Simplex Health Core — Arquitectura de Plugins
### Versión 4.0 — 2026-10-11 | Paciente como usuario; alcance mínimo; realineado con fase4 v1.1

> **v4.0 sustituye a v3.0** (mismo día): el paciente es un **usuario real**
> con `role: 'paciente'` — el rol existe en el CHECK de `users` desde el
> v001, que era el plan original; el diseño de julio (PatientDataKey) y el
> v3 (`patient_creds`) se desviaron con sistemas paralelos de identidad.
> Se elimina `patient_creds`; la DataKey está **estructuralmente ausente**
> para el rol. v3.0 y v2.0 quedan en el historial de git.

---

## 1. Visión y alcance

Un canal de comunicación **único y asíncrono** entre el profesional y un
paciente (usuario con `role: 'paciente'`), con workflow de cobro eventual y
cierre clínico como entry SOAP.

| En alcance (v1) | Fuera de alcance |
|---|---|
| Un thread de conversación por paciente | Video / llamadas en tiempo real |
| Mensajes de texto + foto de comprobante | Multi-thread, bitácora PGHD, signos vitales |
| Pago eventual: QR/transferencia con **confirmación manual** | Pasarela de pago, webhooks, integraciones bancarias |
| Cierre: SOAP como **entry del Core** (firmada, inmutable) | Compartir la SOAP u otros datos clínicos con el paciente (re-wrap por paciente — post-v1) |
| Usuario paciente: vault propio, password propia, pairing propio | Paciente viendo su propio EHR (depende del re-wrap, post-v1) |
| Sync de canal/pago por las primitivas de fase4 | Realtime (suscripciones PocketBase — v2 del plugin) |

**Principio rector:** el plugin es *operativa*; lo clínico es *Core*. El
historial del paciente (incluida la nota SOAP de la teleconsulta) nunca se
duplica en datos del plugin — el thread solo la **referencia**. Y el rol
`paciente` **nunca** tiene DataKey (§4): esa separación es estructural, no
una lista de permisos. Además el paciente **no ve el EHR ni las entries**
del Core — lo clínico que percibe le llega solo por el thread
(`note_reference`).

---

## 2. Qué vive dónde

| Pieza | Hogar | Por qué |
|---|---|---|
| `threads`, `messages`, `payments` | Tablas namespaced en la **DB del Core** (`plugin_teleconsulta_*`) | Outbox atómica real (mismo archivo, misma transacción que `sync_queue`), backup/restore del Core las cubre, crypto y sync vía Core. Regla general: **datasets de referencia → DB propia del plugin; datos clínicos/operativos del plugin → DB del Core** |
| Usuario paciente | `users` (rol `'paciente'`, ya en el CHECK de v001) | Identidad uniforme: vault, login, sesión, `require_session`, `change_password`, pairing, audit — todo lo construido y testeado se reutiliza |
| Vínculo ficha ↔ usuario | `users.entity_id` (FK `entities`, NULL en staff) — **Core** | Creado por `grant_patient_access(entity_id, …)` desde la ficha (**alta lazy, opción B**): sin usuarios fantasma para pacientes que nunca entran. Gate admin+asistente; temp password (v1) |
| Nota SOAP | `entries` del Core (`create_entry`) | Firma Ed25519, inmutabilidad, EHR, export `.md`, verificación, sync por unión — todo gratis |
| Llaves de thread | Tabla en la DB del Core | Envueltas con DataKey (médico) y con la master de la vault del paciente; blobs LWW |
| Datasets de referencia (futuro: SNOMED…) | DB propia del plugin (`data/*.db`) | No es sync: artefactos versionados |
| UI | Componentes del plugin | Modo médico (Core + plugin); modo paciente (vista acotada) |

El plugin **no corre su propio sync engine** ni toca `sync_queue`
directamente: sus commandos (trait `Plugin`, mismo proceso) escriben la tabla
del plugin y su evento de outbox **en la misma transacción**, igual que todo
command del Core.

El Core **no lleva placeholders** de features de plugin: la vista y la
entrada de menú de teleconsulta las aporta el plugin (2026-10-11 se
eliminaron las que vivían en `PacienteView`/`Drawer`; el Core del paciente
es ahora perfil + contraseña, funcional y mínimo).

---

## 3. Modelo de datos (DB del Core)

```sql
-- Un canal por paciente. Mutables → LWW (blob cifrado + updated_at).
CREATE TABLE plugin_teleconsulta_threads (
    id              TEXT PRIMARY KEY,            -- UUID
    patient_user_id TEXT NOT NULL,               -- FK users(id), role 'paciente': contraparte del canal
    patient_entity_id INTEGER,                   -- FK entities(id): sujeto clínico del SOAP (al abrir el thread: users.entity_id del patient_user_id; "linkeo tardío" solo como fallback)
    assigned_to_user_id TEXT,                    -- FK users(id) staff: profesional a cargo
    enc_data_blob   BLOB NOT NULL,               -- status, subject (cifrados, DataKey)
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,               -- toca en cada cambio (LWW)
    closed_at       TEXT,
    FOREIGN KEY(patient_user_id) REFERENCES users(id),
    FOREIGN KEY(patient_entity_id) REFERENCES entities(id),
    FOREIGN KEY(assigned_to_user_id) REFERENCES users(id)
);

-- Mensajes: hechos inmutables → unión por id (como entries).
CREATE TABLE plugin_teleconsulta_messages (
    id              TEXT PRIMARY KEY,            -- UUID
    thread_id       TEXT NOT NULL,
    sender_type     TEXT NOT NULL CHECK(sender_type IN ('patient', 'doctor', 'system')),
    sender_user_id  TEXT NOT NULL,               -- desde require_session (ambos lados)
    kind            TEXT NOT NULL CHECK(kind IN ('text', 'voucher_photo', 'note_reference')),
    content_ciphertext BLOB NOT NULL,            -- con thread key
    content_nonce   BLOB NOT NULL,
    note_external_id TEXT,                       -- entries.id cuando kind = 'note_reference'
    created_at      TEXT NOT NULL,
    FOREIGN KEY(thread_id) REFERENCES plugin_teleconsulta_threads(id)
);

-- Pago: dato OPERATIVO (no clínico). LWW. Paciente se entera por
-- mensaje 'system' en el canal, no leyendo esta tabla.
CREATE TABLE plugin_teleconsulta_payments (
    id              TEXT PRIMARY KEY,
    thread_id       TEXT NOT NULL,
    enc_data_blob   BLOB NOT NULL,               -- amount_ars, method, status, reference (DataKey)
    confirmed_by_user_id TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,               -- LWW
    FOREIGN KEY(thread_id) REFERENCES plugin_teleconsulta_threads(id)
);

-- Llave de conversación por thread, envuelta dos veces.
CREATE TABLE plugin_teleconsulta_thread_keys (
    thread_id            TEXT PRIMARY KEY,
    wrap_doctor_cipher   BLOB NOT NULL,          -- thread key con DataKey (oficina)
    wrap_doctor_nonce    BLOB NOT NULL,
    wrap_patient_cipher  BLOB NOT NULL,          -- thread key con master de la vault del paciente
    wrap_patient_nonce   BLOB NOT NULL,
    created_at           TEXT NOT NULL,
    FOREIGN KEY(thread_id) REFERENCES plugin_teleconsulta_threads(id)
);
-- No hay patient_creds: la vault del paciente ES su credencial.
```

Notas:

- `status` del thread (`open`, `waiting_payment`, `in_consultation`,
  `note_created`, `closed`) va **dentro de `enc_data_blob`**: el servidor no
  ve ni el estado del flujo ni el asunto de la consulta.
- El servidor ve solo: UUIDs, timestamps, `updated` (LWW), ciphertexts y
  `origin_device`. Residual aceptado: el volumen/timing correlaciona
  mensajes con el mismo `thread_id` (mismo caso que fase4 §5.1).
- `patient_entity_id` es el puente a lo clínico: el SOAP se crea con
  `subject_id` = esa entity; si el paciente no tiene ficha, se crea/linkea
  al momento de cerrar la nota.

---

## 4. Cripto: thread keys y ausencia estructural de DataKey

Cadena de descifrado por rol:

```text
Médico:   sesión → DataKey → unwrap wrap_doctor → thread key → mensajes
Paciente: password → su vault (archivo propio) → master → unwrap wrap_patient → thread key
```

**Por qué el paciente jamás tiene DataKey** (propiedad estructural, sin
lista de permisos):

1. El wrap personal `.data_key.{user_id}` de un paciente **nunca se crea**
   (solo se crea para staff, vía seed o `admin_reset_password`).
2. El paciente no conoce la **seed** del consultorio (bootstrap de
   `resolve_data_key`).
3. → `resolve_data_key` falla con `SEED_REQUIRED`; si el flujo de login lo
   permite, inyecta solo master + firma (ver abajo) y la DataKey **no entra
   jamás en su RAM**.

Efecto cascada: todos los commands clínicos (entities, entries, export)
fallan naturalmente para un paciente — requieren DataKey en RAM. La defensa
es la arquitectura, no el ACL.

**Requisitos de Core (prerequisitos del plugin):**

- `unlock_vault` consciente del rol: para `role: 'paciente'`, inyectar solo
  master + keypair de firma y **saltear** `resolve_data_key` y el sync con
  `professional_profiles` (no es staff). **Resuelto (2026-10-11)**:
  `unlock_vault_core` con `UnlockedKeys { data_key: Option<...> }` — el
  paciente no puede siquiera pedir la DataKey. `change_password` idem
  (sin rewrap para paciente; `user_id` desde la sesión) y
  `admin_reset_password` no crea wrap para pacientes (ausencia también en
  disco).
- `require_staff()` para commands **sin** DataKey que no deben ser del
  paciente: `backup_*`, `list_audit_log`, `admin_*`, `list_users` — esos no
  fallan solos por falta de DataKey. **Ya cubierto (verificado 2026-10-11)**:
  todos esos commands ya exigen `require_admin` (estricto ≥ staff).
- `require_session` en `create_entity` / `update_entity` / `create_entry`:
  **resuelto (2026-10-11)** — autor/creador desde la sesión, nunca del
  frontend (fase4 checklist §9 marcado). Esto subió la prioridad de la
  deuda M1: con pacientes adentro de `users`, un command con sola sesión
  era un vector real — ya cerrado.

Rotación:

- **`change_password` del paciente**: el vault re-envuelve la **misma
  master** → los wraps de thread siguen validos; no hay que tocar nada.
- **`admin_reset_password`** (olvidó password / robo de dispositivo): el
  vault se regenera con master **nueva** → el admin (que conoce la password
  temporal nueva y puede derivar la master) re-envuelve
  `wrap_patient_cipher` en todos los threads del paciente (blobs LWW).
  **Trust model explícito: el administrador del consultorio puede leer el
  canal del paciente.** Es el modelo de cualquier portal clínico y queda
  documentado; se mitiga con que el reset queda en `audit_log`.

El plugin **nunca ve claves**: generación de thread keys, wraps y
descifrado son operaciones de Core IPC con la bóveda abierta.

---

## 5. Flujo completo

```text
1. Alta del paciente (una vez, modo médico)
   admin_create_user(role: 'paciente') + password temporal → audit_log.
   El paciente instala la app → login → unlock (su vault se crea en SU
   dispositivo en el primer unlock) → change_password con la suya.
   ⚠ Abrir un thread exige que el paciente ya activó su cuenta (vault
   existe); si no, el Core responde "el paciente aún no activó".

2. Canal
   Staff/paciente inicia consulta (asunto opcional) → thread + thread key
   (wraps: DataKey para oficina, master del paciente para el paciente)
   → mensajes async ambos lados (texto o foto de comprobante).
   El sync de fase4 (polling 60 s) transporta el canal: sin realtime en v1.

3. Pago (eventual)
   Plugin renderiza QR/alias estático del profesional → paciente paga
   externamente (app bancaria) → envía foto del comprobante (voucher_photo)
   → profesional confirma → payments.status = confirmed (LWW)
   → mensaje 'system': "Pago confirmado".

4. Cierre clínico
   Se linkea patient_entity_id (o se crea la ficha del paciente)
   → profesional → "Crear nota SOAP" (formulario del Core) → create_entry
   (subject = entity, author = profesional de sesión, firmada, inmutable)
   → plugin encola mensaje note_reference con note_external_id
   → thread.status = note_created / closed.
```

El SOAP aparece solo en el EHR del paciente (y en su export `.md`): el
plugin no guarda una copia.

---

## 6. Sync (mapeo a fase4 v1.1)

| Dato | Mecanismo | Regla |
|---|---|---|
| `messages` | outbox + pull | **Unión por `id`** (inmutables, como entries) |
| `threads` | outbox + pull | LWW con `updated` del servidor (blob + `updated_at`) |
| `payments` | outbox + pull | LWW |
| `thread_keys` (wraps) | outbox + pull | LWW por blob (rotaciones/reset) |
| Nota SOAP | — | **No es dato del plugin**: entra y sync-ea como `entries` por el Core |
| Vault del paciente | pairing (fase4 §5.3) | Su dispositivo recibe su vault vía pairing normal, como cualquier usuario |

- Colecciones en el relé (mismo PocketBase, device token y pairing de
  fase4): `sync_plugin_teleconsulta_{messages,threads,payments}` + los
  wraps en la colección `files` con namespace `teleconsulta:*`.
- **Orden de bootstrap**: las colecciones del plugin se pull-ean
  **después de `entities`** (los threads referencian fichas por
  `external_id`), al final del orden de fase4 §6.4.
- Reglas de falla heredadas: push con error → pull omitido (§4.1.4); eco
  `origin_device`; cursores por colección; `min_app_version` del Core
  gobierna también a las colecciones del plugin.
- **ACL del relé**: el device token de un paciente solo puede pull-ear
  sus colecciones con scope acotado (sus threads/mensajes/pagos). Server
  rules por usuario — trabajo del hito P4.

---

## 7. Seguridad

| Regla | Detalle |
|---|---|
| DataKey ausente para pacientes | Estructural: sin wrap personal y sin seed (§4). Commands clínicos fallan solos |
| El plugin nunca ve claves | Todo descifrado/cifrado vía Core IPC con la bóveda abierta (contrato del microkernel) |
| El servidor es tubo ciego | Ciphertexts + UUIDs + timestamps; status/asunto/contenido/pago van cifrados |
| `require_admin` en lo no-clínico | `backup_*`, `admin_*`, `list_audit_log`, `list_users` ya exigen `require_admin` (verificado 2026-10-11) |
| `tc_*` participante-scoped | `require_session` + "¿este user_id es `patient_user_id` del thread o el staff asignado/activo del consultorio?" |
| Atribución | Ambos lados: `sender_user_id` desde `require_session`; hecho clínico firmado = entry SOAP del profesional |
| Reset por admin | Trust model: el admin puede derivar la master nueva y leer el canal; queda en `audit_log` |
| Pago | Flujo externo (banca); el plugin solo rendering de QR estático + registro operativo. Sin datos de tarjeta/cuenta en la app |

---

## 8. IPC con el Core

- El plugin corre en el mismo proceso (trait `Plugin` de
  `PLUGIN_ARCHITECTURE.md`) y usa la misma `Connection` que el Core — por
  eso la outbox es atómica.
- **Prerequisitos Core** (§4): **resueltos todos en P0 (2026-10-11)** —
  `unlock_vault` role-aware, `change_password` sin rewrap,
  `grant_patient_access` + `get_entity_access` (gate admin/asistente),
  reset de contraseña sin wrap para paciente; más `require_session` en los
  3 commands de atribución y gates no-clínicos (`require_admin`).
- Comandos Core que el plugin necesita (ya existentes o casi):
  `create_entry` (SOAP), `get_entity`/`decrypt` (ficha del paciente),
  `require_session`, `admin_create_user` (ya valida `role: 'paciente'` en
  `VALID_ROLES`), API de sync de fase4 (encolar/aplicar colecciones
  declaradas).
- Comandos del plugin (nuevos, registrados en `lib.rs` como todo command):
  `tc_open_thread`, `tc_send_message`, `tc_confirm_payment`,
  `tc_create_sap_note` (atajo al formulario Core) — todos con check de
  participación del thread (§7).

---

## 9. Hitos

| # | Hito | Contenido | Tamaño |
|---|---|---|---|
| P0 | Prerequisitos Core | **✅ resuelto (2026-10-11)**: `unlock_vault` role-aware (paciente: sin resolve_data_key, sin professional_profiles), `change_password` sin rewrap de data_key, **`grant_patient_access`** (`users.entity_id`, alta lazy desde la ficha), reset de contraseña sin wrap para paciente, vista paciente = username + contraseña (sin entries/EHR); `require_session` y gates no-clínicos ya resueltos | M |
| P1 | Canal local | Tablas, thread keys, mensajes con Core IPC, sin sync (paciente simulado con su password) | M |
| P2 | Pago | QR/alias configurable, `voucher_photo`, confirmación manual, mensaje system | S |
| P3 | Cierre SOAP | Linkeo de entity, atajo al formulario del Core, `note_reference`, cierre de thread | S |
| P4 | Sync | Colecciones vía API de fase4 (requiere fase4 M1–M3); bootstrap después de entities; ACL de relé por usuario | M |
| P5 | Modo paciente | Vista acotada (canal + pago, sin resto del Core), activación de cuenta en primera sesión, pairing. **Ojo**: `App.tsx` muestra `SeedPhraseSetup` a cualquier rol si la seed no está en localStorage del dispositivo — en el teléfono del paciente hay que saltearlo por rol (nunca ve la frase) | M |

**Dependencia**: P0 es prerequisito de todo lo demás (se puede landear
independientemente de fase4). P4 requiere fase4 M1–M3.

---

## 10. Preguntas abiertas

- QR: v1 **estático** (alias/CVU configurable del profesional). ¿Monto
  dinámico en v2 (requiere integración o al menos QR por importe)?
- Límite de tamaño de `voucher_photo` (¿compresión a WebP en el Core?,
  límite ~5 MB documentado).
- ¿Un solo thread abierto por paciente a la vez? (v1: sí, simplifica).
- Notificaciones: v1 = indicador/polling; v2 = realtime PocketBase.
- Compartir la SOAP / el EHR con el paciente: post-v1 con re-wrap
  por-paciente (hereda el diseño `shared_notes` de v2.0, apoyado ahora en
  un usuario real).
- Multi-dispositivo del paciente: el pairing normal de fase4 lo cubre;
  validar el orden vault-vía-pairing vs. password elegida en el phone.

---

## 11. Delta contra versiones anteriores

| v2.0 (julio) | v3.0 | v4.0 (actual) |
|---|---|---|
| PatientDataKey completo (vault, `patient_keys`, `shared_notes`) | Thread keys + `patient_creds` paralelo | Thread keys + **usuario `role: 'paciente'`** (vault propia); `patient_creds` eliminado; PatientDataKey/re-wrap post-v1 |
| `teleconsulta.db` propia | Tablas en la DB del Core | igual v3 |
| `is_synced` por tabla | Outbox `sync_queue` (fase4 §3.4) | igual v3 |
| `create_note` (pre-entries) | `create_entry` (entries firmadas) | igual v3 |
| Compartir nota con el paciente en v1 | Fuera de alcance v1 | Fuera de alcance v1 (re-wrap post-v1) |
| Sin pago | Workflow de pago QR/transferencia manual | igual v3 |
| Sync propia del plugin | Primitivas de fase4 v1.1 | igual v3 + ACL de relé por usuario |
| Identidad del paciente: ad-hoc | `patient_creds` (kdf_salt + wrap) | Usuario real desde `admin_create_user`; DataKey ausente estructuralmente; reset por admin con trust model documentado |
