# 📋 Guía de Flujo Git — Simplex Health Core
## Versión: v0.0.1-Génesis | Fecha: 2026-07-15

> Esta guía define el flujo de trabajo con Git para el proyecto Simplex Health Core, estableciendo el punto de partida inmutable (Génesis) y las ramas de desarrollo futuro.

---

## 1. Filosofía del Flujo

El proyecto utiliza un modelo **Git Flow simplificado** con las siguientes reglas:

1. **Génesis es inmutable**: El tag `v0.0.1-genesis` nunca cambia. Es la referencia absoluta.
2. **Cada feature parte de Génesis**: Las ramas `feature/*` nacen del tag, no de `main` o `develop`.
3. **Las features no se contaminan entre sí**: Sync, telemedicina y plugins evolucionan en paralelo.
4. **Los hotfixes aplican a Génesis**: Si hay un bug crítico en el core, se fixea y se taggea nueva versión de Génesis.

---

## 2. Estructura de Ramas

```
main (o develop)
  │
  │  commit: "Setup inicial del proyecto"
  │  commit: "Implementa vault y crypto"
  │  commit: "Agrega entidades y notas"
  │  commit: "Corrige entity_keys antes de Génesis"
  │
  └──► v0.0.1-genesis (tag) ───┬── feature/sync-engine ──────► ...
                               │
                               ├── feature/telemedicina ─────► ...
                               │
                               ├── feature/plugins ──────────► ...
                               │
                               └── hotfix/genesis-v0.0.2 ────► v0.0.2-genesis
```

| Rama / Tag | Propósito | Vida | Reglas |
|------------|-----------|------|--------|
| `main` | Código estable en producción | Permanente | Solo recibe merges de features terminadas |
| `develop` | Integración continua (opcional) | Permanente | Ramas feature se mergean aquí para testing |
| `v0.0.1-genesis` | **Punto de partida inmutable** | Eterno | Nunca se modifica. Referencia para todos los forks |
| `feature/*` | Desarrollo de nuevas funcionalidades | Temporal | Nace de `v0.0.1-genesis`. Se mergea y se borra |
| `hotfix/*` | Corrección urgente en Génesis | Corta | Nace de `v0.0.1-genesis`. Genera nuevo tag de Génesis |
| `release/*` | Preparación de release (opcional) | Corta | Nace de `develop`. Solo fixes, no features nuevas |

---

## 3. Comandos Paso a Paso

### 3.1 Preparar Génesis (hacer una sola vez)

```bash
# 1. Asegurarse de estar en la rama principal
git checkout main

# 2. Verificar que el working directory está limpio
git status

# 3. Hacer los fixes mínimos antes del tag (si aplica)
#    - Editar database.rs (encrypted_by_user_id)
#    - Crear KNOWN_ISSUES.md
#    - Actualizar documentación

git add .
git commit -S -m "GENESIS v0.0.1: Prepara core estable pre-sync

- Agrega encrypted_by_user_id a entity_keys
- Documenta deuda técnica en KNOWN_ISSUES.md
- Esquema de DB finalizado: users, entities, entity_keys, notes, sync_queue
- 18 tests pasando (14 crypto + 4 vault)"

# 4. Crear el tag de Génesis
git tag -a v0.0.1-genesis -m "Simplex Health Core v0.0.1-Génesis

Core funcional mínimo:
- Autenticación local (registro/login/unlock/lock)
- Vault por usuario (Argon2id + AES-GCM-256)
- Firma digital Ed25519 en notas clínicas
- Blind Index para búsqueda ciega
- Entidades genéricas + notas con templates
- Esquema SQLite preparado para sync futuro

Excluye:
- Sync Engine
- Servidor remoto
- Plugins
- Telemedicina
- FHIR

Invariantes:
- master_key nunca persiste en disco
- Toda nota lleva firma del médico creador
- PRAGMA foreign_keys = ON siempre activo
- WAL mode para concurrencia"

# 5. Publicar el tag
git push origin main
git push origin v0.0.1-genesis
```

### 3.2 Crear una rama feature desde Génesis

```bash
# Feature: Sync Engine (Fase 4)
git fetch origin
git checkout -b feature/sync-engine v0.0.1-genesis

# Trabajar, commitear, pushear
git add .
git commit -m "sync: implementa Sync Engine con tokio"
git push -u origin feature/sync-engine
```

```bash
# Feature: Telemedicina (Fase 5)
git fetch origin
git checkout -b feature/telemedicina v0.0.1-genesis

git add .
git commit -m "telemedicina: agrega tabla asynchronous_threads"
git push -u origin feature/telemedicina
```

```bash
# Feature: Plugins (Fase 5+)
git fetch origin
git checkout -b feature/plugins v0.0.1-genesis

git add .
git commit -m "plugins: define API base para plugins de terceros"
git push -u origin feature/plugins
```

### 3.3 Mergear una feature terminada

```bash
# Opción A: Mergear a develop (si usás develop)
git checkout develop
git merge --no-ff feature/sync-engine -m "Merge feature/sync-engine into develop"
git push origin develop

# Opción B: Mergear directo a main (si el proyecto es chico)
git checkout main
git merge --no-ff feature/sync-engine -m "Merge feature/sync-engine into main"
git push origin main

# Borrar la rama feature (local y remoto)
git branch -d feature/sync-engine
git push origin --delete feature/sync-engine
```

### 3.4 Hotfix a Génesis (bug crítico en el core)

```bash
# 1. Volver a Génesis
git checkout v0.0.1-genesis

# 2. Crear rama de hotfix
git checkout -b hotfix/genesis-v0.0.2

# 3. Fixear el bug
#    editar archivos...

git add .
git commit -m "HOTFIX: corrige vulnerabilidad en vault.rs

- Descripción del bug...
- Impacto...
- Fix aplicado..."

# 4. Taggear nueva versión de Génesis
git tag -a v0.0.2-genesis -m "Génesis v0.0.2 - Hotfix crítico"

# 5. Publicar
git push origin hotfix/genesis-v0.0.2
git push origin v0.0.2-genesis

# 6. Mergear el hotfix a main (y a las features activas si aplica)
git checkout main
git merge hotfix/genesis-v0.0.2
git push origin main

# 7. Actualizar las features activas con el hotfix
git checkout feature/sync-engine
git merge v0.0.2-genesis
git push origin feature/sync-engine
```

### 3.5 Ver el estado del proyecto

```bash
# Ver todos los tags (versiones de Génesis)
git tag -l -n1

# Ver el árbol completo con tags y ramas
git log --oneline --graph --all --decorate

# Ver de qué tag partió una rama
git merge-base feature/sync-engine v0.0.1-genesis

# Comparar una feature contra Génesis
git diff v0.0.1-genesis..feature/sync-engine --stat

# Ver qué archivos cambió una feature
git diff v0.0.1-genesis..feature/sync-engine --name-only
```

---

## 4. Convenciones de Commit

### 4.1 Formato

```
<tipo>: <descripción corta> (máx 50 chars)

<cuerpo opcional: explicación del cambio, motivación,
impacto, referencias a issues>

<footer opcional: BREAKING CHANGE, fixes #123, etc>
```

### 4.2 Tipos de commit

| Tipo | Uso | Ejemplo |
|------|-----|---------|
| `feat` | Nueva funcionalidad | `feat: agrega sync engine con tokio` |
| `fix` | Corrección de bug | `fix: corrige race condition en vault` |
| `docs` | Documentación | `docs: actualiza guía de instalación` |
| `refactor` | Reestructuración de código | `refactor: extrae módulo de cifrado` |
| `test` | Tests | `test: agrega tests de integración para sync` |
| `chore` | Tareas de mantenimiento | `chore: actualiza dependencias` |
| `genesis` | Cambios al core base | `genesis: agrega encrypted_by_user_id` |

### 4.3 Ejemplos

```bash
# Commit de feature
git commit -m "feat(sync): implementa polling de sync_queue

- Agrega tokio::spawn para background worker
- Procesa operaciones INSERT/UPDATE/DELETE
- Marca registros como synced con timestamp

Closes #42"

# Commit de fix
git commit -m "fix(crypto): maneja nonce de 12 bytes correctamente

El nonce estaba siendo truncado a 8 bytes en arquitecturas
little-endian. Esto rompía la compatibilidad con datos
cifrados en versiones anteriores.

Fixes #69"

# Commit de Génesis
git commit -S -m "genesis: prepara v0.0.1 para tag

- Agrega encrypted_by_user_id a entity_keys
- Documenta deuda técnica en KNOWN_ISSUES.md
- Verifica 18/18 tests pasando
- Esquema de DB congelado pre-sync"
```

---

## 5. Convenciones de Tags

### 5.1 Versionado de Génesis

| Patrón | Significado | Ejemplo |
|--------|-------------|---------|
| `v0.0.N-genesis` | Versión de Génesis con hotfixes | `v0.0.1-genesis`, `v0.0.2-genesis` |
| `v0.1.0` | Primera release con sync funcional | `v0.1.0` (merge de feature/sync-engine) |
| `v0.2.0` | Release con telemedicina | `v0.2.0` (merge de feature/telemedicina) |
| `v1.0.0` | Primera versión estable completa | `v1.0.0` (todas las features mergeadas) |

### 5.2 Anotación de tags

Siempre usar tags **anotados** (`-a`), no ligeros:

```bash
# ✅ Correcto: tag anotado con mensaje descriptivo
git tag -a v0.0.1-genesis -m "Simplex Health Core v0.0.1-Génesis"

# ❌ Incorrecto: tag ligero sin metadata
git tag v0.0.1-genesis
```

Los tags anotados incluyen autor, fecha, y mensaje. Son objetos Git completos.

---

## 6. Cheatsheet Rápido

```bash
# ─── GÉNESIS ───
git tag -a v0.0.1-genesis -m "Core estable pre-sync"
git push origin v0.0.1-genesis

# ─── NUEVA FEATURE ───
git checkout -b feature/nombre v0.0.1-genesis
# ... trabajar ...
git push -u origin feature/nombre

# ─── MERGEAR FEATURE ───
git checkout main
git merge --no-ff feature/nombre
git push origin main
git branch -d feature/nombre
git push origin --delete feature/nombre

# ─── HOTFIX ───
git checkout v0.0.1-genesis
git checkout -b hotfix/nombre
# ... fixear ...
git tag -a v0.0.2-genesis -m "Hotfix"
git push origin v0.0.2-genesis

# ─── UTILIDADES ───
git tag -l                           # listar tags
git log --oneline --graph --all      # ver árbol
git diff v0.0.1-genesis..HEAD        # comparar con Génesis
```

---

## 7. Escenarios Comunes

### 7.1 "Quiero empezar Sync Engine"

```bash
git checkout -b feature/sync-engine v0.0.1-genesis
# Empezar a codear. El core está congelado, no te preocupes por romper nada.
```

### 7.2 "Encontré un bug en el core mientras trabajo en Sync"

```bash
# NO fixear en feature/sync-engine
git stash                          # guardar cambios de sync
git checkout v0.0.1-genesis
git checkout -b hotfix/nombre-del-bug
# fixear...
git commit -m "fix: corrige bug crítico"
git tag -a v0.0.2-genesis -m "Hotfix"
git push origin v0.0.2-genesis

# Volver a sync y traer el hotfix
git checkout feature/sync-engine
git merge v0.0.2-genesis
git stash pop                      # recuperar cambios de sync
```

### 7.3 "Quiero ver qué cambió en Sync respecto a Génesis"

```bash
git diff v0.0.1-genesis..feature/sync-engine --stat
```

### 7.4 "Necesito volver al core puro para hacer una demo"

```bash
git checkout v0.0.1-genesis
# Compilar y demo. Sin sync, sin telemedicina, sin nada extra.
```

---

## 8. Buenas Prácticas

1. **Nunca force-pushees a `main` o tags de Génesis**
2. **Siempre usá `--no-ff` al mergear features** (preserva la historia)
3. **Borrá las ramas feature después del merge** (limpieza)
4. **Firmá los commits importantes** (`git commit -S` con GPG)
5. **Taggeá solo desde commits estables** (tests pasando, build limpio)
6. **Documentá los breaking changes** en el mensaje del tag

---

*Guía de Flujo Git — Simplex Health Core v0.0.1-Génesis*
*Fecha: 2026-07-15*
*Próxima revisión: al mergear la primera feature*
