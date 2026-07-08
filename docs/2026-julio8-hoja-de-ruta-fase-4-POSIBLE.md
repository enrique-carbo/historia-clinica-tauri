# 🗺️ Hoja de Ruta Definitiva — Fase 4: Sincronización Híbrida
## Simplex Health Core | Policonsultorio Multi-Dispositivo
## Versión 3.0 — 2026-07-08 | Notas firmadas = inmutables

---

## 0. Principios Rectores

> "Una nota firmada es un hecho clínico. Los hechos no se editan. Se corrigen con nuevos hechos."

1. **Offline por defecto, sync como optimización.** La app funciona 100% sin red. El sync es background, no bloqueante.
2. **El servidor es un tubo ciego.** Nunca descifra. Solo almacena blobs cifrados, blind indexes y metadatos de sync.
3. **Sin resolución de conflictos.** Las notas firmadas son inmutables. No hay "versión ganadora". Todas las notas coexisten, ordenadas por tiempo.
4. **Un consultorio = una unidad de confianza.** Todos los dispositivos del mismo consultorio comparten los mismos secretos.
5. **La identidad del médico es su responsabilidad.** El vault del médico es su llave física. El sistema no la distribuye.

---

## 1. Pre-requisitos: Core Fortalecido (Fase 3.5)

Tareas bloqueantes para la Fase 4.

| # | Tarea | Justificación |
|---|-------|---------------|
| 3.5.1 | Cifrar `.data_key` en disco con password del admin | Robo físico del disco no descifra todo |
| 3.5.2 | Corregir `get_notes_by_entity`: mostrar todas las notas, indicar si es corrección de otra | Las notas firmadas son inmutables; el historial es lineal |
| 3.5.3 | Agregar `previous_note_id` opcional a la tabla `notes` | Permite encadenar correcciones sin editar |
| 3.5.4 | Alimentar `sync_queue` desde TODOS los comandos de escritura | Sin esto, el sync engine no tiene trabajo |
| 3.5.5 | Agregar índices faltantes: `idx_notes_entity_id`, `idx_entities_blind_index` | Performance para sync de grandes volúmenes |
| 3.5.6 | Implementar múltiples blind indexes por entidad | Búsqueda server-side sin descifrado masivo |
| 3.5.7 | Zeroize de claves sensibles en RAM (`zeroize` crate) | Cuando `lock_vault` limpia, garantizar borrado seguro |

---

## 2. Modelo de Datos: Inmutabilidad de Notas Firmadas

### Regla fundamental

> **Una nota con firma digital NO se edita. NO se borra. Solo se corrige con una nueva nota que referencia a la anterior.**

### Estructura de la tabla `notes`

| Campo | Tipo | Propósito |
|-------|------|-----------|
| `id` | `INTEGER PRIMARY KEY AUTOINCREMENT` | Índice local |
| `external_id` | `TEXT UUID UNIQUE` | Identificador global |
| `entity_id` | `INTEGER NOT NULL` | Paciente vinculado |
| `template_id` | `TEXT NOT NULL` | Tipo de nota (SOAP, evolución, etc.) |
| `created_by_user_id` | `TEXT NOT NULL` | Médico que firmó |
| `enc_fields` | `BLOB NOT NULL` | Campos cifrados con DataKey |
| `signature` | `BLOB NOT NULL` | Firma Ed25519 del payload |
| `previous_note_id` | `INTEGER NULL` | Referencia a nota corregida (opcional) |
| `created_at` | `TEXT NOT NULL` | Timestamp de creación (inmutable) |

### Flujo de corrección

```
Nota 1 (id=1, previous_note_id=NULL, signature=AAA, created_at=10:00)
  "Diagnóstico: gripe"

Nota 2 (id=2, previous_note_id=1, signature=BBB, created_at=10:15)
  "Corrección de Nota 1: Diagnóstico: neumonía"

Nota 3 (id=3, previous_note_id=NULL, signature=CCC, created_at=11:00)
  "Nueva evolución: mejoría"
```

### Implicaciones para sync

| Escenario | Resultado |
|-----------|-----------|
| Dos médicos crean notas simultáneamente | Ambas existen. Orden por `created_at`. |
| Médico crea nota offline, otro online | Ambas existen. No hay conflicto. |
| Nota "corregida" en Device A, original en Device B | Ambas existen. La UI muestra la cadena de correcciones. |
| Nota con `previous_note_id` que aún no llegó | La UI muestra "corrección pendiente" hasta que llegue la original. |

**No se necesita resolución de conflictos.** Solo ordenamiento cronológico.

---

## 3. Infraestructura Remota

### 3.1 VPS + Dokploy

| Paso | Acción | Entregable |
|------|--------|------------|
| 3.1.1 | Contratar VPS (Hetzner, DigitalOcean, etc.) | IP pública, SSH accesible |
| 3.1.2 | Instalar Dokploy (PaaS self-hosted) | Panel web en `https://dokploy.simplexhealth.io` |
| 3.1.3 | Configurar reverse proxy (Traefik) + Let's Encrypt | HTTPS automático |
| 3.1.4 | Configurar firewall: solo HTTPS (443), SSH (22) | Reducción de superficie de ataque |

### 3.2 PocketBase como Backend de Sync

| Paso | Acción | Entregable |
|------|--------|------------|
| 3.2.1 | Desplegar PocketBase como servicio Dokploy | Contenedor Docker corriendo |
| 3.2.2 | Configurar colecciones | Schema de PocketBase listo |
| 3.2.3 | Configurar autenticación: admin con TOTP | Acceso seguro al panel |
| 3.2.4 | Configurar backups automáticos diarios a S3/MinIO | Resiliencia de datos |

### 3.3 Colecciones en PocketBase

```javascript
// sync_entities — el servidor solo almacena, nunca descifra
{
  external_id: "uuid-v4",           // PK
  entity_type: "paciente",
  blind_index: "sha256-hex",        // Para búsqueda server-side
  enc_data_blob: "base64",          // Blob cifrado con DataKey
  created_at: "ISO8601",
  updated_at: "ISO8601",
  deleted: false                    // Soft delete para sync
}

// sync_notes — notas inmutables, sin conflicto
{
  external_id: "uuid-v4",           // PK
  entity_id: "uuid-v4 (FK)",
  template_id: "soap",
  enc_fields: "base64",             // Blob cifrado con DataKey
  signature: "hex-64",              // Ed25519 signature
  created_by_user: "user-uuid",
  previous_note_id: "uuid-v4|null", // Referencia a nota corregida
  created_at: "ISO8601",            // Inmutable
  deleted: false                    // Soft delete (no borrar físicamente)
}

// onboarding_codes — puente temporal, expira en 10 minutos
{
  code: "7A3B9F",                   // PK, 6 caracteres alfanuméricos
  installation_id: "uuid-v4",     // Identifica el consultorio
  encrypted_blob: "base64",         // Secrets cifrados del consultorio
  expires_at: "ISO8601",            // Timestamp de expiración
  used: false                       // True después de descarga
}
```

> **Nota:** La colección `onboarding_codes` es temporal. Los códigos expiran y se borran. El servidor nunca acumula secrets permanentemente.

---

## 4. Sync Engine Local

### 4.1 Arquitectura del Worker

```
┌─────────────────┐                    ┌─────────────────┐
│  DISPOSITIVO A  │                    │   SERVIDOR PB   │
│  (Consultorio)  │                    │  (Dokploy/VPS)  │
└────────┬────────┘                    └────────┬────────┘
         │                                      │
         │  1. PUSH: sync_queue → HTTPS POST    │
         │ ─────────────────────────────────────► │
         │     {entities: [...], notes: [...]}    │
         │                                      │
         │  2. Server: INSERT en PB               │
         │     (solo metadatos + blobs cifrados)  │
         │                                      │
         │  3. PULL: GET ?since=timestamp         │
         │ ◄───────────────────────────────────── │
         │     {entities: [...], notes: [...]}      │
         │                                      │
         │  4. Device: INSERT OR IGNORE local       │
         │     (sin descifrar, solo persistir)      │
         │                                      │
```

### 4.2 Reglas del Sync Engine

| Regla | Descripción |
|-------|-------------|
| **Push** | Leer `sync_queue` local, enviar al servidor, eliminar de la cola local. |
| **Pull** | Pedir registros con `created_at` > último sync. Insertar localmente con `INSERT OR IGNORE`. |
| **Notas** | No hay UPDATE ni DELETE físico. Solo INSERT. Las notas son inmutables. |
| **Entities** | Soft delete: `deleted = true` en lugar de borrar físicamente. |
| **Frecuencia** | Cada 60 segundos si hay conexión. Si no hay conexión, la app funciona normalmente. |

### 4.3 Pseudocódigo del Worker

```rust
loop {
    sleep(60s);

    if !is_online() { continue; }

    // PUSH: enviar cambios locales
    let pending = db.query("SELECT * FROM sync_queue ORDER BY id");
    for record in pending {
        server.post("/api/sync", record);
        db.execute("DELETE FROM sync_queue WHERE id = ?", record.id);
    }

    // PULL: recibir cambios del servidor
    let last_sync = db.query("SELECT MAX(created_at) FROM sync_log");
    let incoming = server.get("/api/sync?since=" + last_sync);
    for record in incoming {
        db.execute("INSERT OR IGNORE INTO entities/notes (...) VALUES (...)");
    }

    // Actualizar timestamp de último sync
    db.execute("INSERT INTO sync_log (timestamp) VALUES (datetime('now'))");
}
```

---

## 5. Onboarding por Código de Un Solo Uso

### 5.1 Principio

> Dos personas, una pantalla, un código hablado. El servidor hace de puente temporal. Los secrets nunca se quedan en el servidor permanentemente.

### 5.2 Flujo

**Paso 1: Dispositivo maestro genera código**

```
Admin: "Quiero agregar una laptop"
App: Genera código 7A3B9F
Admin: "El código es 7A3B9F"
```

**Paso 2: Dispositivo nuevo ingresa código**

```
Dr. García: "Quiero unirme al consultorio"
App: "Ingresá el código de 6 caracteres"
Dr. García: "7A3B9F"
App: "Descargando configuración..."
```

**Paso 3: El servidor hace de puente**

```
Servidor: Recibe blob cifrado del maestro, etiquetado con 7A3B9F
Servidor: Recibe solicitud del nuevo con 7A3B9F
Servidor: Entrega el blob. Marca código como usado. Borra en 10 minutos.
```

**Paso 4: El dispositivo nuevo se configura**

```
App: Pide password del admin
Dr. García: [ingresa password]
App: Descifra blob con password. Instala .data_key, .installation_salt.
App: Listo para usar.
```

### 5.3 Seguridad del flujo

| Amenaza | Mitigación |
|---------|-----------|
| Código interceptado | El código solo sirve con el password del admin. Sin password, el blob es inútil. |
| Servidor comprometido | El servidor solo tiene blobs cifrados temporales. No tiene el password del admin. |
| Código adivinado | 6 caracteres alfanuméricos = ~2.000 millones de combinaciones. Expira en 10 minutos. |
| Replay attack | El código se marca como `used` después del primer uso. No se puede reutilizar. |

### 5.4 Lo que viaja en el blob cifrado

```
.data_key           ← Secreto del consultorio (cifrado con password del admin)
.installation_salt   ← Secreto del consultorio (cifrado con password del admin)
.installation_id    ← Identidad del consultorio (texto plano)
```

El password del admin nunca viaja. El dispositivo nuevo lo pide al usuario.

---

## 6. Identidad del Médico: Vault como Llave Física

### 6.1 Principio

> El vault del médico es su llave personal. El sistema no la distribuye. El médico la lleva consigo como llevaría una llave física.

### 6.2 Qué contiene el vault

```
vault_{user_id}.salt    ← Sal para derivar master key (única por usuario)
vault_{user_id}.bin     ← Llave privada Ed25519 cifrada con master key
password                ← Lo que el médico sabe (nunca se guarda)
```

### 6.3 Cómo se mueve

| Escenario | Acción del médico | Resultado |
|-----------|-------------------|-----------|
| Trabajar en consultorio | Vault en la PC del consultorio | Funciona normalmente |
| Trabajar en casa | Copia vault a su laptop (USB, Dropbox personal, email) | Funciona normalmente |
| Trabajar en hospital | Copia vault a la PC del hospital | Funciona normalmente |
| Perder el vault | Regenera uno nuevo en cualquier dispositivo | Pierde capacidad de firmar notas viejas. Puede crear notas nuevas. |
| Robo del laptop | El ladrón tiene el vault pero no el password | No puede desbloquear. Sin password, el vault es inútil. |

### 6.4 Qué NO hace el sistema

- ❌ No distribuye el vault automáticamente.
- ❌ No recupera un vault perdido.
- ❌ No fuerza al médico a sincronizar su vault.
- ❌ No almacena el vault en el servidor (aunque el médico puede optar por hacerlo).

### 6.5 Opcional: Vault en el servidor

Si el médico quiere, puede subir su vault cifrado al servidor:

```
Servidor: Almacena vault_{user_id}.bin (cifrado, sin password)
Médico: En dispositivo nuevo, descarga vault + ingresa password → desbloquea
```

Esto es **opcional**, no obligatorio. El default es que el médico maneje su propio vault.

---

## 7. Búsqueda Server-Side con Blind Index

### 7.1 El problema

Si un dispositivo nuevo descarga 10.000 pacientes y el médico busca "García", el dispositivo debe descifrar 10.000 registros en RAM. Es O(n) y no escala.

### 7.2 La solución

El servidor indexa `blind_index` y responde búsquedas exactas sin descifrar:

```
Médico: Busca DNI "12345678"
Dispositivo: Calcula blind_index = SHA-256(salt + "12345678")
Dispositivo: GET /api/entities?blind_index=abc123...
Servidor: SELECT * FROM sync_entities WHERE blind_index = 'abc123...'
Servidor: Devuelve 1-5 registros (no 10.000)
Dispositivo: Descifra solo los resultados
```

### 7.3 Búsqueda difusa (futuro)

Para búsqueda por nombre (no DNI exacto), se necesitan múltiples blind indexes:

```
schema.json:
  "dni" → blind_index exacto
  "phone" → blind_index exacto
  "last_name" → blind_index exacto
  "full_name" → búsqueda difusa (FTS5 local o server-side v2)
```

---

## 8. Timeline Sugerido

| Semana | Enfoque | Entregable |
|--------|---------|------------|
| 1 | Fase 3.5: Core fortalecido | `.data_key` cifrado, `previous_note_id`, `sync_queue` alimentado, índices, blind indexes múltiples |
| 2 | Infraestructura | VPS + Dokploy + PocketBase deployado |
| 3 | Sync Engine v1 | Worker tokio: push + pull básico, notas inmutables |
| 4 | Onboarding por código | Generar código, ingresar código, descifrar blob, instalar secrets |
| 5 | Blind index server | Búsqueda server-side sin descifrar |
| 6 | Testing + pulido | 2 dispositivos, sync bidireccional, edge cases |
| 7 | Teleconsulta prep | Tabla `asynchronous_threads`, notificaciones push |
| 8 | Documentación | Guía de deploy, troubleshooting, rollback |

---

## 9. Riesgos y Mitigaciones

| Riesgo | Probabilidad | Impacto | Mitigación |
|--------|-------------|---------|------------|
| Servidor caído, sync no funciona | Media | Alto | App funciona offline. Sync se reanuda automáticamente. |
| Robo de `.data_key` en dispositivo | Media | Alto | `.data_key` cifrada con password del admin (3.5.1). |
| Código de onboarding adivinado | Muy baja | Alto | 6 caracteres alfanuméricos, expira en 10 min, uso único. |
| Médico pierde su vault | Media | Medio | Regenera vault nuevo. Pierde firma de notas viejas, pero puede seguir trabajando. |
| Servidor comprometido | Baja | Alto | Solo tiene blobs cifrados temporales. No tiene secrets permanentes. |
| PocketBase tiene vulnerabilidad | Baja | Alto | Mantener actualizado. Monitor de seguridad. |

---

## 10. Checklist de Entrega de Fase 4

- [ ] Fase 3.5 completada: core fortalecido, notas inmutables, sync_queue activo
- [ ] VPS desplegado con Dokploy + HTTPS
- [ ] PocketBase corriendo con colecciones configuradas
- [ ] Sync Engine (`tokio`) funcionando en background
- [ ] Push de cambios: sync_queue → servidor
- [ ] Pull de cambios: servidor → DB local
- [ ] Notas inmutables: no hay UPDATE ni DELETE físico
- [ ] Onboarding por código de un solo uso: generar, ingresar, descifrar, instalar
- [ ] Códigos expiran en 10 minutos y se borran del servidor
- [ ] Blind index funcional en búsqueda server-side
- [ ] Vault del médico: copia manual, responsabilidad del médico
- [ ] Backups automáticos diarios
- [ ] Tests de integración: 2 dispositivos, sync bidireccional

---

*Hoja de ruta definitiva. Refleja las decisiones de arquitectura validadas: notas inmutables, onboarding por código, vault como llave física, servidor tubo ciego.*
