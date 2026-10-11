# 🧩 Plugin: Teleconsulta Asíncrona

## Simplex Health Core — Arquitectura de Plugins
### Versión 3.0 — 2026-10-11 | Alcance mínimo, realineado con fase4 v1.1

> **v3.0 sustituye a v2.0** (2026-07-09, PatientDataKey). Cambios: alcance
> reducido a canal único + workflow de pago + atajo a entry SOAP; los datos
> clínicos viven **100% en el Core**; sync por las primitivas de fase4 v1.1
> (outbox, unión inmutable, LWW con reloj de servidor); thread keys en vez
> de la jerarquía completa PatientDataKey. v2.0 queda en el historial de git.

---

## 1. Visión y alcance

Un canal de comunicación **único y asíncrono** entre el profesional y un
paciente, con workflow de cobro eventual y cierre clínico como entry SOAP.

| En alcance (v1) | Fuera de alcance |
|---|---|
| Un thread de conversación por paciente | Video / llamadas en tiempo real |
| Mensajes de texto + foto de comprobante | Multi-thread, bitácora PGHD, signos vitales |
| Pago eventual: QR/transferencia con **confirmación manual** | Pasarela de pago, webhooks, integraciones bancarias |
| Cierre: SOAP como **entry del Core** (firmada, inmutable) | Compartir la nota SOAP con el paciente (requiere PatientDataKey — post-v1) |
| Sync de canal/pago por las primitivas de fase4 | Realtime (suscripciones PocketBase — v2 del plugin) |
| Credencial mínima del paciente (password → wrap) | Vault completo del paciente / identidad clínica autónoma |

**Principio rector:** el plugin es *operativa*; lo clínico es *Core*. El
historial del paciente (incluida la nota SOAP de la teleconsulta) nunca se
duplica en datos del plugin — el thread solo la **referencia**.

---

## 2. Qué vive dónde

| Pieza | Hogar | Por qué |
|---|---|---|
| `threads`, `messages`, `payments` | Tablas namespaced en la **DB del Core** (`plugin_teleconsulta_*`) | Outbox atómica real (mismo archivo, misma transacción que `sync_queue`), backup/restore del Core las cubre, crypto y sync vía Core. Regla general: **datasets de referencia → DB propia del plugin; datos clínicos/operativos del plugin → DB del Core** |
| Nota SOAP | `entries` del Core (`create_entry`) | Firma Ed25519, inmutabilidad, EHR, export `.md`, verificación, sync por unión — todo gratis |
| Llaves de thread | Tabla en la DB del Core | Envueltas con DataKey (médico) y con la clave del paciente; blobs LWW |
| Credencial del paciente | Tabla en la DB del Core | Wrap mínimo sin vault completo |
| Datasets de referencia (futuro: SNOMED…) | DB propia del plugin (`data/*.db`) | No es sync: artefactos versionados |
| UI | Componentes del plugin | Modo médico (Core + plugin); modo paciente (vista acotada) |

El plugin **no corre su propio sync engine** ni toca `sync_queue`
directamente: sus commandos (trait `Plugin`, mismo proceso) escriben la tabla
del plugin y su evento de outbox **en la misma transacción**, igual que todo
command del Core.

---

## 3. Modelo de datos (DB del Core)

```sql
-- Un canal por paciente. Mutables → LWW (blob cifrado + updated_at).
CREATE TABLE plugin_teleconsulta_threads (
    id              TEXT PRIMARY KEY,            -- UUID
    patient_entity_id INTEGER NOT NULL,          -- FK entities(id)
    assigned_to_user_id TEXT,                    -- FK users(id); desde sesión
    enc_data_blob   BLOB NOT NULL,               -- status, subject (cifrados, DataKey)
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,               -- toca en cada cambio (LWW)
    closed_at       TEXT,
    FOREIGN KEY(patient_entity_id) REFERENCES entities(id),
    FOREIGN KEY(assigned_to_user_id) REFERENCES users(id)
);

-- Mensajes: hechos inmutables → unión por id (como entries).
CREATE TABLE plugin_teleconsulta_messages (
    id              TEXT PRIMARY KEY,            -- UUID
    thread_id       TEXT NOT NULL,
    sender_type     TEXT NOT NULL CHECK(sender_type IN ('patient', 'doctor', 'system')),
    sender_user_id  TEXT NOT NULL,               -- doctor: desde require_session; patient: token del dispositivo
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
    wrap_patient_cipher  BLOB NOT NULL,          -- thread key con patient_master
    wrap_patient_nonce   BLOB NOT NULL,
    created_at           TEXT NOT NULL,
    FOREIGN KEY(thread_id) REFERENCES plugin_teleconsulta_threads(id)
);

-- Credencial del paciente: suficiente para derivar patient_master y
-- abrir sus wraps. Sin vault, sin llaves de firma (v1).
CREATE TABLE plugin_teleconsulta_patient_creds (
    patient_entity_id INTEGER PRIMARY KEY,
    kdf_salt       BLOB NOT NULL,                -- Argon2id(password) → patient_master
    kdf_params     TEXT NOT NULL,                -- versionado de parámetros
    created_at     TEXT NOT NULL,
    FOREIGN KEY(patient_entity_id) REFERENCES entities(id)
);
```

Notas:

- `status` del thread (`open`, `waiting_payment`, `in_consultation`,
  `note_created`, `closed`) va **dentro de `enc_data_blob`**: el servidor no
  ve ni el estado del flujo ni el asunto de la consulta.
- El servidor ve solo: UUIDs, timestamps, `updated` (LWW), ciphertexts y
  `origin_device`. Residual aceptado: el volumen/timing correlaciona
  mensajes con el mismo `thread_id` (mismo caso que fase4 §5.1).

---

## 4. Cripto: thread keys y credencial del paciente

Cadena de descifrado por rol:

```text
Médico:  sesión → DataKey → unwrap wrap_doctor → thread key → mensajes
Paciente: password + kdf_salt → Argon2id → patient_master
          → unwrap wrap_patient → thread key → mensajes
```

- **Generación**: al abrir el thread, el Core genera la thread key (32 B),
  cifra los mensajes con AES-GCM y escribe los dos wraps (operaciones con
  la bóveda abierta, vía Core IPC — el plugin nunca ve claves).
- **Rotación de password del paciente**: re-wrap de `wrap_patient` en todos
  sus threads (blobs LWW, mismo patrón que los vaults de fase4). No se
  re-cifran los mensajes.
- **Rotación de DataKey del consultorio**: re-wrap de `wrap_doctor`
  (idéntico al flujo de wraps de fase4).
- **Mensajes sin firma Ed25519, por diseño**: el canal es workflow, no
  registro clínico. El hecho clínico atribuible y firmado es la entry SOAP
  (`create_entry`, `author_id` desde `require_session`). Atribución del
  canal: `sender_user_id` de la sesión (doctor) o del device token (patient).
- **PatientDataKey completo** (vault del paciente, `patient_keys`,
  compartir notas históricas) queda como evolución post-v1 — las thread
  keys son suficientes para el canal.

---

## 5. Flujo completo

```text
1. Dar acceso (una vez, desde el modo médico)
   Profesional → "Dar acceso al paciente" → código de uso único + expiración
   → paciente instala / abre modo paciente → ingresa código + elige password
   → patient_master; server registra el device token acotado a su entity.

2. Canal
   Paciente inicia consulta (asunto opcional) → thread + thread key
   → mensajes async ambos lados (texto o foto de comprobante).
   El sync de fase4 (polling 60 s) transporta el canal: sin realtime en v1.

3. Pago (eventual)
   Plugin renderiza QR/alias estático del profesional → paciente paga
   externamente (app bancaria) → envía foto del comprobante (voucher_photo)
   → profesional confirma → payments.status = confirmed (LWW)
   → mensaje 'system': "Pago confirmado".

4. Cierre clínico
   Profesional → "Crear nota SOAP" (formulario del Core) → create_entry
   (subject = entity del paciente, firmada, inmutable)
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
| `thread_keys` (wraps) | outbox + pull | LWW por blob (rotaciones) |
| `patient_creds` | outbox + pull | LWW por blob |
| Nota SOAP | — | **No es dato del plugin**: entra y sync-ea como `entries` por el Core |

- Colecciones en el relé (mismo PocketBase, device token y pairing de
  fase4): `sync_plugin_teleconsulta_{messages,threads,payments}` + los
  wraps en la colección `files` con namespace `teleconsulta:*`.
- **Orden de bootstrap**: las colecciones del plugin se pull-ean
  **después de `entities`** (los threads referencian pacientes por
  `external_id`), al final del orden de fase4 §6.4.
- Reglas de falla heredadas: push con error → pull omitido (§4.1.4);
  eco `origin_device`; cursores por colección; `min_app_version` del Core
  gobierna también a las colecciones del plugin.

---

## 7. Seguridad

| Regla | Detalle |
|---|---|
| El plugin nunca ve claves | Todo descifrado/cifrado vía Core IPC con la bóveda abierta (contrato del microkernel) |
| El servidor es tubo ciego | Ciphertexts + UUIDs + timestamps; status/asunto/contenido/pago van cifrados |
| Credencial del paciente = wrap chico | Password del paciente solo abre thread keys; **no** da DataKey, no da otras entidades, no da notas del EHR |
| Código de acceso del paciente | Uso único + expiración + rate-limit (mismo perfil que el pairing de dispositivos de fase4 §5.3) |
| Atribución | Doctor: `require_session`; paciente: device token acotado a su `patient_entity_id`; hecho clínico firmado = entry SOAP |
| Pago | Flujo externo (banca); el plugin solo rendering de QR estático + registro operativo. Sin datos de tarjeta/cuenta en la app |

---

## 8. IPC con el Core

- El plugin corre en el mismo proceso (trait `Plugin` de
  `PLUGIN_ARCHITECTURE.md`) y usa la misma `Connection` que el Core — por
  eso la outbox es atómica.
- Comandos Core que el plugin necesita (ya existentes o casi):
  `create_entry` (SOAP), `get_entity`/`decrypt` (paciente del thread),
  `require_session` (atribución), API de sync de fase4 (encolar/aplicar
  colecciones declaradas).
- Comandos del plugin (nuevos, registrados en `lib.rs` como todo command):
  `tc_open_thread`, `tc_send_message`, `tc_confirm_payment`,
  `tc_create_sap_note` (atajo al formulario Core), `tc_grant_patient_access`.

---

## 9. Hitos

| # | Hito | Contenido | Tamaño |
|---|---|---|---|
| P1 | Canal local | Tablas, thread keys, mensajes con Core IPC, sin sync ni paciente real (paciente simulado con password) | M |
| P2 | Pago | QR/alias configurable, `voucher_photo`, confirmación manual, mensaje system | S |
| P3 | Cierre SOAP | Atajo al formulario del Core, `note_reference`, cierre de thread | S |
| P4 | Sync | Colecciones vía API de fase4 (requiere fase4 M1–M3); bootstrap después de entities | M |
| P5 | Paciente real | Código de acceso, modo paciente, device token acotado, re-wrap por rotación | M |

**Dependencia**: P4 requiere fase4 M1–M3 (outbox + engine). P1–P3 y P5 son
independientes del servidor.

---

## 10. Preguntas abiertas

- QR: v1 **estático** (alias/CVU configurable del profesional). ¿Monto
  dinámico en v2 (requiere integración o al menos QR por importe)?
- Límite de tamaño de `voucher_photo` (¿compresión a WebP en el Core?,
  límite ~5 MB documentado).
- ¿Un solo thread abierto por paciente a la vez? (v1: sí, simplifica).
- Notificaciones: v1 = indicador/polling; v2 = realtime PocketBase.
- Compartir la SOAP con el paciente: post-v1 con PatientDataKey
  (`shared_notes` de v2.0, se reutiliza ese diseño).

---

## 11. Delta contra v2.0 (julio 2026)

| v2.0 | v3.0 |
|---|---|
| PatientDataKey completo (vault, `patient_keys`, `shared_notes`) | Thread keys + wrap mínimo del paciente; PatientDataKey post-v1 |
| `teleconsulta.db` propia del plugin | Tablas namespaced en la DB del Core |
| `is_synced` por tabla | Outbox `sync_queue` (fase4 §3.4) |
| `create_note` (pre-entries) | `create_entry` (entries firmadas) |
| Compartir nota con el paciente en v1 | Fuera de alcance v1 |
| Sin pago | Workflow de pago QR/transferencia con confirmación manual |
| Sync propia del plugin | Primitivas de fase4 v1.1 (unión/LWW/relay mismo) |
