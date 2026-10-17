# `docs/module-contract.md`
**Estado**: Borrador v1. Reemplaza a `PLUGIN_ARCHITECTURE.md` (que describía carga dinámica — descartada).
**Decisión vigente**: **Ruta 1** (módulos compilados con feature flags, paywall build-time) con el borde diseñado *remote-ready* para que una eventual Ruta 2 (proceso separado) sea un swap de transporte, no una reescritura.

---

## 1. Qué es un módulo (y qué no)

Un **módulo** es un crate Rust que agrega funcionalidad al Core sin modificarlo. Se compila dentro del binario, se activa en runtime por instalación, y se comunica con el Core a través de un contrato serializable.

Un módulo **no es**:

- Un plugin de terceros cargado en runtime (`.so`/`.dll`) — descartado por modelo de amenaza y por paywall.
- Una feature del Core disfrazada. Si algo es indispensable para identidad, bóveda o registro clínico básico, **es Core** y es gratis.
- Un perfil de producto. Med y Vet son builds del Core (`profiles/{med,vet}/`), no módulos.

**Criterio de corte** (Principio Rector, heredado del ahora retirado
`PLUGIN_ARCHITECTURE.md`):

> ¿Es indispensable para identidad, bóveda o registro clínico básico? Si no → módulo, cuando haga falta. Nunca al Core "por si acaso", ni como placeholder.

**Es Core**: sesión/usuarios/roles/auditoría, login/vault/seed/cambio de contraseña, criptografía/DataKey/respaldo/restore, pacientes (filiación), entries SOAP, perfil profesional, export `.md`.

**Es módulo** (cuando exista): teleconsulta y mensajería, solicitud de estudios, métricas/PGHD, export FHIR/HL7, terminología (LOINC/SNOMED), integración con obras sociales, IA/OCR, reportes avanzados.

**Modelo de dato**: lo clínico lo escribe el profesional en el Core con el flujo normal (`create_entry`, firmado, inmutable). El módulo **no** crea entries ni escribe registros clínicos: guarda solo su operativa (chat, estados, referencias a entries por id) en **su propia DB**. Primer caso: teleconsulta — chat en `fichio-teleconsulta.db`; la SOAP la escribe el profesional en el Core.

---

## 2. Principios

1. **El Core no conoce módulos.** Ni el código, ni los descriptores, ni el schema de sus DBs. El Core es genérico: entidades, entries, crypto, audit, API.
2. **Un módulo no toca el Core.** No abre `fichio-core.db`, no escribe `users`/`entries`/`signing_keys`, no ve la DataKey. Si necesita un dato del Core, lo pide por `ModuleContext`. Si necesita una operación nueva del Core, es una feature del Core o una extensión de la API — nunca SQL directo.
3. **El paywall es build-time.** Un módulo comercial no está compilado en el binario Free. No hay "binario full desbloqueado en runtime" (extraíble).
4. **El borde está diseñado remote-ready.** Todo input/output del módulo es `Serialize + Deserialize` desde el día 1, aunque la ejecución sea in-process. Eso hace que Ruta 2 sea un cambio de transporte, no de módulos.
5. **La activación es operativa, no de seguridad.** La tabla `modules` decide qué está encendido en una instalación. No es una frontera criptográfica: un admin técnico con el binario adecuado puede encender lo que tenga compilado. La frontera es qué código está en el binario.
6. **El actor viene de la sesión.** `module_execute` deriva actor (user_id, rol, entity) con `require_session()` — nunca del request. El `entity_id` del payload es una *referencia blanda*: para un actor `paciente`, el Core solo resuelve su propia entity (`users.entity_id`).

---

## 3. El trait `Module`

```rust
// core::module

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleRequest {
    /// Versión del protocolo de mensajes. Hoy 1. Se incrementa solo si
    /// cambia la forma de ModuleRequest/ModuleResponse/ModuleContext.
    pub protocol_version: u16,
    /// Acción solicitada. String libre, el módulo la parsea.
    pub action: String,
    /// Entidad sobre la que opera, si aplica (referencia blanda).
    pub entity_id: Option<i64>,
    /// Payload específico de la acción. El módulo valida.
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleResponse {
    pub payload: serde_json::Value,
}

#[async_trait]
pub trait Module: Send + Sync + 'static {
    fn descriptor(&self) -> ModuleDescriptor;

    /// Migración de la DB propia del módulo. El host la llama en la
    /// activación y en cada arranque con el módulo activo. Idempotente.
    fn migrate(&self, conn: &mut rusqlite::Connection) -> Result<(), ModuleError>;

    /// Único punto de entrada. Serialización de ida y vuelta obligatoria.
    /// En Ruta 1 el host llama in-process; en Ruta 2 serializa y manda por IPC.
    async fn handle(
        &self,
        req: ModuleRequest,
        ctx: &dyn ModuleContext,
    ) -> Result<ModuleResponse, ModuleError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDescriptor {
    pub id: String,
    pub name: String,
    pub version: String,          // informativo, no semver runtime
    pub scope: ModuleScope,
    /// Roles que pasan el gate, explícitos (fail-closed: lista vacía = nadie).
    /// Ej. teleconsulta: todos los roles incluido Paciente.
    pub allowed_roles: Vec<ActorRole>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActorRole {
    Administrador,
    Asistente,
    Profesional,
    Paciente,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleScope {
    /// Activación a nivel instalación. Todo el staff accede.
    Installation,
    /// Activación a nivel instalación + acceso explícito por profesional.
    InstallationAndProfessional,
}
```

**Reglas del trait**:

- **No hay métodos por acción** (`get_filiation`, `send_message`, etc. son `action` strings, no métodos del trait). Agregar una acción no cambia el trait. Eso es lo que sobrevive a Ruta 2.
- **`protocol_version` desde el día 1**. En Ruta 1 no se usa; en Ruta 2 el host rechaza versiones que no entiende. Agregarlo después es un breaking change del struct.
- **`migrate` es síncrono y recibe `&mut Connection` de la DB del módulo**, no del Core. Es la única función que ve una conexión, y es la propia.

---

## 4. `ModuleContext` — el acceso al Core

`ModuleContext` es un **trait**, no un struct concreto. Eso permite:

- En Ruta 1: implementación in-process que llama a la `CoreApi`.
- En Ruta 2: implementación proxy que serializa al host.
- En tests: mock que devuelve fixtures sin levantar el Core.

```rust
// core::module

#[async_trait]
pub trait ModuleContext: Send + Sync {
    /// Filiación proyectada del paciente. Descifra internamente.
    /// Actor `paciente`: solo la propia entity (`users.entity_id`) —
    /// scoping garantizado por el Core, no por el módulo.
    async fn fetch_filiation(&self, entity_id: i64)
        -> Result<FiliationDto, ApiError>;

    async fn entity_exists(&self, entity_id: i64)
        -> Result<bool, ApiError>;

    /// Perfil del profesional (nombre, matrícula, pública Ed25519).
    async fn fetch_professional(&self, user_id: &str)
        -> Result<ProfessionalDto, ApiError>;

    /// Auditoría. Enum cerrado, sin PHI.
    async fn audit(&self, actor: &str, event: AuditEvent)
        -> Result<(), ApiError>;

    /// Crypto opt-in (decidido v1: el chat de teleconsulta lo necesita).
    async fn wrap_with_datakey(&self, plaintext: &[u8])
        -> Result<Vec<u8>, ApiError>;
    async fn unwrap_with_datakey(&self, ciphertext: &[u8])
        -> Result<Zeroizing<Vec<u8>>, ApiError>;

    /// Wrap/unwrap contra la master wrap de un usuario — thread keys
    /// (v1: solo targets `role: 'paciente'`). El módulo nunca ve claves:
    /// el Core gate-audita y envuelve.
    async fn wrap_for_user(&self, user_id: &str, plaintext: &[u8])
        -> Result<Vec<u8>, ApiError>;
    async fn unwrap_for_user(&self, user_id: &str, ciphertext: &[u8])
        -> Result<Zeroizing<Vec<u8>>, ApiError>;

    /// HTTP mediado. El módulo no usa reqwest directo.
    async fn http_fetch(&self, request: HttpRequest)
        -> Result<HttpResponse, ApiError>;
}
```

**DTOs en el Core** (`core::api::dto`), no en cada módulo:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiliationDto {
    pub entity_id: i64,
    pub full_name: String,
    pub document: String,
    pub birth_date: Option<String>,   // ISO 8601
    pub sex: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfessionalDto {
    pub user_id: String,
    pub full_name: String,
    pub license_number: String,
    pub specialty: Option<String>,
    pub public_key_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEvent {
    SolicitudIssued { subject_id: i64, solicitud_id: String, sha256: String },
    SolicitudCompleted { subject_id: i64, solicitud_id: String },
    TeleconsultaMessageSent { entity_id: i64, conversation_id: String },
    TeleconsultaPaymentConfirmed { entity_id: i64, conversation_id: String, amount_cents: i64 },
    ModuleActivated { module_id: String },
    ModuleDeactivated { module_id: String },
    // variantes nuevas: PR que revisa que no lleven PHI
}
```

**Reglas**:

- **Proyecciones acotadas**, no filas completas. Un módulo recibe lo que un documento impreso necesita, no `enc_data_blob`.
- **`AuditEvent` es enum cerrado**. Un módulo no pasa `detail: &str` libre. Eso impide PHI accidental en el audit. El test anti-PHI (`"Pérez"`, `".md"`) se extiende a todas las variantes.
- **`http_fetch` es el único camino a red**. El Core audita **solo host+path** de la URL (nunca query string: puede llevar tokens o PHI), aplica allowlist por módulo (config), y puede negarse. Un módulo con `reqwest` directo en su `Cargo.toml` es un bug de revisión — el `Cargo.toml` del módulo no debe tener `reqwest`; usa `ctx.http_fetch`.
- **Evolución del trait**: métodos nuevos llevan `default` que devuelve `Err(Unsupported)`. Aditivo, no rompe implementaciones proxy (Ruta 2) ni mocks de test.

---

## 5. Dispatch — un solo command Tauri

```rust
#[tauri::command]
async fn module_execute(
    app: AppHandle,
    module_id: String,
    req: ModuleRequest,
) -> Result<ModuleResponse, String> {
    let host = app.state::<PluginHost>();
    host.dispatch(&module_id, req).await.map_err(|e| e.to_string())
}
```

**No hay `#[tauri::command]` por acción de módulo.** Todo el frontend habla con `module_execute`. Eso:

- Mantiene **una sola superficie IPC** para módulos.
- Permite que Ruta 2 cambie el `PluginHost` sin tocar el frontend.
- Centraliza el gate (`is_module_installed ∧ is_module_active ∧ tiene_access`) en un lugar.

**Orden del gate** (en `PluginHost::dispatch`):

1. `descriptor` presente en el binario? Si no → `ApiError::ModuleNotInstalled`.
2. Fila en `modules` con `state='active'`? Si no → `ApiError::ModuleNotActive`.
3. Rol del actor (derivado de `require_session()`, nunca del request) ∈ `descriptor.allowed_roles`? Si no → `ApiError::ModuleRoleDenied`.
4. Si `scope == InstallationAndProfessional`: el actor tiene fila en `module_access`? Si no → `ApiError::ModuleAccessDenied`.
5. Construir `ModuleContext` con el actor de la sesión.
6. `module.handle(req, ctx).await`.

**Nunca al revés**: el gate corre **antes** que cualquier chequeo de rol de dominio, para no filtrar información a usuarios no autorizados. Regla dura: el actor **siempre** se deriva de la sesión — `module_execute` no acepta user_id ni rol del frontend.

---

## 6. Ciclo de vida

### 6.1 Instalación inicial

- El binario trae N módulos compilados (feature flags).
- Al primer arranque, el Core corre sus migraciones (`user_version`).
- La tabla `modules` se crea vacía. **Ningún módulo se activa solo.**
- El admin ve en "Módulos" la lista de los compilados, todos `state='inactive'`.

### 6.2 Activación

`module_activate(module_id)` (solo `administrador`):

1. Verifica que `descriptor` exista en el binario. Si no → `ModuleNotInstalled`.
2. Abre la DB del módulo (lazy): `app_local_data_dir/fichio-<id>.db`.
3. Corre `migrate()` en transacción. Si falla → rollback, `ModuleError`, **no se activa**.
4. Upsert en `modules`: `state='active'`, `activated_at`, `activated_by`.
5. `audit(ModuleActivated { module_id })`.

### 6.3 Desactivación

`module_deactivate(module_id)` (solo `administrador`):

1. `state='inactive'`. **La DB del módulo no se borra.**
2. `module_access` no se toca (reactivar preserva accesos).
3. `audit(ModuleDeactivated { module_id })`.
4. La conexión se cierra (si estaba abierta).

**Desactivar es "pausar y ocultar", no "borrar".** Si el admin reactiva, los datos están.

### 6.4 Upgrade de edición (instalar binario con más módulos encima)

1. Usuario instala `Fichio Médico` sobre `Fichio Free`. El instalador reemplaza el binario, **preserva `app_local_data_dir`** (mismo `identifier`).
2. Primer arranque: migraciones del Core (no-op si ya estaba al día).
3. `PluginHost` encuentra el nuevo descriptor en el binario.
4. `modules` no tiene fila → `state='uninstalled'` implícito (o se inserta al detectar). Admin activa → flujo §6.2.
5. La DB del módulo se crea en la primera activación, no antes. Sin estado huérfano.

### 6.5 Downgrade de edición (instalar binario con menos módulos)

1. El nuevo binario no tiene el descriptor.
2. `modules` tiene una fila con `module_id` que no está en el binario → se marca `state='uninstalled'` al arranque (o se ignora con warning). **La DB del módulo sigue en disco.**
3. La UI no muestra el módulo. Los datos quedan por si se reactiva (upgrade de nuevo).

### 6.6 Estado en la DB

Migración de activación de módulos — **versión a determinar al implementar**
(hoy el catálogo está en `v001`; fase 4 reserva `v002` para su M1 según
`fase4-diseno-sync.md` — las tablas de módulos toman la siguiente versión
libre; append-only):

```sql
CREATE TABLE modules (
    id TEXT PRIMARY KEY NOT NULL,
    state TEXT NOT NULL DEFAULT 'inactive'
        CHECK(state IN ('inactive','active','uninstalled')),
    activated_at TEXT,
    activated_by TEXT,
    FOREIGN KEY(activated_by) REFERENCES users(id) ON DELETE SET NULL
);

CREATE TABLE module_access (
    module_id TEXT NOT NULL,
    user_id   TEXT NOT NULL,
    granted_at TEXT NOT NULL,
    granted_by TEXT NOT NULL,
    PRIMARY KEY (module_id, user_id),
    FOREIGN KEY(module_id) REFERENCES modules(id) ON DELETE CASCADE,
    FOREIGN KEY(user_id)   REFERENCES users(id)   ON DELETE CASCADE,
    FOREIGN KEY(granted_by) REFERENCES users(id)  ON DELETE RESTRICT
);
```

`config` por módulo **no** se incluye en v1. Si un módulo necesita config, la guarda en su propia DB. Eso mantiene el Core agnóstico.

---

## 7. Aislamiento de la DB del módulo

- **Archivo**: `app_local_data_dir/fichio-<module_id>.db`. WAL habilitado, `PRAGMA user_version` propio.
- **Conexión**: la abre y la cierra el `PluginHost` en activación/desactivación. El módulo recibe `&mut Connection` solo en `migrate`; en `handle` recibe un handle opaco (`ModuleDb`) que el módulo usa con su propia API.
- **Backup**: `create_backup` respalda **todos** los `fichio-*.db` presentes en disco, independientemente del estado. El manifest incluye un hash por archivo. El estado de activación viaja en la DB del Core (también respaldada), así que restore preserva el estado.
- **Downgrade de schema del módulo**: si la DB del módulo tiene `user_version` mayor que las migraciones del binario, la activación **falla explícita** (`ModuleError::SchemaTooNew`). Mismo guard que el Core.

---

## 8. Seguridad y límites

### 8.1 Lo que el contrato garantiza

- **El módulo no abre `fichio-core.db`.** La conexión al Core es privada de `PluginHost`; el módulo no la recibe.
- **El módulo no ve la DataKey.** Recibe datos descifrados en proyecciones acotadas. `wrap_with_datakey` es una función, no la llave.
- **El módulo no escribe tablas del Core.** La única escritura es `ctx.audit(...)` con `AuditEvent` cerrado.
- **El módulo no hace HTTP directo.** Usa `ctx.http_fetch`, que audita y aplica allowlist.

### 8.2 Lo que el contrato **no** garantiza (honestidad)

- **Aislamiento de memoria.** En Ruta 1, el módulo corre en el mismo proceso que el Core. Un módulo malicioso o con `unsafe` puede leer memoria del proceso. La frontera es **de diseño y de revisión de código**, no criptográfica. Ruta 2 (proceso separado) sí aislaría memoria, pero no es el estado actual.
- **Confidencialidad entre módulos.** Dos módulos pueden leer la misma entidad vía `ModuleContext`. El modelo es "staff de la instalación", no "módulo A no ve lo de módulo B".
- **Que el módulo sea de confianza por ser "interno".** Los módulos son código del vendor, revisado en PR. Un módulo de un contribuidor externo requiere el mismo nivel de revisión que el Core.

### 8.3 Paywall

El paywall es **build-time**: un módulo comercial no está en el binario Free. No hay verificación de licencia en runtime, no hay revocación, no hay `plugins.json`. Si en el futuro se necesita licencia runtime (Ruta 2 con plugins de terceros), es un proyecto aparte.

---

## 9. Frontend

**Decisión v1: UI de módulos en el repo del Core, renderizada condicionalmente.**

- El bundle JS incluye componentes de UI de todos los módulos (peso aceptable para 1–3 módulos).
- El `Drawer` y las vistas filtran por `is_module_available(id)`:

```ts
// el backend expone esta función; el frontend la consume.
// UX solamente — el gate de verdad es PluginHost::dispatch.
is_module_available(id) =
    is_module_installed(id)   // descriptor en el binario
    && is_module_active(id)   // fila en modules con state='active'
    && role ∈ descriptor.allowed_roles
    && (scope === 'installation' || role === 'paciente' || has_module_access(id, userId))
```

- Las acciones del módulo llaman **siempre** a `module_execute`:

```ts
await invoke("module_execute", {
  moduleId: "teleconsulta",
  req: { protocol_version: 1, action: "send_message", entity_id, payload: {...} },
});
```

- **Nunca** `#[tauri::command]` por módulo en el frontend. Eso es lo que permite migrar a Ruta 2 sin tocar la UI.

**Migración a Opción B** (UI por módulo en paquete npm, `import.meta.glob` filtrado por edición) queda para cuando el **repo Core se espeeje público** (la UI de módulos pagos no puede vivir en un repo público — gatillo principal), o cuando el bundle pese o haya ≥3 módulos. No en v1.

---

## 10. Distribución — ediciones

| Edición | Core (med+vet) | teleconsulta | solicitudes | otros |
|---|:---:|:---:|:---:|:---:|
| **Fichio Free** | ✅ | — | — | — |
| **Fichio Médico** | ✅ | ✅ | ✅ | — |
| **Fichio Vet** | ✅ | ✅ | — | [vet-specific] |
| **Fichio Full** | ✅ | ✅ | ✅ | ✅ |

- **Una sola base de código**, N builds por feature flags de Cargo.
- **Med y Vet no son ediciones comerciales**: son builds del Core (`FICHIO_PROFILE=med|vet`). Coexisten en la misma máquina (distinto `identifier` → distinto `app_local_data_dir`).
- **El vendor controla los builds** de las ediciones pagas. Los módulos viven en repos privados (o en un monorepo privado con el Core público como espejo).
- **El usuario que compra un módulo instala un binario nuevo encima**. No hay "descarga de plugin" sin reinstalar la app. Es una limitación aceptada: las actualizaciones clínicas son raras y controladas.
- **AGPL × ediciones**: el paywall depende de ser el **único copyright holder** del Core y de los módulos (dual licensing). Contribuciones externas al Core obligan a revisar la distribución de binarios con módulos privados antes de aceptarlas.

---

## 11. Ruta 1 → Ruta 2 (por qué el borde está diseñado así)

**Ruta 1 (actual)**: módulos compilados en el binario, `PluginHost` in-process, `ModuleContext` implementado directamente sobre la `CoreApi`.

**Ruta 2 (futura, solo si hay demanda de terceros o update sin reinstalar)**: el `PluginHost` lanza un proceso separado por módulo y hace de proxy del `ModuleRequest`/`ModuleResponse` por IPC (stdin/stdout JSON-RPC o Unix socket).

**Qué cambia en Ruta 2**:
- `PluginHost` gana un modo "external" que lanza procesos.
- `ModuleContext` gana una implementación proxy que serializa.
- El `Cargo.toml` del binario deja de incluir el crate del módulo, pero lo incluye el instalador como binario separado.

**Qué NO cambia**:
- El trait `Module` (misma firma).
- `ModuleRequest`/`ModuleResponse`/DTOs (ya son `Serialize + Deserialize`).
- El frontend (`module_execute` es el único entry point).
- La tabla `modules` y `module_access`.
- Los módulos en sí (solo se mueven a otro proceso).

**Condición para que esto sea un swap y no una reescritura** (la disciplina que hay que respetar desde hoy):

1. Todo input/output del módulo es `Serialize + Deserialize`. Sin excepciones.
2. El módulo **nunca** recibe `&Connection` del Core ni la DataKey.
3. El frontend **nunca** usa un command tipado por módulo. Solo `module_execute`.
4. HTTP solo vía `ctx.http_fetch`.
5. `protocol_version` en `ModuleRequest` desde el día 1.

Si se rompe cualquiera de las cinco, Ruta 2 reescribe esa parte. Con las cinco, Ruta 2 son semanas de `PluginHost`, no meses por módulo.

---

## 12. Testing

### Del Core
- `PluginHost::dispatch`: gate en orden correcto, `ModuleNotInstalled`/`ModuleNotActive`/`ModuleRoleDenied`/`ModuleAccessDenied` distinguidos, `VaultLocked` propagado.
- Gate de rol: descriptor sin `Paciente` → actor paciente denegado; con `Paciente` → pasa al handle.
- `fetch_filiation` con actor `paciente`: propia entity → ok; entity ajena → `AccessDenied` (no `EntityNotFound`: no se filtra existencia).
- `AuditEvent::to_detail()`: cada variante, sin PHI (fixtures con nombres, check explícito).
- `wrap_with_datakey`/`unwrap_with_datakey`: roundtrip, key incorrecta, ciphertext corrupto.
- `wrap_for_user`: roundtrip contra master wrap; target no-paciente → rechazado.
- `http_fetch`: la URL auditada incluye solo host+path (fixture con `?token=` → el query no aparece en el detail).
- `create_backup`: con 0, 1, N DBs de módulo; todas presentes en el manifest con su hash.

### De un módulo de prueba (en `#[cfg(test)]`)
- Flujo end-to-end con `PluginHost` in-process: activar → `handle` → auditar → desactivar.
- `VaultLocked` manejado en cada acción que use `fetch_filiation`.
- `EntityNotFound` manejado sin panic.
- Idempotencia de `migrate` (dos llamadas seguidas, mismo resultado).
- `SchemaTooNew` rechazado.

### De contrato (compile-time)
- `ModuleRequest`/`ModuleResponse`/DTOs derivan `Serialize + Deserialize` (test trivial que falla la compilación si se rompe).
- El `Cargo.toml` del módulo **no** incluye `reqwest` (test de CI que greppea el manifest).

### Del frontend
- `is_module_available` respeta rol (`allowed_roles`), scope y acceso.
- Las acciones usan `module_execute`, nunca un command tipado por módulo (test que greppea el código fuente por `invoke("teleconsulta_` y falla).

---

## 13. Lo que este contrato NO hace

- ❌ Carga dinámica de `.so`/`.dll` (terceros, runtime).
- ❌ `create_entry` (u otra escritura clínica) para módulos — el profesional escribe en el Core con el flujo normal; el módulo solo referencia entries por id.
- ❌ `plugins.json` como config de carga (reemplazado por feature flags + tabla `modules`).
- ❌ `PluginInput`/`PluginOutput` con `serde_json::Value` como contrato principal — el contrato es `ModuleContext` tipado; el `payload` interno de cada acción es libre pero **entre** módulos y Core viaja por DTOs.
- ❌ Verificación de licencia en runtime (paywall = build-time).
- ❌ Sync de módulos (cada módulo tiene su propia cola; el engine de Fase 4 itera módulos).
- ❌ Extensibilidad de terceros (sin proceso separado, sin sandbox de memoria).
- ❌ Render de documentos en el Core (cada módulo renderiza su HTML/Markdown; el Core provee datos).

---

## 14. Pendientes (marcados, no bloquean v1)

- ✅ **`wrap_with_datakey` en v1**: decidido — el chat de teleconsulta lo necesita. Se suma `wrap_for_user`/`unwrap_for_user` (thread keys lado paciente).
- ⚠️ **CoreApi (prerequisite Ruta 1)**: refactorizar `commands/` en una capa de servicio `CoreApi` sobre la que se implemente `ModuleContext` in-process. Hoy no existe (0 referencias en el código) — es el trabajo real previo al primer módulo.
- ⚠️ **Monorepo vs. repos separados**: para un solo dev, monorepo privado con `core/` + `modules/` es más simple. `core/` se espeja a repo público cuando haya algo que publicar. Decidir antes de crear el primer crate de módulo.
- ⚠️ **Nombre**: `modules` + `Module` (recomendado) vs. `plugins` + `Plugin` (vocabulario de clientes). Cosmético, pero se propaga. Recomendación: `Module` en código, "Plugin" en UI de cara al usuario si se prefiere.
- ⚠️ **Allowlist de `http_fetch`**: dónde vive la config de URLs permitidas por módulo. Recomendación: `config/http_allowlist.json` en el repo del Core, embebido, editable por PR. No en la DB (sería config de seguridad en un lugar mutable).
- ⚠️ **UI de "Módulos" en Admin**: confirmar que el tab es solo `administrador` (no `asistente`). Recomendación: sí, porque activar un módulo es una decisión técnica/contractual.
- ✅ **Retiro de `PLUGIN_ARCHITECTURE.md`**: resuelto en este cambio (borrado; referencias en AGENTS.md actualizadas).

---

## 15. Resumen en una frase

**Core mínimo y gratis; módulos compilados con feature flags, activados por instalación, comunicados por un trait serializable y un único `module_execute`; el borde tiene forma de proceso separado desde el día 1 para que Ruta 2 sea un swap, no una reescritura; el paywall es build-time, no runtime.**
