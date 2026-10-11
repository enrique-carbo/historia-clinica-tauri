# Fase 4 — Diseño de Sincronización: Desktop + Servidor

**Proyecto:** Simplex Health Core
**Versión:** 1.0 — 2026-10-03
**Estado:** Diseño aprobado (decisiones §1). Sustituye a `docs/2026-julio8-hoja-de-ruta-fase-4-POSIBLE.md` como fuente vigente; ese documento se conserva como histórico.

---

## 0. Contexto

Requisito de producto: un policonsultorio con varios profesionales, **cada uno en su propio equipo**, viendo las mismas evoluciones de los pacientes, con atribución de autor intacta (cada entry firmada por quien la creó).

La v1 local (una estación de trabajo, uso por turnos) ya funciona y es la base. Este diseño la extiende a **múltiples dispositivos de la misma instalación**: cada equipo corre la app Tauri con su propia copia local de la base cifrada, y un servidor PocketBase actúa como relé de datos cifrados.

Principios heredados que se mantienen:

1. **Offline por defecto, sync como optimización.** La app funciona 100% sin red; el sync es background y no bloqueante.
2. **El servidor es un tubo ciego.** Nunca descifra. Solo almacena blobs cifrados, claves públicas, blind indexes y metadatos de sync.
3. **Entries inmutables.** Los hechos clínicos firmados no se editan ni se borran: el sync de entries es unión de conjuntos, sin resolución de conflictos.
4. **Un consultorio = una unidad de confianza.** Todos los dispositivos del mismo consultorio comparten la misma DataKey y los mismos datos.

---

## 1. Decisiones de arquitectura (aprobadas)

| # | Tema | Elección |
|---|------|----------|
| D1 | Hosting | **Híbrido desde el inicio**: perfil *LAN* (Docker Compose en la PC siempre encendida del consultorio, sin exposición pública) y perfil *VPS* (Traefik + TLS + Let's Encrypt, tipo hoja de ruta de julio). URL del servidor configurable en la app; el mismo binario sirve para ambos. |
| D2 | Vault multi-dispositivo | **Sync cifrado de vaults y wraps**: `vault_*.{bin,salt}` y `.data_key.{user}` viajan al servidor como blobs cifrados. El profesional entra con su mismo password en cualquier equipo y usa **el mismo par de llaves Ed25519** → atribución correcta. La frase semilla queda **exclusivamente para break-glass** y nunca viaja por la red. |
| D3 | Cuentas y roles | **Cuentas sincronizadas**: `users` + `professional_profiles` + `signing_keys` se replican en todos los dispositivos. Mismo login en todas partes; el administrador gestiona altas, bajas y roles desde cualquier equipo y se propaga. |
| D4 | Transporte (v1) | **Polling cada 60 s** + pull al iniciar sesión + botón "Sincronizar ahora". Realtime (suscripciones PocketBase) queda como extensión v2. |

### 1.1 Qué cambia respecto a la hoja de ruta de julio

- **D2 reemplaza el principio 5 de julio** ("el vault del médico es su llave física, el sistema no lo distribuye"). La propiedad de seguridad relevante se preserva: sin el password del usuario, el blob del vault almacenado en el servidor es inútil (Argon2id). El vault pasa de *copia manual por USB* a *réplica cifrada automática*.
- El onboarding de julio (código de 6 caracteres + password del admin para abrir el blob de secrets) se simplifica: el código solo **empareja el dispositivo** ante el servidor; los secrets llegan en el pull ordinario y cada uno se abre con la cadena de desbloqueo que le corresponde (password del usuario; semilla solo en break-glass).
- La identidad cross-install (usuarios, perfiles, llaves de firma) **no existía en julio** (llegó con la fase de roles) y es requisito nuevo: sin `signing_keys` sincronizadas, un equipo no puede verificar las firmas del otro.

---

## 2. Delta con la hoja de ruta de julio

| Ítem de julio | Estado hoy | Acción |
|---|---|---|
| `notes` + `previous_note_id` + soft delete | **Obsoleto**: el esquema real es `entries` (id TEXT UUID, append-only, sin cadena de corrección) | Sin acción. Corrección con referencia (`previous_entry_id`) es una posible v003 futura; el sync no la necesita. |
| 3.5.1 `.data_key` cifrado con password del admin | **Resuelto de otra forma**: `.data_key.master` (semilla) + `.data_key.{user}` (password) | Sin acción |
| 3.5.2 `get_notes_by_entity` lineal | **Obsoleto** con entries inmutables | Sin acción |
| 3.5.3 Alimentar `sync_queue` desde todos los comandos de escritura | **Pendiente**: la tabla existe pero ningún comando la escribe | **Pre-requisito M1** |
| 3.5.4 Índices faltantes | `idx_entries_*` ya existen; `blind_index UNIQUE` ya indexa; `entity_id` de notes aplica a notes | Sin acción |
| 3.5.5 Múltiples blind indexes (dni, teléfono…) | **Pendiente**, no bloquea v1 (la búsqueda server-side es post-v1) | Diferido |
| 3.5.6 Zeroize de claves en RAM | **Resuelto**: estados y transitorios en `Zeroizing` (crate `zeroize`), args de comandos como `Zeroizing<String>` | Sin acción |
| `entities.external_id` | Columna existe pero **no se puebla** | **Pre-requisito M1** |
| `entities.updated_at` / soft delete | No existen (`update_entity` solo cambia `enc_data_blob`); no hay comando de borrado | `updated_at` en v002; soft delete solo si algún día hay delete |

---

## 3. Alcance de sync por tabla

### 3.1 Matriz de alcance

| Tabla / archivo | ¿Sync? | Mecanismo | Regla de conflicto |
|---|---|---|---|
| `entries` | ✅ | cola + pull | Unión por `id` (inmutable, `INSERT OR IGNORE`) |
| `entities` | ✅ | cola + pull | LWW por timestamp del servidor (§4) |
| `users` | ✅ | cola + pull | LWW + restricción de `username` único (§4.3) |
| `professional_profiles` | ✅ | cola + pull | LWW |
| `signing_keys` | ✅ | cola + pull | Merge por dedupe + reglas de vigencia (§4.4) |
| `vault_*.{bin,salt}`, `.data_key.{user}` | ✅ (D2) | blobs de archivo | LWW por blob |
| `.installation_salt` | ✅ (imprescindible) | blob de archivo | LWW; **primero en el orden de pull** (§6.4) |
| `.data_key.master` + frase semilla | ❌ | — | Break-glass local por dispositivo (los wraps master *nuevos* sí se propagan vía rotación, §5.5) |
| `audit_log` | ❌ v1 (local) | — | Cada equipo audita sus propias acciones sensibles |
| `sync_queue`, `sync_state`, `sync_cursors` | ❌ (estado local) | — | — |
| `entity_keys` (telemedicina) | ❌ fuera de alcance | — | — |
| Archivos de respaldo (`validate_backup`/`create_backup`) | ❌ | — | El respaldo es por equipo |
| `migrations` / `PRAGMA user_version` | ❌ | — | Lo maneja el binario (§7.4) |

### 3.2 El problema de los IDs

`entities.id` es `INTEGER PRIMARY KEY AUTOINCREMENT` **local**: dos equipos crean pacientes distintos con el mismo `id`. Sin resolver esto, un pull con `INSERT OR IGNORE` descartará datos reales.

Solución:

- `entities.external_id TEXT UNIQUE` (ya existe) se **backfillea con UUIDv4 en v002** y se asigna en cada `create_entity`.
- El sync de entities usa `external_id` como clave externa; el `id` INTEGER sigue siendo local y privado.
- El payload de sync de una entry lleva el `external_id` del paciente; al aplicar el pull se resuelve al `id` local (insert implícito del entity si aún no existe).
- `entries.id` ya es TEXT UUID → sirve como clave externa directa.
- `sync_queue.external_record_id` (columna existente) guarda el UUID de la fila empujada.

### 3.3 Migración v002 (pre-requisito)

```sql
-- backfill de UUIDs para entidades existentes
UPDATE entities SET external_id = lower(hex(randomblob(4))) || '-' || ... WHERE external_id IS NULL;
-- timestamp de modificación para LWW
ALTER TABLE entities ADD COLUMN updated_at TEXT NOT NULL DEFAULT '';
UPDATE entities SET updated_at = created_at WHERE updated_at = '';
CREATE INDEX idx_entities_updated ON entities(updated_at);

-- estado de sync por registro (LWW) y cursores por colección
CREATE TABLE sync_state (
  table_name    TEXT NOT NULL,
  record_key    TEXT NOT NULL,      -- external_id / user_id / path de archivo
  remote_updated TEXT NOT NULL,     -- último timestamp de servidor conocido
  PRIMARY KEY (table_name, record_key)
);
CREATE TABLE sync_cursors (
  collection TEXT PRIMARY KEY,
  cursor     TEXT NOT NULL
);
-- tracker de archivos sucios (vaults, wraps, salt)
CREATE TABLE file_sync (
  path TEXT PRIMARY KEY,
  dirty INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT
);
```

Notas:

- `entries.is_synced` queda **deprecado** (fuente única de verdad = `sync_queue`); no se elimina en v002 por prudencia.
- `users.last_login_at` **no se sincroniza** (churn en cada login, sin valor cross-device): va fuera del envelope.
- `update_entity` (hoy: `enc_data_blob`, `created_by_user_id`) pasa a tocar `updated_at = datetime('now')` y a encolar.

### 3.4 Outbox: la cola se alimenta en la misma transacción

Todo comando de escritura debe insertar en `sync_queue` **dentro de su transacción** (rollback atómico: o el cambio y su evento salen juntos, o ninguno). Comandos que encolan:

- `create_entity`, `update_entity`
- `create_entry`
- Admin: `create_user`, cambio de rol, `is_active`, reset de password, rotación de `signing_keys`, edición de perfil
- Auth: cambio de password (también marca sucios los wrap/vault correspondientes)
- Eventos de archivo (vault/wrap/salt): marcan `file_sync.dirty` (no son filas de tabla)

Pull aplicado desde el servidor **no re-encola** (o se marca `origin_device` propio y se omite al empujar); los upserts idempotentes hacen inocuo el eco.

---

## 4. Reglas de conflicto

### 4.1 Principio LWW con reloj de servidor

Los registros mutables resuelven por **orden de llegada al servidor**, nunca por reloj del cliente (un reloj manipulado no puede ganar). Ciclo de cada tick:

1. **PUSH** de lo local pendiente → el servidor responde/registra `updated` (su reloj) → se guarda en `sync_state.remote_updated`.
2. **PULL** de lo remoto → se aplica solo si `remote.updated > sync_state.remote_updated` (la última versión conocida del servidor).
3. Una edición local **no pusheada** sobrevive al pull: el remoto solo la pisa si es más nuevo que la última versión conocida, no que la edición local. La ventana de riesgo (ediciones simultáneas en dos equipos dentro del mismo tick de 60 s) se resuelve en el siguiente ciclo a favor del que llegó después al servidor.

### 4.2 Por tabla

| Tabla | Regla |
|---|---|
| `entries` | Unión pura. Sin UPDATE ni DELETE. Dos autores en paralelo → coexisten, orden por `timestamp`. |
| `entities` | LWW completo del registro (`enc_data_blob` + `blind_index` van juntos; gana el último que llegó). |
| `users` | LWW del envelope. `last_login_at` excluido. Ver 4.3 por `username`. |
| `professional_profiles` | LWW del blob. |
| `signing_keys` | Ver 4.4. |
| Archivos (vault/wrap/salt) | LWW por blob. |

### 4.3 Borde: `username` duplicado

`users.username` es UNIQUE. Si dos equipos crean el mismo username en paralelo:

- El pull **no** pisa silenciosamente: si el `user_id` remoto choca por `username` con uno local distinto, se descarta el registro, se incrementa un contador de conflictos en estado de sync y la UI de admin muestra *"Conflicto de usuario: elegir cuál prevalece"* (resolución manual, una sola vez, LWW después).
- Escenario raro en la práctica (el admin crea cuentas desde un equipo).

### 4.4 Merge de `signing_keys`

Clave de dedupe: `(user_id, public_key, valid_from)`.

- Mismo registro en ambos lados → no duplicar.
- Si un lado tiene `valid_to` cerrado y el otro abierto → **gana el cerrado** (la rotación es autoritativa); si ambos cerraron, gana el `valid_to` **más temprano** (regla más restrictiva).
- Regla de sesión: si este dispositivo recibe por sync un vault o wrap **más nuevo** que el que tiene abierto en sesión (rotación hecha en otro equipo: cambio de password, reset admin, rotación de semilla), la app muestra *"Tu llave de firma fue actualizada en otro equipo"* y **fuerza un re-login** antes de poder firmar de nuevo. Así ninguna entry se firma con una llave recién cerrada.

### 4.5 Verificación cross-install (invariante)

Una entry creada en el equipo A y recibida por B debe verificar en B:

- `author_id` resuelve en `users` sincronizado → FK OK.
- La firma se verifica contra `signing_keys` sincronizado usando el rango vigente **en el `timestamp` de la entry** (query existente en `entry_commands`).
- El `payload` se descifra con la DataKey compartida (D1/D3).

Este es el test invariante #1 de la fase.

---

## 5. Seguridad

### 5.1 Qué ve el servidor (y qué no)

**Ve:** ciphertexts (payloads, blobs de entities/users/profiles, vaults y wraps), `blind_index` (hash con sal), claves **públicas** de firma, UUIDs, timestamps, metadatos de dispositivos. **No ve:** ningún dato clínico ni identificante en claro, passwords, la DataKey, la frase semilla, roles (van dentro del envelope cifrado de `users`).

Residual aceptado: por volumen/timing y por los `blind_index` (propósito de búsqueda server-side) el servidor correlaciona registros del mismo paciente — ya declarado en `docs/estrategia_encriptacion.md` §3.

### 5.2 Cadenas de desbloqueo por archivo

| Blob sincronizado | Se abre con | Función |
|---|---|---|
| `.installation_salt` | es una sal, no se "abre" | Consistencia de blind index entre equipos (debe llegar **antes** de usar búsqueda) |
| `vault_{user}.bin/.salt` | Argon2id(password usuario) | Par Ed25519 del usuario (misma llave en todos los equipos) |
| `.data_key.{user}` | master del usuario (Argon2id + sal) | Fast path → DataKey sin semilla |
| envelope de `users` | master del usuario | **Rompe la circularidad del bootstrap** (§6.4): permite verificar password y reconstruir la fila local sin DataKey previa |
| resto (entities, entries, profiles) | DataKey | Datos clínicos |

### 5.3 Onboarding de dispositivo nuevo (v1)

```
1. Admin (cualquier equipo): Panel → Dispositivos → "Generar código de emparejamiento"
   → código de 6 caracteres, expira en 10 min, uso único (colección pairing_codes).
2. Equipo nuevo: "Unirse al consultorio" → ingresa código + URL del servidor.
   El servidor lo registra y devuelve un device token (se guarda local).
3. Primer pull (orden §6.4): salt → vaults/wraps → envelope users → perfiles/firmas → entities → entries.
4. El profesional inicia sesión normalmente: password → master → abre su envelope
   (verifica credencial) → vault (llave de firma) → .data_key.{user} → DataKey → todo descifrado.
```

- La **semilla no participa**: solo queda en el papel para break-glass.
- El código de emparejamiento **solo autentica al dispositivo**, no libera secrets (a diferencia del diseño de julio): sin password de usuario, el pull entrega blobs inútiles.
- Mitigaciones: código 6 caracteres alfanuméricos + expiración 10 min + uso único + **rate-limit en el endpoint de canje** (anti brute-force).

### 5.4 Revocación

Panel de admin → lista de dispositivos → revocar token. El dispositivo pierde sync de inmediato (pull/push rechazados). **Lo que ya replicó localmente no se puede deshacer** (era un requisito: todos ven todo); la revocación corta el flujo futuro. Documentado en la guía de operación.

### 5.5 Break-glass y propagación

El flujo break-glass (frase semilla → reset del admin más antiguo → vault/keypair nuevos → rotación de `signing_keys` → re-envuelve wraps) se ejecuta en **un** equipo y **se propaga por sync**:

- `.data_key.master` nuevo, vault del usuario reseteado y wraps → blobs LWW.
- `signing_keys` → merge §4.4.
- Los demás equipos reciben los cambios en el siguiente tick; el usuario afectado se re-loguea (§4.4 regla de sesión).

### 5.6 Amenazas nuevas vs. la app local

| Amenaza | Mitigación |
|---|---|
| Servidor comprometido /Proveedor cloud | Solo blobs cifrados (Argon2id en vaults y envelopes de users). Sin password no hay nada que cracking razonable abra. |
| Password + hash expuestos juntos (envelope users contiene `password_hash`) | El hash está para re-guardarlo localmente; el riesgo de cracking offline es el mismo que si robaran la DB local — Argon2id es la defensa, ya implementada. |
| Sniffing en LAN (perfil LAN sin TLS) | Los payloads son ciphertext desde el cliente; el riesgo residual es robo de token/código de pairing durante su uso → **recomendación: TLS también en LAN** (Traefik + cert autofirmada); HTTP solo como fallback documentado. |
| Reloj manipulado | LWW usa timestamps del servidor (§4.1). |
| Dispositivo robado | Revocación (§5.4) + cifrado local ya existente; sin password no abre nada. |
| Fork de versiones (app vieja vs. nueva) | `meta.min_app_version` en el servidor; un cliente con versión menor **no sincroniza** (falla con mensaje claro) — §7.4. |

---

## 6. Sync engine

### 6.1 Componente

Worker `tokio` dentro del proceso Tauri (nuevo módulo `src-tauri/src/sync/`): `outbox` (lee `sync_queue` + `file_sync`), `transport` (HTTP contra PocketBase), `pull` (cursores + upserts), `state` (`sync_state`, `sync_cursors`). Cliente HTTP: `reqwest` (ya presente en el ecosistema Tauri).

### 6.2 Ciclo

```
trigger: startup | cada 60 s | "Sincronizar ahora" | pull post-login
if sin conexión: backoff exponencial (60s → máx 15 min), UI "reintentando en Xs"

PUSH:  para cada fila de sync_queue (y archivo con file_sync.dirty), en orden:
         files → users → profiles/signing_keys → entities → entries   (respeta FK lógicas)
       → PUT/POST al servidor → guardar remote_updated en sync_state
       → DELETE de sync_queue / limpiar dirty
PULL:  por colección, desde sync_cursors[collection]:
         GET ?since=cursor → aplicar §4 → upsert idempotente → avanzar cursor
       → skip de records con origin_device propio
FIN:   checkpoint de WAL + UI "sincronizado hace Xs"
```

### 6.3 Modo offline

Nada cambia en la UX de trabajo: los comandos encolan como siempre, el indicador muestra *pendientes: N*. Al volver la red, el push drena la cola. El orden FIFO de `sync_queue` se conserva.

### 6.4 Orden de bootstrap (dispositivo nuevo y restablecimientos)

El orden no es cosmético:

1. `.installation_salt` (si no, los `blind_index` calculados no coinciden con los remotos)
2. vaults + wraps de `users` (habilitan el login por password)
3. envelope de `users` (credenciales locales)
4. `professional_profiles` + `signing_keys`
5. `entities`
6. `entries` (último: requiere entities para el FK y firmas para verificar)

Un pull parcial interrumpido es seguro: cada colección se aplica idempotentemente y el cursor avanza solo al confirmar.

### 6.5 Estado visible (UI)

- Pill de estado global: `Sincronizado (hace Xs)` / `Pendientes: N` / `Sin conexión — reintentando en Xs` / `Error: token revocado` / `Conflicto: requiere atención del admin`.
- Panel admin → **Dispositivos**: alta (generar código), lista (nombre, último visto, versión de app), revocar.
- Modal "Tu llave de firma fue actualizada en otro equipo → re-login" (§4.4).
- Pantalla de bienvenida "Unirse al consultorio" (paso 2 del §5.3).

---

## 7. Infraestructura

### 7.1 Perfiles de deploy

| | Perfil LAN | Perfil VPS |
|---|---|---|
| Dónde | Docker Compose en la PC del consultorio | VPS (Hetzner/DigitalOcean) + Dokploy o Compose |
| Red | Bind a IP local; sin puertos públicos | Traefik + Let's Encrypt (HTTPS obligatorio), firewall 443/22 |
| Costo | $0 | ~USD 4–6/mes |
| Acceso | Solo en la red del consultorio | También desde casa/viajes (misma app, otra URL) |
| Config en la app | `server_url` = `http://192.168.x.x:8090` | `server_url` = `https://sync.ejemplo.com` |

`docker-compose.yml` con profiles (`lan` / `vps`) y volumen persistente para la base de PocketBase.

### 7.2 Colecciones PocketBase

| Colección | Rol | Contenido |
|---|---|---|
| `pairing_codes` | Emparejamiento temporal | `code` (6 chars), `installation_id`, `expires_at` (10 min), `used` |
| `devices` | Identidad de equipo | `device_id`, `name`, `token_hash`, `app_version`, `created_at`, `revoked_at` |
| `sync_entities` | Relé | `external_id` (PK), `entity_type`, `blind_index`, `enc_data_blob`, `origin_device` |
| `sync_entries` | Relé | `id` UUID (PK), `subject_external_id`, `author_id`, `category`, `timestamp`, `enc_meta` (title/status cifrados), `payload` (cifrado), `signature`, `origin_device` |
| `sync_users` | Relé | `user_id` (PK), `enc_envelope` (master-key del usuario), `origin_device` |
| `sync_profiles` | Relé | `user_id` (PK), `enc_blob` (DataKey) |
| `sync_signing_keys` | Relé | `(user_id, public_key, valid_from)` PK, `valid_to` — claves públicas, en claro por definición |
| `files` | Blobs de archivo | `key` (path lógico: `salt` / `vault:{user}` / `wrap:{user}`), `content`, `origin_device` |
| `meta` | Handshake | `installation_id`, `min_app_version`, `schema_hint` |

Timestamps de LWW: el campo `updated` de PocketBase (reloj del servidor) — no se confía en `created_at` de cliente.

> **Nota:** `entries.title` y `status` son columnas en claro **localmente**; para el servidor van dentro de `enc_meta` cifrado con DataKey (nada de metadata clínica en claro en la nube).

### 7.3 Auth del servidor

- Superusuario de PocketBase solo para ops (panel + TOTP), **no** para la app.
- Los dispositivos se autentican con **device token** (Bearer) por colección/registro; reglas de acceso: token válido → leer/escribir solo su `installation_id`.
- Rate-limit en `pairing_codes` (reverso o hook).

### 7.4 Versiones y migraciones

- El `user_version` local es de cada equipo; **no se sincroniza**. Lo que sincroniza es el binario.
- Regla: todos los equipos de un consultorio deben correr la **misma versión menor** de la app. El handshake de sync compara `app_version` contra `meta.min_app_version`: versión menor → sync bloqueado con *"Actualizá la app en este equipo"*.
- Migraciones de esquema (v002, v003…) siguen el mecanismo existente (`MIGRATIONS` + guard de downgrade), por equipo, al abrir.

### 7.5 Backups del servidor

- Backup diario de la base de PocketBase (job propio de PB o script cron) con retención ≥ 30 días, al volumen o S3/MinIO en el perfil VPS.
- Los respaldos **locales** por equipo (`create_backup`) siguen siendo el mecanismo de recuperación de disaster local: restaurar un respaldo local + sync = estado recuperado.

---

## 8. Testing

| Nivel | Qué cubre | Herramienta |
|---|---|---|
| Unit | Reglas de conflicto (tabla-driven: LWW, dedupe signing_keys, username duplicado), outbox atómica (misma transacción), resolución de IDs external↔local, roundtrip de envelopes | `cargo test` |
| Unit red | Push/pull contra servidor simulado | `wiremock` |
| Integración | Engine completo contra **PocketBase real en Docker** (compose de test): pairing → push → pull → revocación | test `#[ignore]` local + job opcional de CI |
| 2 dispositivos | Dos directorios de datos temporales + dos `Connection` en el mismo test: A crea entry → B la recibe y **verifica la firma de A** (invariante §4.5); B edita entity → A aplica LWW; admin desactiva usuario en A → B lo bloquea en el próximo pull/re-login | `cargo test` |
| Fallas | Servidor caído (backoff), token revocado, pull interrumpido a mitad (cursor no avanza), skew de reloj, eco `origin_device` | unit + integración |
| E2E de flujo | Onboarding de equipo nuevo completo (§5.3) | manual en checklist de release + (futuro) tauri-driver |

Cobertura objetivo: las reglas de §4 y el orden de §6.4 tienen test de invariantes, no solo happy path.

---

## 9. Milestones

| # | Hito | Contenido | Tamaño |
|---|---|---|---|
| M0 | Este documento | Aprobado | ✅ |
| M1 | Pre-requisitos locales | v002 (external_id, `updated_at`, `sync_state`/`sync_cursors`/`file_sync`), outbox en todos los comandos de escritura, verificación de zeroize, CI verde sin servidor | M |
| M2 | Infra + identidad de dispositivo | Compose perfiles lan/vps, colecciones PB, pairing + tokens, panel admin "Dispositivos" | M |
| M3 | Engine push/pull | Worker, reglas de conflicto, indicador de estado, tests unit + integración PB + 2 dispositivos | L |
| M4 | Onboarding + vaults E2E | Flujo "Unirse al consultorio", sync de archivos (salt/vaults/wraps), re-login por rotación, invariante de firma cross-install | M |
| M5 | Hardening y docs | Guía de deploy LAN/VPS, operación (revocación, break-glass multi-equipo), checklist de release, §5 de `estrategia_encriptacion.md` actualizado, README roadmap | S |

**Fuera de alcance de la Fase 4 (post-v1):** realtime PocketBase (v2), búsqueda server-side con múltiples blind indexes, centralización de `audit_log`, telemedicina / `entity_keys`, multi-clínica (varias instalaciones con claves distintas), `previous_entry_id` (cadena de correcciones), notificaciones push, app web.

---

## 10. Riesgos

| Riesgo | Prob. | Impacto | Mitigación |
|---|---|---|---|
| Servidor caído | Media | Bajo | App 100% offline; sync se reanuda solo |
| Colisión LWW en ediciones simultáneas de una entity | Media | Bajo | Ventana de 60 s; gana el último en llegar; en la práctica una filiación rara vez se edita a la vez en dos equipos |
| Cambio de password en A mientras B firma | Baja | Alto | Regla de sesión por rotación (§4.4): B se re-loguea antes de firmar |
| Código de emparejamiento adivinado | Muy baja | Alto | 10 min + uso único + rate-limit; y el código solo da token, no secrets |
| Fork de versiones de app | Media | Medio | Handshake `min_app_version` (§7.4); checklist: actualizar todos los equipos juntos |
| Colisión de `username` en paralelo | Baja | Medio | Resolución manual visible en admin (§4.3) |
| VPS/PB comprometido | Baja | Alto | Tubo ciego: blobs Argon2id/AES-GCM; rotación de token de dispositivo |
| Perfil LAN sin TLS | Media | Medio | Recomendación TLS también en LAN; riesgo residual documentado |
| Dispositivo robado con sesión abierta | Baja | Alto | Mismo riesgo que hoy local; revocación corta sync futuro |
| Tamaño del pull inicial en bases grandes | Media | Medio | Pull por colección con cursores, incremental y reanudable |

---

## 11. Checklist de entrega de la Fase 4

- [ ] M1: v002 aplicada, outbox en todos los comandos, `external_id` siempre poblado
- [ ] Compose perfil LAN y perfil VPS funcionando; PocketBase con colecciones y reglas por `installation_id`
- [ ] Emparejamiento: generar código → canjear → token → revocación probada
- [ ] Push: cola drena en orden; pull: cursores avanzan; eco `origin_device` descartado
- [ ] Unión de entries + LWW de entities + merge de signing_keys con tests de invariantes
- [ ] Invariante: firma creada en A **verifica** en B (users + signing_keys + DataKey sincronizados)
- [ ] Bootstrap de equipo nuevo completo sin frase semilla (solo código + password)
- [ ] Semilla: exclusiva de break-glass, no aparece en ningún flujo de red
- [ ] Rotación de llave en A → B pide re-login antes de firmar
- [ ] Indicador de estado + panel de dispositivos en UI
- [ ] Backups diarios del servidor + respaldo local por equipo documentados
- [ ] Guía de deploy LAN y VPS; guía de operación (revocación, versiones, break-glass multi-equipo)
- [ ] README y `docs/estrategia_encriptacion.md` §5 actualizados al estado final
- [ ] Tests 2 dispositivos en CI (unit/wiremock) + integración PB manual documentada

---

*Diseño aprobado 2026-10-03: hosting híbrido, vaults sincronizados cifrados, cuentas sincronizadas, polling v1. Sustituye a la hoja de ruta de julio 2026 como referencia de la Fase 4.*
