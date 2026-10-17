# 🔐 Estrategia de Cifrado y Privacidad — Simplex Health Core

Este documento detalla la arquitectura de seguridad, los algoritmos criptográficos y el flujo de datos del Core. El sistema está diseñado bajo el principio de **Zero-Knowledge Parcial (Conocimiento Cero)** y **Local-First**, garantizando que los datos de salud e identificación de los pacientes nunca sean expuestos en texto plano fuera del dispositivo local del profesional autorizado.

---

## 🏗️ 1. Pilares Criptográficos

El Core de Rust utiliza librerías de nivel de producción (`aes-gcm`, `argon2`, `sha2`, `ed25519-dalek`) para ejecutar cuatro tipos de operaciones criptográficas complementarias en la memoria RAM:

| Operación | Algoritmo | Propósito | Implementación |
| :--- | :--- | :--- | :--- |
| **Cifrado Simétrico** | `AES-GCM-256` | Blindar textos clínicos (entries), filiación (nombres) y el payload del perfil. | Cifrado Autenticado con datos asociados (AEAD). |
| **Firma Digital** | `Ed25519` | No-repudio y validez legal de cada entry clínica. | Firma determinística (RFC 8032) sobre SHA-256 del payload de firma; verificación con la llave pública del autor. |
| **Derivación de Claves (KDF) / Hashing** | `Argon2id` | Hasheo de contraseñas locales (Auth) y derivación de la Clave Maestra desde la contraseña del usuario. | Ganador de la Password Hashing Competition (PHC). |
| **Índice Ciego (Blind Index)** | `SHA-256 + Salt` | Búsqueda y prevención de registros duplicados de forma anónima. | Hashing determinista e irreversible. |

---

## 🔒 2. Ciclo de Vida de los Datos Clínicos (Entries Append-Only)

El sistema clínico vive en la tabla `entries` (paradigma Entity-Entry): un registro **inmutable** por hecho clínico — SOAP, medicación, alergia o condición. No existen operaciones de update ni delete; las correcciones se hacen con entries nuevas.

```text
[ React Frontend ] (Payload JSON en RAM, armado según config/entry_templates/*.json)
       │
       ▼ (IPC - Tauri Bridge)
[ Rust Core Engine ]
       │──► 1. Extrae la DataKey del estado `DataKey` (Mutex en RAM).
       │──► 2. Serializa el payload y genera un Nonce de 12 bytes (OsRng).
       │──► 3. AES-GCM-256 sobre el payload → `entries.payload` (BLOB cifrado).
       │──► 4. Hash SHA-256 del material de firma (JSON canónico: category, subject_id,
       │       title, status, timestamp, author_id, id, created_at, hex(payload_cifrado)).
       │──► 5. Firma Ed25519 del hash con la llave privada del autor → `entries.signature`.
       ▼
[ SQLite Local ] (Solo texto cifrado + firma — nunca payload legible)
```

### Reglas de Implementación en Disco

* `entries.payload`: JSON completo de la entry cifrado con la **DataKey** (nonce incluido en el blob).
* `entities.enc_data_blob`: filiación del paciente cifrada campo a campo con la DataKey.
* `professional_profiles.*_ciphertext`: cada dato del perfil cifrado individualmente (nombre, matrícula, especialidad, contacto).
* `entries.signature`: firma Ed25519 en hex — verificable con la llave pública del autor (ver §4.4).
* `vault_{user_id}.bin`: llave privada de firma Ed25519 cifrada con la master key derivada de la contraseña (ver §4.1).
* `.data_key.{user_id}` / `.data_key.master`: envoltorios AES-GCM de la DataKey (ver §4.2).

> ⚠️ **Nota de Seguridad:** Dos payloads con el mismo texto generarán cadenas de bytes completamente diferentes en la base de datos, porque el `nonce` se genera con un generador criptográficamente seguro (`rand::rngs::OsRng`) en cada escritura. Los tests del Core verifican además que ninguna palabra en claro del payload aparezca en la DB.

---

## 🚫 3. Estrategia de No-Duplicación: Índices Ciegos (Blind Indexing)

En un ecosistema web tradicional, el servidor evita pacientes duplicados aplicando un índice `UNIQUE` a la columna de identidad (DNI o Pasaporte). En este desarrollo el DNI nunca llega a un servidor remoto ni se guarda en texto plano en el disco local.

Para resolver esto sin romper el modelo Zero-Knowledge, el Core implementa un **Índice Ciego Determinista con HMAC**:

1. El usuario ingresa el documento de identidad en la interfaz.
2. El Core de Rust normaliza el texto (remueve espacios, guiones y lo transforma a minúsculas).
3. Se deriva una clave de dominio de la DataKey: `HK = HMAC-SHA256(DataKey, "fichio/blind-index/v1")` (se calcula al vuelo; la DataKey no cambia con contraseñas ni rotaciones, así que los índices son estables).
4. Se calcula `HMAC-SHA256(HK, DNI normalizado)` en hexadecimal.

$$\text{Blind Index} = \text{HMAC-SHA256}(\text{HK}, \text{DNI Normalizado})$$

No hay sal persistida: `.installation_salt` fue **eliminado** (del disco, del respaldo y del protocolo de sincronización). Un HMAC solo puede verificarse —no invertirse— con la clave: un servidor que ve los índices **no puede enumerar el espacio de DNIs** (~10⁸ candidatos) sin la DataKey, y la consistencia entre dispositivos de la instalación la aporta la DataKey compartida, no un archivo sincronizado.

```sql
-- El hash resultante se guarda en la columna 'blind_index' de `entities`
-- con restricción UNIQUE. Si se intenta ingresar el mismo DNI, SQLite
-- rebotará la query por duplicación, sin que el motor de base de datos
-- sepa jamás qué DNI originó ese hash.
```

---

## 🔑 4. Ciclo de Vida de las Claves

La seguridad descansa sobre la generación y destrucción estricta de las llaves en la memoria volátil (RAM). **No existen claves hardcodeadas en el binario.** Las llaves viven envueltas en `Zeroizing` (crate `zeroize`): al liberarse se **ponen a cero** en vez de dejar bytes residuales en el heap — borrado *best-effort*, con los límites documentados en §4.7. El sistema maneja **dos familias de claves** con responsabilidades distintas:

### 4.0 Secuencia de unlock (cold start)

El sistema no utiliza claves hardcodeadas. Las claves nacen y mueren en la RAM:

1. El usuario ingresa su contraseña en la UI.
2. React llama a `unlock_vault`. Rust verifica el hash en la tabla `users` (Argon2id).
3. La misma contraseña se usa como semilla para derivar 32 bytes mediante Argon2id en modo KDF (Llave Maestra). La sal del KDF es única por usuario (`vault_{user_id}.salt`).
4. Se desbloquea el archivo `vault_{user_id}.bin`, el cual contiene la Llave Privada de Firma (Ed25519) cifrada con la Llave Maestra.
5. Se resuelve la **DataKey** (ver §4.2): wrap personal → master wrap → migración legacy → generación.
6. Las tres llaves (Master, Signing, DataKey) se inyectan en `CryptoState`, `SigningState` y `DataKey` (RAM volátil de Rust).
7. Al cerrar sesión, React llama a `lock_vault`. Rust descarta los `Option` de las tres llaves: cada `Zeroizing` se **pone a cero al caer** (`Drop`), en vez de dejar los 32 bytes residuales en la memoria. Las llaves dejan de estar disponibles para cualquier lectura posterior del proceso.

**Nota**: la DataKey **no** se carga al arrancar la app — solo existe en RAM después de un `unlock_vault` exitoso.

### 4.1 Vault por usuario → llave de firma Ed25519

1. **Derivación:** la contraseña del usuario se procesa con `Argon2id` (sal única por usuario en `vault_{user}.salt`) para generar exactamente 32 bytes de *master key*.
2. **Validación (`vault.rs`):** esos 32 bytes descifran `vault_{user}.bin` (AES-GCM). Si la firma mágica `HISTORIA_CLINICA_VAULT_OK` aparece, la contraseña es correcta y el payload contiene la **llave privada Ed25519** del usuario.
3. **Primer login:** si el vault no existe, se crea con un keypair nuevo.
4. **Inyección en Memoria:** la llave privada y la master key viven solo en `Mutex` de Rust (`CryptoState`/`SigningState`); el frontend (React) nunca las ve — solo recibe datos ya descifrados por el canal IPC.
5. **Destrucción:** `lock_vault` descarta los `Option` de los states y limpia la sesión (`SessionState`); cada `Zeroizing` se pone a cero en su `Drop`. Al cerrar la aplicación, el proceso de Rust muere y todo desaparece de la RAM. Nota: es borrado *best-effort* — con la bóveda abierta, un volcado de memoria del proceso expone las llaves vivas (ver §4.7).

### 4.2 DataKey de instalación → datos clínicos

Una única **DataKey** (32 bytes, generada una sola vez) cifra todo lo clínico: `entries.payload`, `entities.enc_data_blob` y el perfil profesional. Nunca se persiste en claro; se guarda envuelta:

| Archivo | Llave que la envuelve | Rol |
| :--- | :--- | :--- |
| `.data_key.{user_id}` | master key derivada de la contraseña del usuario (Argon2id) | **Fast path**: login normal sin tipear la semilla |
| `.data_key.master` | `Argon2id(frase semilla, sal de dominio fichio/seed-kdf/v1)` | **Bootstrap / recovery**: primer login y wrap huérfano |
| `.data_key` (legacy) | — | Hex en claro — se migra y **borra** en el primer login |

**Orden de resolución** (`resolve_data_key`, en `unlock_vault`):

1. Wrap personal existe → desenvuelve con master_key → listo.
2. Master wrap existe + seed provista → desenvuelve, crea wrap personal → listo.
3. Master wrap existe sin seed → error `SEED_REQUIRED` → frontend pide la frase del papel.
4. Legacy hex existe → migra, borra legacy, crea wrap personal.
5. Nada → genera DataKey nueva (primera instalación).

Los tests de invariante garantizan que un cambio de contraseña o un reset **nunca alteran la DataKey** — solo se re-envuelve el wrap personal con la nueva master key, y **solo para roles staff**: el paciente no tiene wrap (ausencia estructural) y sus cambios de contraseña son solo del hash de login.

### 4.3 Aislamiento multi-usuario (qué protege y qué no)

> ⚠️ **Corrección importante:** los vaults (y por tanto las llaves de firma) son **por usuario**, pero la DataKey es **compartida por instalación**. Por eso el aislamiento entre consultas **no es criptográfico entre usuarios**: cualquier usuario **staff** con sesión abierta —médico, enfermería, asistente o administrador— descifra los mismos datos clínicos. La distinción entre roles es de **autorización y UI** (ver §4.6), no de cifrado. La excepción es el rol `paciente`: inicia sesión pero **no tiene DataKey** (ausencia estructural — no hay wrap personal para él), así que todos los commands clínicos le fallan por diseño.

Lo que el modelo Zero-Knowledge protege de verdad es el **disco**: sin la contraseña o la frase semilla, la base local es un bloque incomprensible — incluso para alguien con acceso físico o para el propio administrador, que en su cuenta solo tiene un wrap personal idéntico al de cualquier otro usuario **staff** y no custodia claves de datos ajenas. (El paciente no tiene wrap alguno: no hay ningún `.data_key.{user_id}` suyo en disco.)

### 4.4 Firmas y el histórico `signing_keys`

* Cada entry se firma con Ed25519 sobre `SHA-256(JSON_canónico)` — un struct con **todos los campos de la fila** (`category`, `subject_id`, `title`, `status`, `timestamp`, `author_id`, `id`, `created_at`) más `hex(payload_cifrado)`, serializado en orden de declaración (`signed_hash`, la única fuente de verdad del formato). Se firman los bytes persistidos, no el plaintext: alterar el payload rompe la firma aunque el atacante tenga la DataKey para re-cifrar; alterar `id` o `created_at` también. El JSON canónico elimina la ambigüedad de re-partición del viejo formato concatenado con `|` (un pipe en el title ya no puede desplazar la frontera con los campos vecinos). La verificación usa la **llave pública del autor**.
* `professional_profiles.public_key` guarda la clave **vigente** (se sincroniza en cada `unlock_vault`).
* `signing_keys` guarda el **histórico**: rangos `valid_from`/`valid_to` por clave. La verificación contrasta cada entry contra la clave vigente en **el timestamp de la entry** (`public_key_at`), con fallback al perfil para registros previos al histórico.
* **Cambio de contraseña propio:** mismo keypair → nada cambia en las firmas.
* **Reset por el administrador** (`admin_reset_password`, sin contraseña previa): regenera el vault con keypair **nuevo**, re-envuelve el wrap personal (**solo staff** — para un paciente no hay wrap que crear), actualiza la pública vigente y cierra el rango anterior en `signing_keys` — las entries viejas siguen verificando con la clave que les corresponde por fecha.

### 4.5 Frase semilla (recovery, continuidad y break-glass)

* 6 palabras generadas en el dispositivo desde una wordlist curada (sin duplicados ni tildes, evita palabras ambiguas), mostradas una sola vez, **jamás persistidas en disco ni en la base**.
* Deriva `.data_key.master` con **Argon2id** (salt de dominio `fichio/seed-kdf/v1` — constante por diseño: sin sal aleatoria por instalación para que el sync futuro no exija sincronizar sal). **No sustituye a la contraseña** en el login normal.
* **UI de configuración** (`SeedPhraseSetup`, 4 pasos): Generar → Mostrar (anotar en papel) → Verificar (reescribir) → Confirmar (master wrap). El botón "Omitir" no marca la seed como configurada.
* **Rotación** (tab del administrador): generar → anotar → re-escribir para verificar → recién entonces `admin_rotate_seed` reenvuelve `.data_key.master`. Hasta el último paso la frase anterior sigue siendo válida, y la acción queda registrada en `audit_log`.
* **Break-glass** (`seed_recovery`, desde el enlace de login): sí permite recuperar el acceso cuando se olvidó **todas** las contraseñas. La frase se verifica contra `.data_key.master` (mismo `unwrap` del recovery de datos — poseer la frase ya es poder descifrar todo, así que este paso solo lo formaliza y audita). Se restablece la contraseña del administrador activo más antiguo con vault y keypair nuevos, se rota `signing_keys` (las firmas históricas siguen verificando por fecha) y la acción queda en `audit_log` con acción `seed_recovery`. La DataKey no cambia.
* **No sirve** si el usuario olvidó la contraseña (el login falla antes de pedir la frase) — para eso existe el reset del administrador o el break-glass.

### 4.6 Custodia: roles y autorización en backend

`login_user` registra la sesión activa en `SessionState` (Rust) y `lock_vault` la limpia. Todo command sensible exige `require_admin()` —rol `administrador`— **del lado de Rust**, nunca confiando en flags del frontend:

* altas y bajas lógicas de usuarios (`admin_create_user`, `set_user_active`),
* resets de contraseña ajenos,
* rotación de la frase semilla,
* lectura del registro de auditoría (`audit_log`, con actor y objetivo).

Y el acceso de paciente usa el gate intermedio `require_admin_or_assistant()` (operación de admisión, no de custodia): `grant_patient_access` y `get_entity_access`.

El administrador **no gana lectura nueva de datos clínicos** (misma DataKey, mismo wrap que todos): su poder es sobre *cuentas y credenciales*, y cada uso queda auditado. El paciente, con sesión abierta, no gana lectura alguna: sin DataKey, ningún command clínico le responde.

### 4.7 Límites de zeroize (honestidad sobre la limpieza de RAM)

`zeroize` es un borrado de **best-effort**, no una garantía criptográfica:

- **Sí hace:** poner a cero el buffer exacto que posee el `Zeroizing` (llaves de 32 bytes, contraseñas/frases `String`, buffers intermedios de desencriptación del vault y de unwrap de data_key) cuando se libera, de forma resistente a la optimización del compilador (writes volátiles).
- **No puede garantizar:** copias que escape del `Zeroizing` — buffers de `serde_json` en el canal IPC (el IPC copia argumentos y resultados), registros/dump de pila del runtime, memoria de swap o de archivos de página si el OS intercambia el proceso, ni el heap de V8. En el frontend solo se limpian **referencias** (`setPassword("")`): V8 no permite borrado criptográfico de strings.
- **Máximo práctico:** la ventana que se protege es la que va *después* de liberar la llave (`lock_vault`, fin de command, cierre de sesión). **Con la bóveda abierta, un volcado de memoria del proceso expone las llaves vivas** — esa es una limitación de cualquier app que necesite usar las llaves en RAM. No se usan `mlock`/`VirtualLock` ni se audita el swap.

---

## ☁️ 5. Flujo de Sincronización Segura (Futuro: PocketBase)

Cuando se implemente el puente de sincronización, el servidor remoto actuará como un mero casillero de almacenamiento ciego:

1. **Validación de Canales:** Los datos viajarán sobre TLS (HTTPS), pero aunque el canal TLS fuese vulnerado (Man-in-the-Middle), los payloads ya viajan encriptados desde el cliente.
2. **Estructura del Payload Remoto:** El servidor en la nube solo almacenará:
    * UUIDs relacionales generados aleatoriamente.
    * Bloques de texto cifrado incomprensibles (Hexadecimal).
    * Nonces públicos.
    * El `blind_index` para indexación y búsquedas opacas (HMAC-SHA256 con clave derivada de la DataKey: el servidor compara igualdad, no invierte).
3. **Filosofía Zero-Knowledge:** El proveedor de la nube (o cualquier atacante que acceda al servidor central) solo verá metadatos correlativos, haciendo imposible la reconstrucción de la historia clínica de un paciente o su identificación legal.

---

## 💾 6. Respaldo y Restauración Local

El mecanismo de respaldo no introduce claves nuevas: copia los archivos que **ya están cifrados** y les agrega una huella de integridad.

### Contenido de un respaldo (`respaldo-{timestamp}/`)

| Archivo | Rol en la recuperación |
| :--- | :--- |
| `historia_clinica.db` (con `wal_checkpoint(TRUNCATE)` previo) | Todos los datos (cifrados por campo/payload) |
| `.data_key.master` | Recuperación con la **frase semilla** |
| `.data_key.{user}` | Recuperación con la **contraseña** del usuario (fast path; roles staff — el paciente no tiene wrap) |
| `vault_*.{bin,salt}` | Llaves de firma por usuario (si faltan, se regeneran al hacer login sin perder verificabilidad histórica) |
| `manifest.json` | `schema_version` + **SHA-256 por archivo** + **firma Ed25519** del admin que lo creó (integridad y autenticidad al restaurar) |

### Reglas

1. **Nada en claro**: el manifest solo contiene nombres, tamaños, hashes y los campos de firma; todo lo demás es ciphertext envuelto. Un respaldo robado es tan inútil como la base sin contraseña ni frase.
2. **Validación antes de tocar nada**: `validate_backup` verifica cada SHA-256, rechaza `schema_version` más nuevo que la app y nombres con separadores de ruta (path traversal).
3. **Firma del manifest**: `create_backup` lo firma con la llave Ed25519 del admin (la de su bóveda, en RAM) sobre su material canónico (`ManifestSignable`: versión, `created_at`, hashes). La verificación **ancla la pública en el histórico local `signing_keys`** (`public_key_at` por `created_at` — un respaldo viejo verifica tras rotaciones); la clave embebida en el propio manifest es solo informativa, porque quien escribe el archivo también puede reemplazarla. Estados: *verificada* → restore normal; *no verificable* (manifest legítmo sin firma o firmante ajeno a esta instalación) → el admin debe confirmar el restore de todos modos; *inválida* (el firmante es conocido y la firma no coincide) → restore bloqueado. Cualquiera con acceso de escritura al respaldo puede alterarlo, recalcular hashes y **reescribir el manifest** — sin llave privada no puede re-firmarlo, y sin llave registrada localmente su manifest no passa como verificado.
4. **Orden destructivo del restore**: validar → auditar (`backup_restored`, con nota de si la firma se verificó) → cerrar llaves, sesión y conexión SQLite → copiar archivos → reabrir con `run_migrations`. Si la copia falla, se reintenta abrir la base previa.
5. **Matriz de recuperación tras restaurar**:
   * Contraseña conocida → login normal → wrap personal (o semilla si no hay wrap) → todo. Para el paciente su contraseña es solo la de login: no hay wrap que recuperar.
   * Sin contraseñas → break-glass con frase semilla (§4.5).
   * Sin frase ni ninguna contraseña → los datos son irrecuperables por diseño (*crypto-shredding*): por eso el respaldo y la frase se guardan juntos y separados del equipo.
