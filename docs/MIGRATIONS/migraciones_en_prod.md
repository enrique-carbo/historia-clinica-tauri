# 🧬 Migraciones de Esquema en Producción

> Cómo se versiona y actualiza el esquema de SQLite (`historia_clinica.db`) cuando la app corre en una instalación real con datos de pacientes.
>
> Código fuente: [`src-tauri/src/db/migrations/mod.rs`](../src-tauri/src/db/migrations/mod.rs) · [`src-tauri/src/db/database.rs`](../src-tauri/src/db/database.rs)

---

## 1. Por qué existe

En desarrollo es válido borrar la DB y recrearla. En producción **no**: la instalación tiene la historia clínica firmada, los perfiles y la auditoría. Cualquier cambio de esquema (nueva tabla, columna, índice, constraint) tiene que aplicarse **encima de la base existente, sin perder datos y de forma atómica**.

El sistema de migraciones resuelve eso con tres promesas:

1. **Determinismo**: cada DB sabe en qué versión está (`PRAGMA user_version`) y solo corre lo que le falta.
2. **Atomicidad**: cada migración corre en su propia transacción — o aplica completa, o no aplica nada.
3. **Compatibilidad**: una base creada por una versión *más nueva* de la app se rechaza en el arranque en vez de corromperse.

---

## 2. Mecanismo técnico

### 2.1 El marcador de versión

SQLite guarda un entero en el header del archivo: **`PRAGMA user_version`**. Arranca en `0` (sin migrar) y sube con cada migración aplicada. No necesita tabla propia, no se puede editar desde SQL común y viaja dentro del archivo (si la migración hace rollback, el número también).

```sql
PRAGMA user_version;   -- ¿En qué versión está esta DB?
PRAGMA user_version = 2; -- Fijar versión (dentro de la transacción de la migración)
```

### 2.2 El catálogo

En Rust existe una lista ordenada y **append-only**:

```rust
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        description: "baseline: esquema inicial (users, profiles, signing_keys, ...)",
        up: v001_baseline,
    },
    // v002, v003… siempre al final, nunca reordenar ni editar las anteriores
];

pub struct Migration {
    pub version: u32,                 // contiguo, arranca en 1
    pub description: &'static str,    // qué hace (aparece en el log)
    pub up: fn(&Connection) -> Result<(), String>,  // DDL/DML
}
```

### 2.3 El runner (`run_migrations`)

```text
run_migrations(conn)
 │
 ├─ 1. leer PRAGMA user_version → `current`
 │
 ├─ 2. current > última versión conocida
 │      → ERROR: "la base fue creada por una versión más nueva de la app"
 │      → la app NO arranca (mejor abortar que degradar)
 │
 ├─ 3. para cada migración con version > current:
 │      BEGIN
 │        ├─ (m.up)(&tx)          ← DDL de la migración
 │        ├─ PRAGMA user_version = N
 │      COMMIT                    ← si algo falla: ROLLBACK automático
 │
 └─ 4. devolver la versión final
```

- Cada migración tiene **su propia transacción**: si `v3` falla, `v1` y `v2` ya quedaron aplicadas y `v3` no dejó rastro (SQLite DDL es transaccional).
- Si ninguna está pendiente (arranque normal), el costo total es **una lectura de `user_version`**.

### 2.4 Dónde se ejecuta

En `init_db()`, dentro del `setup` de Tauri — **antes** de que exista cualquier sesión, vault o comando IPC:

```rust
conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
run_migrations(&mut conn)?;   // ← acá; si falla, setup devuelve Err y la app no abre
```

---

## 3. Ciclo de vida en producción

### Arranque normal (sin cambios pendientes)

```text
Usuario abre la app
  → init_db()
  → user_version == 8 (por ejemplo)
  → no hay pendientes
  → login, vault, datos… (sin ninguna escritura de esquema)
```

### Actualización de la app (nueva versión con migraciones nuevas)

```text
1. El usuario instala el nuevo binario (por ejemplo, v0.1.0 → v0.2.0 con 2 migraciones nuevas)
2. Al abrir, init_db() lee user_version = 8
3. Aplica v9 (transacción) → v10 (transacción)
4. Log: "🧬 [DB] Migración aplicada: v9 — …"
5. La app sigue el arranque normal
```

El usuario no hace nada, no ve prompts y no hay "modo mantenimiento": toda la operación ocurre en los primeros milisegundos del arranque, antes de que la UI toque la base.

### Falla de una migración (el peor caso)

```text
v9 falla (SQL inválido, constraint inesperado…)
  → ROLLBACK completo de v9
  → user_version sigue en 8
  → init_db devuelve Err
  → el setup de Tauri falla → la app muestra el error y NO abre
```

La base queda **exactamente como estaba antes de intentar** (v8, datos intactos). Acción de soporte: leer el mensaje, publicar una corrección (nueva migración o fix del binario) o restaurar un respaldo. **Nunca** queda una migración aplicada a medias.

---

## 4. Reglas de oro

| # | Regla | Motivo |
|---|---|---|
| 1 | **Nunca editar una migración ya publicada** | Las DBs de producción ya la corrieron; editarla crea divergencias silenciosas entre entornos. Cualquier fix → migración nueva. |
| 2 | **Versiones contiguas desde 1** | `run_migrations` calcula los pendientes por número; huecos rompen la lógica. |
| 3 | **Solo hacia adelante (sin `down`)** | Para SQLite con datos reales, revertir esquema = perder columnas con datos. La vía de reversa es: restaurar respaldo + fix. |
| 4 | **Una intención por migración** | Preferí 3 migraciones chiquitas a una gigante: el rollback es más fino y el log más claro. |
| 5 | **Idempotencia donde sea posible** | `IF NOT EXISTS`, `ADD COLUMN` con default conocido — protege contra DBs de desarrollo antiguas (user_version 0 con tablas ya creadas). |
| 6 | **Cambios destructivos ⇒ respaldo antes** | Renombrar/eliminar columna con datos: el flujo es *respaldo → upgrade*, y el respaldo lo hace el usuario desde la tab **Respaldos**. |

### ¿Y `v001`? ¿Está congelada?

**Todavía no.** Mientras no exista una instalación de producción con datos:

- podés reescribir `v001_baseline` (añadir tablas, renombrar columnas, cambiar CHECKs),
- borrando después la DB en dev para que se recree.

**En cuanto el primer usuario real corra `v001`, queda congelada para siempre.** A partir de ahí, todo cambio de esquema es `v002`, `v003`, …

---

## 5. Cómo escribir un cambio nuevo (paso a paso)

Ejemplo: agregar una columna `notes TEXT` a `audit_log`.

**1. Agregar la migración al catálogo** (al final, en `migrations/mod.rs`):

```rust
pub const MIGRATIONS: &[Migration] = &[
    Migration { version: 1, description: "baseline: …", up: v001_baseline },
    Migration {
        version: 2,
        description: "audit_log: agregar columna notes",
        up: v002_audit_log_notes,
    },
];

fn v002_audit_log_notes(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "ALTER TABLE audit_log ADD COLUMN notes TEXT NOT NULL DEFAULT '';",
    )
    .map_err(|e| format!("v002 falló: {}", e))
}
```

**2. Verificar en dev**:

```bash
# Opción A (recomendada para probar el camino incremental):
#   mantener la DB de dev existente y relanzar → debe aplicar solo v2.
# Opción B (camino desde cero):
rm ~/.local/share/com.ecarbo.historia-clinica-app/historia_clinica.db*
pnpm tauri dev   # recrea y corre v1 + v2
```

Ambos caminos importan: la DB vieja ejercita `v1 → v2` (incremental, lo que hará prod) y la nueva ejercita `v1 → v2` desde 0 (lo que hará una instalación recién instalada).

**3. Tests**: `cargo test --lib` — el módulo `db::migrations` ya tiene los tests de runner; si el cambio toca lógica de negocio, sumar tests en el comando correspondiente.

**4. Documentar**: contabilizar en la tabla de tests del `README.md` y, si el cambio es relevante para la operación, en esta guía.

---

## 6. Dev vs Producción

| Aspecto | Desarrollo | Producción |
|---|---|---|
| DB vieja | Se puede borrar sin drama | Jamás borrar; siempre migrar encima |
| Cambio de esquema | Editar `v001` + borrar DB (pre-primer-release) | Solo migración **nueva** (`v00N`) |
| Ejecución de migraciones | Automática al relanzar `pnpm tauri dev` | Automática al abrir la app |
| Fallo | Borrás la DB y seguís | Rollback + app no abre → fix o restore |
| Cómo probar el incremental | Mantener una copia de DB vieja | Se ejercita solo en cada upgrade real |
| Respaldos | Opcional | **Obligatorios antes de upgrades con cambios destructivos** |

---

## 7. Relación con los respaldos

El manifiesto del respaldo (`manifest.json`) incluye `schema_version` (= `MIGRATIONS.len()` al momento de crearlo). Esto habilita dos protecciones cruzadas:

- **Restaurar un respaldo de una app más nueva**: `validate_backup` lo rechaza con *"Actualizá la aplicación"* — nunca se pega un esquema más nuevo sobre un binario más viejo.
- **Restaurar un respaldo viejo en una app nueva**: se permite (`schema_version` anterior ≤ actual) y `restore_backup` reabre la base con `init_db` → **las migraciones pendientes se aplican solas sobre el respaldo restaurado**.

Flujo recomendado ante un upgrade riesgoso:

```text
1. Tab "Respaldos" → Generar respaldo  (queda registrado en audit_log)
2. Instalar la versión nueva de la app
3. Abrir la app → migraciones corren solas
4. Si algo falla → Restaurar el respaldo + binario anterior
```

---

## 8. Checklist antes de cada release

- [ ] Ninguna migración ya publicada fue editada (`git diff` sobre `migrations/mod.rs`).
- [ ] Versiones contiguas sin huecos.
- [ ] `cargo test` en verde (incluye `db::migrations`).
- [ ] Probado con una **DB anterior copiada** (camino incremental `vM → vN`).
- [ ] Probado con una **DB recién creada** (camino `0 → vN`).
- [ ] Si hay cambios destructivos: existe respaldo reciente y se probó el restore.
- [ ] El `README.md` refleja el nuevo total de tests/migraciones.

---

## 9. Qué NO hace este sistema

Para no llevar expectativas equivocadas:

- **Sin checksums de contenido**: nada verifica que `v001` en tu máquina byte-a-byte con la de otra instalación. La protección es la convención (append-only) + `git`.
- **Sin migraciones inversas**: no existe `migrate down`. Reversa = respaldo.
- **Sin `schema_migrations` de auditoría**: solo se guarda el número (`user_version`), no fecha ni hash por migración aplicada. Si algún día hace falta trazabilidad por migración, se puede añadir una tabla en una `v00N` sin romper nada.
- **Sin coordinación multi-proceso**: el diseño asume una sola instancia de la app por instalación (el lock de la DB está en un `Mutex` en RAM). Dos app instancias abriendo la misma DB a la vez no está contemplado.
- **El guard de downgrade protege la base, no al usuario**: si la DB es más nueva, la app no arranca y el usuario debe actualizar el binario — no intenta "adaptarse".

---

## 10. Referencia rápida

```bash
# ¿En qué versión está una DB?
sqlite3 ~/.local/share/com.ecarbo.historia-clinica-app/historia_clinica.db "PRAGMA user_version;"

# Log de migraciones al arrancar (tauri dev / app)
# 🧬 [DB] Migración aplicada: v2 — audit_log: agregar columna notes

# Tests del sistema de migraciones
cargo test --lib db::migrations
```

**TL;DR**: en dev podés borrar la DB todo lo que quieras; en prod nunca se borra — se migra. Las migraciones corren solas en el arranque, cada una en su transacción, y una DB de una versión más nueva hace que la app se niegue a abrir en vez de corromperla. A partir del primer release real, `v001` queda congelada y todo cambio nuevo es una migración nueva.
