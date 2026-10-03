# 🔐 Estrategia de Cifrado y Privacidad — Simplex Health Core

Este documento detalla la arquitectura de seguridad, los algoritmos criptográficos y el flujo de datos del Core. El sistema está diseñado bajo el principio de **Zero-Knowledge (Conocimiento Cero)** y **Local-First**, garantizando que los datos de salud e identificación de los pacientes nunca sean expuestos en texto plano fuera del dispositivo local del profesional autorizado.

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
       │──► 4. Hash SHA-256 de `category|subject_id|title|status|timestamp`.
       │──► 5. Firma Ed25519 del hash con la llave privada del autor → `entries.signature`.
       ▼
[ SQLite Local ] (Solo texto cifrado + firma — nunca payload legible)
```

### Reglas de Implementación en Disco

* `entries.payload`: JSON completo de la entry cifrado con la **DataKey** (nonce incluido en el blob).
* `entities.enc_data_blob`: filiación del paciente cifrada campo a campo con la DataKey.
* `professional_profiles.*_ciphertext`: cada dato del perfil cifrado individualmente (nombre, matrícula, especialidad, contacto).
* `entries.signature`: firma Ed25519 en hex — verificable con la llave pública del autor (ver §4.4).

> ⚠️ **Nota de Seguridad:** Dos payloads con el mismo texto generarán cadenas de bytes completamente diferentes en la base de datos, porque el `nonce` se genera con un generador criptográficamente seguro (`rand::rngs::OsRng`) en cada escritura. Los tests del Core verifican además que ninguna palabra en claro del payload aparezca en la DB.

---

## 🚫 3. Estrategia de No-Duplicación: Índices Ciegos (Blind Indexing)

En un ecosistema web tradicional, el servidor evita pacientes duplicados aplicando un índice `UNIQUE` a la columna de identidad (DNI o Pasaporte). En este desarrollo el DNI nunca llega a un servidor remoto ni se guarda en texto plano en el disco local.

Para resolver esto sin romper el modelo Zero-Knowledge, el Core implementa un **Índice Ciego Determinista**:

1. El usuario ingresa el documento de identidad en la interfaz.
2. El Core de Rust normaliza el texto (remueve espacios, guiones y lo transforma a minúsculas).
3. Se concatena con una clave secreta del sistema (`BLIND_INDEX_SALT`).
4. Se procesa a través de **SHA-256** y se convierte a Hexadecimal.

$$\text{Blind Index} = \text{SHA-256}(\text{DNI Normalizado} + \text{BLIND\_INDEX\_SALT})$$

```sql
-- El hash resultante se guarda en la columna 'blind_index' de `entities`
-- con restricción UNIQUE. Si se intenta ingresar el mismo DNI, SQLite
-- rebotará la query por duplicación, sin que el motor de base de datos
-- sepa jamás qué DNI originó ese hash.
```

---

## 🔑 4. Ciclo de Vida de las Claves

La seguridad descansa sobre la generación y destrucción estricta de las llaves en la memoria volátil (RAM). **No existen claves hardcodeadas en el binario.** El sistema maneja **dos familias de claves** con responsabilidades distintas:

### 4.1 Vault por usuario → llave de firma Ed25519

1. **Derivación:** la contraseña del usuario se procesa con `Argon2id` (sal única por usuario en `vault_{user}.salt`) para generar exactamente 32 bytes de *master key*.
2. **Validación (`vault.rs`):** esos 32 bytes descifran `vault_{user}.bin` (AES-GCM). Si la firma mágica `HISTORIA_CLINICA_VAULT_OK` aparece, la contraseña es correcta y el payload contiene la **llave privada Ed25519** del usuario.
3. **Primer login:** si el vault no existe, se crea con un keypair nuevo.
4. **Inyección en Memoria:** la llave privada y la master key viven solo en `Mutex` de Rust (`CryptoState`/`SigningState`); el frontend (React) nunca las ve — solo recibe datos ya descifrados por el canal IPC.
5. **Destrucción:** `lock_vault` limpia los states y la sesión (`SessionState`); al cerrar la aplicación, el proceso de Rust muere y todo desaparece de la RAM.

### 4.2 DataKey de instalación → datos clínicos

Una única **DataKey** (32 bytes, generada una sola vez) cifra todo lo clínico: `entries.payload`, `entities.enc_data_blob` y el perfil profesional. Nunca se persiste en claro; se guarda envuelta:

| Archivo | Llave que la envuelve | Rol |
| :--- | :--- | :--- |
| `.data_key.{user_id}` | master key derivada de la contraseña del usuario | **Fast path**: login normal sin tipear la semilla |
| `.data_key.master` | `SHA-256(frase semilla)` | **Bootstrap / recovery**: primer login y wrap huérfano |

**Orden de resolución** (`resolve_data_key`): wrap personal → semilla → generación (`SEED_REQUIRED` pide la frase al frontend, nunca falla perdiendo datos). Los tests de invariante garantizan que un cambio de contraseña o un reset **nunca alteran la DataKey** — solo se re-envuelve el wrap personal con la nueva master key.

### 4.3 Aislamiento multi-usuario (qué protege y qué no)

> ⚠️ **Corrección importante:** los vaults (y por tanto las llaves de firma) son **por usuario**, pero la DataKey es **compartida por instalación**. Por eso el aislamiento entre consultas **no es criptográfico entre usuarios**: cualquier usuario con sesión abierta —médico, enfermería, asistente o administrador— descifra los mismos datos clínicos. La distinción entre roles es de **autorización y UI** (ver §4.6), no de cifrado.

Lo que el modelo Zero-Knowledge protege de verdad es el **disco**: sin la contraseña o la frase semilla, la base local es un bloque incomprensible — incluso para alguien con acceso físico o para el propio administrador, que en su cuenta solo tiene un wrap personal idéntico al de cualquier otro usuario y no custodia claves de datos ajenas.

### 4.4 Firmas y el histórico `signing_keys`

* Cada entry se firma con Ed25519 sobre `SHA-256(category|subject_id|title|status|timestamp)`. La verificación usa la **llave pública del autor**.
* `professional_profiles.public_key` guarda la clave **vigente** (se sincroniza en cada `unlock_vault`).
* `signing_keys` guarda el **histórico**: rangos `valid_from`/`valid_to` por clave. La verificación contrasta cada entry contra la clave vigente en **el timestamp de la entry** (`public_key_at`), con fallback al perfil para registros previos al histórico.
* **Cambio de contraseña propio:** mismo keypair → nada cambia en las firmas.
* **Reset por el administrador** (`admin_reset_password`, sin contraseña previa): regenera el vault con keypair **nuevo**, re-envuelve el wrap personal, actualiza la pública vigente y cierra el rango anterior en `signing_keys` — las entries viejas siguen verificando con la clave que les corresponde por fecha.

### 4.5 Frase semilla (recovery, continuidad y break-glass)

* 6 palabras generadas en el dispositivo, mostradas una sola vez, **jamás persistidas en disco ni en la base**.
* Deriva únicamente `.data_key.master`; **no sustituye a la contraseña** en el login normal.
* **Rotación** (tab del administrador): generar → anotar → re-escribir para verificar → recién entonces `admin_rotate_seed` reenvuelve `.data_key.master`. Hasta el último paso la frase anterior sigue siendo válida, y la acción queda registrada en `audit_log`.
* **Break-glass** (`seed_recovery`, desde el enlace de login): sí permite recuperar el acceso cuando se olvidó **todas** las contraseñas. La frase se verifica contra `.data_key.master` (mismo `unwrap` del recovery de datos — poseer la frase ya es poder descifrar todo, así que este paso solo lo formaliza y audita). Se restablece la contraseña del administrador activo más antiguo con vault y keypair nuevos, se rota `signing_keys` (las firmas históricas siguen verificando por fecha) y la acción queda en `audit_log` con acción `seed_recovery`. La DataKey no cambia.

### 4.6 Custodia: roles y autorización en backend

`login_user` registra la sesión activa en `SessionState` (Rust) y `lock_vault` la limpia. Todo command sensible exige `require_admin()` —rol `administrador`— **del lado de Rust**, nunca confiando en flags del frontend:

* altas y bajas lógicas de usuarios (`admin_create_user`, `set_user_active`),
* resets de contraseña ajenos,
* rotación de la frase semilla,
* lectura del registro de auditoría (`audit_log`, con actor y objetivo).

El administrador **no gana lectura nueva de datos clínicos** (misma DataKey, mismo wrap que todos): su poder es sobre *cuentas y credenciales*, y cada uso queda auditado.

---

## ☁️ 5. Flujo de Sincronización Segura (Futuro: PocketBase)

Cuando se implemente el puente de sincronización, el servidor remoto actuará como un mero casillero de almacenamiento ciego:

1. **Validación de Canales:** Los datos viajarán sobre TLS (HTTPS), pero aunque el canal TLS fuese vulnerado (Man-in-the-Middle), los payloads ya viajan encriptados desde el cliente.
2. **Estructura del Payload Remoto:** El servidor en la nube solo almacenará:
    * UUIDs relacionales generados aleatoriamente.
    * Bloques de texto cifrado incomprensibles (Hexadecimal).
    * Nonces públicos.
    * El `blind_index` para indexación y búsquedas opacas.
3. **Filosofía Zero-Knowledge:** El proveedor de la nube (o cualquier atacante que acceda al servidor central) solo verá metadatos correlativos, haciendo imposible la reconstrucción de la historia clínica de un paciente o su identificación legal.

---

## 💾 6. Respaldo y Restauración Local

El mecanismo de respaldo no introduce claves nuevas: copia los archivos que **ya están cifrados** y les agrega una huella de integridad.

### Contenido de un respaldo (`respaldo-{timestamp}/`)

| Archivo | Rol en la recuperación |
| :--- | :--- |
| `historia_clinica.db` (con `wal_checkpoint(TRUNCATE)` previo) | Todos los datos (cifrados por campo/payload) |
| `.data_key.master` | Recuperación con la **frase semilla** |
| `.data_key.{user}` | Recuperación con la **contraseña** del usuario (fast path) |
| `vault_*.{bin,salt}` | Llaves de firma por usuario (si faltan, se regeneran al hacer login sin perder verificabilidad histórica) |
| `.installation_salt` | Sin ella los `blind_index` restaurados no coincidirían con los nuevos |
| `manifest.json` | `schema_version` + **SHA-256 por archivo** (integridad al restaurar) |

### Reglas

1. **Nada en claro**: el manifest solo contiene nombres, tamaños y hashes; todo lo demás es ciphertext envuelto. Un respaldo robado es tan inútil como la base sin contraseña ni frase.
2. **Validación antes de tocar nada**: `validate_backup` verifica cada SHA-256, rechaza `schema_version` más nuevo que la app y nombres con separadores de ruta (path traversal).
3. **Orden destructivo del restore**: validar → auditar (`backup_restored`) → cerrar llaves, sesión y conexión SQLite → copiar archivos → reabrir con `run_migrations` → recargar la sal de blind index. Si la copia falla, se reintenta abrir la base previa.
4. **Matriz de recuperación tras restaurar**:
   * Contraseña conocida → login normal → wrap personal (o semilla si no hay wrap) → todo.
   * Sin contraseñas → break-glass con frase semilla (§4.5).
   * Sin frase ni ninguna contraseña → los datos son irrecuperables por diseño (*crypto-shredding*): por eso el respaldo y la frase se guardan juntos y separados del equipo.
