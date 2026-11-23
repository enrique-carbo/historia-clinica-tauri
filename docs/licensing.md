# Licencias de módulos — idea (no implementada)

> **Estado: idea / propuesta, sin implementar.** La decisión vigente sigue
> siendo `docs/module-contract.md` §8.3 y §10: paywall build-time, sin
> verificación de licencia en runtime, sin revocación. Este documento solo
> describe un escenario posible para habilitar módulos de pago mediante un
> código entregado al cliente tras el pago. No cambia ningún comportamiento
> actual del sistema.

## 1. Motivación

Hoy la edición se resuelve compilando: el binario Free no incluye los módulos
de pago y el cliente que compra instala un binario distinto
(`module-contract.md` §10). Eso funciona, pero para cada venta hay que
preparar/entregar un artefacto distinto.

La idea es reemplazar esa entrega por una **licencia firmada**: el vendor
cobra el honorario y envía un código (o archivo) que el administrador instala
desde la propia app. El mismo binario de pago sirve para todos los clientes;
la licencia decide qué módulos puede encender cada instalación.

Sigue siendo un gate para el **cliente honesto**, no DRM: el monorepo es
público, el binario es local y quien quiera puede parchear la verificación. Lo
que se vende es el release firmado, las actualizaciones y el soporte. Ese
criterio ya está aceptado en `module-contract.md` §9.

## 2. Restricciones de producto acordadas para esta idea

- **Licencia perpetua**: un solo pago, sin vencimiento ni renovación.
- **Pago por módulo**: cada módulo de pago se compra por separado.
- **Módulos mixtos**: pueden coexistir módulos gratis y de pago.
- **Offline-first**: sin servidor de licencias, sin phone-home, sin
  telemetría. La verificación ocurre en Rust, en la máquina del cliente.
- **Sin PHI en la licencia**: es un dato de negocio, no clínico.

## 3. Diseño propuesto

### 3.1 Ediciones y builds

Dos builds por perfil (med/vet), no uno por combinación de módulos:

| Build | Contenido |
|---|---|
| **Free** (`--no-default-features`) | Core + módulos gratis |
| **Pro** (default) | Core + módulos gratis + **todos** los módulos de pago |

Los módulos gratis se compilan siempre; los de pago van detrás de una feature
de Cargo que solo habilita el build Pro. Agregar un módulo nuevo no crea una
edición nueva: solo suma un ID posible a las licencias.

### 3.2 Formato de la licencia

Un token firmado con Ed25519. La clave privada es del vendor; la pública va
embebida en la app, separada de `signing_keys` (que firma entries y backups).

```json
{
  "v": 1,
  "id": "lic_2026_0042",
  "customer": "Clínica San Martín",
  "modules": ["solicitudes"],
  "kid": "vendor-2026",
  "sig": "ed25519:..."
}
```

- No hay `expires_at`, ni período de gracia, ni reloj: la licencia es
  perpetua.
- `modules` es la lista de IDs de módulos **de pago** habilitados. Los módulos
  gratis no aparecen.
- `kid` permite rotar la clave del vendor sin romper licencias ya emitidas
  (la app lleva una lista de públicas).
- `customer` es solo metadata para soporte; no es PHI.
- El payload se serializa de forma canónica antes de firmarse, con el mismo
  criterio de `ManifestSignable` (`security/backup.rs`).

### 3.3 Acumulación de licencias

Como el pago es por módulo, un cliente puede comprar varios en momentos
distintos. Las licencias **se acumulan**: el entitlement efectivo es la unión
de los `modules` de todas las licencias válidas instaladas. No hace falta
reemitir una licencia combinada.

Se guardan en `app_local_data_dir/licenses.json` (array de tokens), en texto
plano, fuera de la DB clínica. Consecuencias:

- No hay migración de DB nueva.
- No viajan en los backups clínicos.
- Un restore o reinstall en otra máquina requiere volver a importar los
  tokens; el cliente los conserva (por email, por ejemplo).

### 3.4 Gratis vs. pago

En lugar de una lista hardcodeada de IDs, cada módulo declara su naturaleza
en su descriptor:

```rust
pub struct ModuleDescriptor {
    // ...campos actuales...
    pub paid: bool,
}
```

El gate genérico queda:

```text
disponible =
    instalado                    // descriptor en el binario
    && activo                    // fila en `modules`, state='active'
    && rol ∈ allowed_roles
    && (!paid || id ∈ licenciados)
```

Así, un módulo gratis se declara `paid: false` y no necesita licencia.

### 3.5 Dónde se aplica el gate

- `is_module_available(id)` → filtro de UI y del drawer (no es seguridad).
- `module_activate(id)` → exige `administrador` **y** entitlement para módulos
  de pago.
- `module_execute(...)` → el gate real; rechaza cualquier acción de un módulo
  de pago sin licencia válida.
- `module_deactivate(id)` → sigue libre, igual que hoy.

Instalar una licencia **no** activa módulos por sí sola: el admin sigue
activándolos desde "Módulos", como en el flujo actual
(`module-contract.md` §6.2). La licencia es un derecho; la activación es una
decisión operativa.

### 3.6 Emisión

La licencia la genera una CLI del vendor que no se distribuye:

1. Se cobra el honorario por el canal que sea (transferencia, plataforma de
   pagos, etc.).
2. Se emite un token firmado con el ID del módulo comprado y los datos de
   soporte.
3. Se entrega al cliente como archivo o texto para pegar.

No hay servidor de activación. Si más adelante se quiere un código corto
tipo `FICHIO-XXXX-XXXX`, haría falta un servicio que lo canjee por el token
firmado; recién ahí la app seguiría verificando offline. Queda fuera de esta
idea.

### 3.7 Auditoría

Instalar o quitar una licencia escribe eventos de metadata pura en
`audit_log` (por ejemplo `license_installed` con `license_id` y la lista de
módulos). Nunca el contenido del token ni datos clínicos.

## 4. Lo que esta idea NO incluye

- Vencimientos, renovaciones ni períodos de gracia.
- Device binding, límites de dispositivos ni límites de usuarios.
- Revocación remota.
- Phone-home, telemetría ni verificación online.
- Códigos cortos con secreto embebido en el binario (falsa seguridad).
- Un único binario con todos los módulos para todas las ediciones.

## 5. Cómo escala

| Escenario | Acción requerida |
|---|---|
| Nuevo módulo de pago | Feature de Cargo + `paid: true` + vender licencia con ese ID. Sin build por cliente. |
| Nuevo módulo gratis | `paid: false`, compilado en ambos builds. Sin licencia. |
| Nuevo cliente | Emitir un token con la CLI del vendor. Sin servidor. |
| Cliente que compra otro módulo | Instalar una segunda licencia; se unen los entitlements. |
| Volumen alto | Automatizar la emisión después; el formato no cambia. |

## 6. Impacto si se adopta

Sería un cambio de decisión, no un agregado inocuo:

- `docs/module-contract.md` §8.3 y §10 dejarían de ser vigentes y habría que
  reescribirlos.
- Backend: módulo `license` (verificar/guardar/entitlements), commands de
  instalación y consulta, campo `paid` en `ModuleDescriptor`, gates en
  `PluginHost` / `module_commands.rs`.
- Frontend: panel de administración para importar/listar licencias (o una
  sección dentro de "Módulos" con un badge de "requiere licencia").
- Tests: token válido, firma manipulada, clave equivocada, módulo no
  incluido, licencia duplicada, unión de varias licencias, módulo gratis sin
  licencia, y que el build Free siga sin código de módulos de pago.
- Sin migración de DB, porque el estado vive en un archivo.

## 7. Decisiones aún abiertas

- ¿El entitlement es por **ID de módulo** sin importar el perfil, o querés
  que `solicitudes` en Med y en Vet sean dos compras distintas? La opción
  simple es por ID.
- ¿Mantener `customer` en el token o dejarlo anónimo con solo `id` y
  `modules`? Conviene tenerlo para soporte.
- ¿Autoactivar el módulo al instalar la licencia o mantener el paso manual
  de "Módulos → Activar"? Se recomienda mantenerlo manual para no cambiar la
  semántica actual.

Cuando esta idea se convierta en decisión, este documento debe pasar a ser
normativo y `module-contract.md` debe actualizarse en consecuencia.
