# 🗺️ Mapa de Ruta del Desarrollo (Roadmap)

> Extraído del README (2026-10-17) para mantenerlo compacto. El estado
> actual vive acá; el README solo resume.

### 🟩 Fase 1: Almacenamiento Local y Autenticación (✅ Completado)
- [x] Entorno Tauri v2 + pnpm + React + Tailwind CSS
- [x] Sistema de Vault Local con sal única por usuario
- [x] Refactorización a módulos con patrón `with_conn`

### 🟩 Fase 2: Robustez del Core Local (✅ Completado)
- [x] Índice Ciego (Blind Index) con sal derivada
- [x] Buscador Relacional Clínico
- [x] Creación de Design System propio (`/ui`)

### 🟩 Fase 3: Métricas, Firmas Digitales y Arquitectura Frontend (✅ Completado)
- [x] Cifrado selectivo (Numéricas en claro, notas cifradas)
- [x] Migración del estado a Zustand
- [x] Implementación de Optimistic UI y tipado estricto E2E
- [x] Firma Digital Ed25519 para notas (No-Repudio)
- [x] Arquitectura de Template v1 con tablas genéricas
- [x] Comandos genéricos de entidades y notas
- [x] IDs duales: `id` local + `external_id` UUID
- [x] Interfaz clínica unificada (`PatientEhrView`)
- [x] 18 tests unitarios pasando

### 🟩 Fase 3.5: Separación de Datos Clínicos (✅ Completado — legacy eliminado)
- [x] Tabla dedicada `medical_history` con UNIQUE por paciente
- [x] Schema separado `medical_history_schema.json`
- [x] Comandos `upsert_medical_history` + `get_medical_history`
- [x] Store unificado `usePatientStore` (paciente + clinical compuesto)
- [x] `MedicalHistoryForm` desacoplado con optimistic update
- [x] `Entity.tsx` con panel dual sincronizado al store global
- [x] `PatientEhrView` consume antecedentes desde `selected.clinical`
- [x] Limpieza de `schema.json` (solo filiación en `patient`)
- [x] **Eliminado en favor de entries**: tabla, schema, comandos, `MedicalHistoryForm` y tab "Antecedentes"

### 🟩 Fase 3.6: Entity-Entry Paradigm (✅ Completado)
- [x] **Seed Phrase**: Módulo `seed.rs` con mnemonic de 6 palabras + 10 tests
- [x] **Tabla `entries`**: Append-only con índices para subject/category/timestamp
- [x] **Entry Templates**: 4 templates JSON (soap, allergy, medication, condition)
- [x] **Backend**: `entry_commands.rs` con create/get/by_subject/by_category/search_entries
- [x] **Frontend**: `useEntryStore`, `EntryTimeline`, `EntryCard`, `EntryForm`
- [x] **Seed UI**: `SeedPhraseSetup` con flujo de 4 pasos + `useSeedStore`
- [x] **Navegación**: `NavigationDrawer` con menú por roles + `useNavigationStore`
- [x] **Búsqueda**: `search_entries` con debounce de 300ms + filtros por categoría
- [x] **Cleanup**: Removido tab "Antecedentes" (legacy), 26 tests pasando

### 🟩 Fase 3.7: Cifrado de DataKey + Limpieza Legacy (✅ Completado)
- [x] **Módulo `data_key.rs`**: wrap/unwrap AES-GCM + `resolve_data_key` con prioridades
- [x] **DataKey cifrada en disco**: `.data_key.{user_id}` (Argon2id password) + `.data_key.master` (Argon2id seed, sal de dominio `fichio/seed-kdf/v1`)
- [x] **Migración automática**: `.data_key` legacy hex → wrap personal, archivo borrado
- [x] **Bootstrap**: `unlock_vault` devuelve `SEED_REQUIRED`; `AuthBox` pide la frase
- [x] **Master wrap**: `setup_seed_master_wrap` tras verificar la frase en `SeedPhraseSetup`
- [x] **Limpieza `medical_history`**: eliminados comandos, schema (ambas copias), `MedicalHistoryForm`, `usePatientStore.clinical`
- [x] **Seed en papel**: frase no persistida en `localStorage`, solo flag `isSeedConfigured`
- [x] 36 tests pasando

### 🟩 Fase 3.8: Unificación del Sistema Clínico (✅ Completado)
- [x] **Eliminado el sistema SOAP viejo (`notes`)**: tabla, índices, `note_commands.rs`, `NoteTemplatesConfig`, `include_str!(soap.json)`, comandos `create_note`/`get_note`/`get_notes_by_entity`
- [x] **Frontend limpio**: eliminados `useNoteStore.ts`, `note_templates/soap.json` (ambas copias), uso en `PatientEhrView` y `DashboardLayout`
- [x] **Único sistema clínico**: `PatientEhrView` + `EntryTimeline` consumen solo `entries` (paradigma Entity-Entry)
- [x] **Fix firma multi-usuario**: `unlock_vault` sincroniza `professional_profiles.public_key` siempre (derivada desde la privada si el vault ya existía), antes de `resolve_data_key`
- [x] 60 tests pasando

### 🟩 Fase 3.9: Exportación de Historia Clínica + Orden del Core (✅ Completado)
- [x] **`export_history` (roundtrip único)**: descifra paciente + perfil + todas las entries, render puro a Markdown y escritura en `<app_local_data_dir>/exports/` — el snapshot nunca cruza el IPC; el frontend solo manda `subject_id` (sin plugin de diálogos, compatible con mobile futuro); error claro si la bóveda está cerrada
- [x] **Render `export/render.rs`**: tabla de filiación y perfil, resumen por categoría, entries cronológicas con labels canónicos de los templates, check de firma ✓/✗, footer con SHA-256 + aviso de confidencialidad
- [x] **UI en `MedicoView` (tab Entries)**: botón "Exportar .md", banner de éxito con "Revelar en carpeta" (`revealItemInDir`), banners descartables y reset al cambiar de paciente
- [x] **Reorganización `src-tauri/src/` por dominio**: `security/` (auth, crypto, data_key, seed, vault), `db/`, `export/` y `commands/` (incluye `seed_commands`); `lib_types.rs` → `types.rs`
- [x] **Templates en JSON**: `EntryForm.tsx` consume `config/entry_templates/*.json` via `import.meta.glob` (eliminado el `TEMPLATES` hardcodeado)
- [x] 70 tests pasando

### 🟩 Fase 3.10: Roles, Gestión de Usuarios y Recuperación (✅ Completado)
- [x] **Roles nuevos**: `admin` → `asistente` (recepción/admisión) + `administrador` (gestión técnica: usuarios, contraseñas, semilla, auditoría) — CHECK de SQLite actualizado (DB nueva en dev, sin migración)
- [x] **Esquema**: `users.is_active` (baja lógica) + `users.last_login_at`, tablas `signing_keys` (histórico de claves de firma) y `audit_log`
- [x] **`SessionState` en Rust**: sesión activa seteada en `login_user`, limpiada en `lock_vault`; `require_admin()` valida el rol en backend para todo command sensible
- [x] **Login reforzado**: bloquea usuarios desactivados y registra último acceso
- [x] **Registro cerrado**: `register_user` solo crea el primer usuario como `administrador` (bootstrap en `AuthBox`); altas restantes vía `admin_create_user`
- [x] **Panel de Usuarios** (`AdminUsersPanel`): listado con estado/último acceso, alta con rol, baja lógica (sin borrado físico, protecciones: no a uno mismo ni dejar la instalación sin admins)
- [x] **`admin_reset_password`**: sin contraseña previa — regenera vault (keypair nuevo), re-envuelve el wrap personal con la DataKey del admin, actualiza `password_hash` + pública, rota `signing_keys` y audita
- [x] **Verificación histórica**: `public_key_at` contrasta cada entry contra la clave vigente en su timestamp (fallback a `professional_profiles`)
- [x] **Rotación de frase semilla** (`AdminSeedPanel`): generar → anotar → re-escribir → `admin_rotate_seed` persiste (la frase vieja solo muere en el paso final)
- [x] **`AdminView`** con tabs por rol (Usuarios, Admisión, Auditoría, Frase Semilla) + menús de `Drawer` para `administrador`/`asistente`
- [x] 75 tests pasando

### 🟩 Fase 3.11: Respaldo, Recuperación y Migraciones (✅ Completado)
- [x] **Migraciones de esquema** (`db/migrations/`): `PRAGMA user_version` + catálogo append-only; v001 = baseline (schema actual), transacción por migración con rollback, guard de downgrade (DB de una app más nueva no abre); `init_db` delega en `run_migrations`
- [x] **`security/backup.rs`**: `create_backup` (checkpoint WAL → copia DB + `.data_key.*` + `vault_*` → `manifest.json` con SHA-256 por archivo), `validate_backup` (integridad + `schema_version` + defensa contra path traversal), `restore_backup`
- [x] **`backup_commands.rs`**: `create_backup` / `validate_backup` / `restore_backup` — solo administrador, auditados (`backup_created`, `backup_restored`); el restore cierra sesión y llaves, reemplaza archivos, reabre la DB (migraciones)
- [x] **`AdminBackupPanel`** (tab "Respaldos"): generar con diálogo de carpeta + "Revelar en carpeta"; restaurar con resumen del manifest + confirmación → vuelve al login
- [x] **Break-glass `seed_recovery`**: enlace "¿Olvidaste la contraseña?" en `AuthBox`; verifica la frase contra `.data_key.master` → resetea la contraseña del administrador activo más antiguo (vault con keypair nuevo, rotación de `signing_keys`, re-envuelve el wrap personal) → sesión abierta directa + `audit_log`
- [x] **Sal de blind index re-settable** (`OnceLock` → `Mutex<Option<..>>`): un restore desde otra instalación puede cambiarla en caliente → **removido (2026-10-11)**: el blind index se deriva de la DataKey, ya no existe sal
- [x] **Plugin `tauri-plugin-dialog`** (+ capability `dialog:default`) para elegir carpetas de respaldo
- [x] 91 tests pasando

### 🟧 Fase 4: Infraestructura y Sincronización Híbrida (Próximo paso)
- [ ] Despliegue de VPS con Dokploy y PocketBase
- [ ] Sync Engine (`tokio`): background worker polling `sync_queue`
- [ ] Sincronización de `.data_key.master` entre dispositivos
- [ ] Búsqueda server-side por `blind_index` (HMAC con clave derivada de la DataKey: el servidor solo compara, no invierte)
- [x] Cifrar `.data_key` en disco con llave derivada (Fase 3.7)
- [ ] `enroll_data_key`: admin envuelve DataKey al registrar médico (sin tipear seed)
- [x] `change_password`: verifica actual, re-cifra vault, re-envuelve `.data_key.{user_id}`, actualiza `password_hash` (no toca master wrap) + UI en Perfil & 6 tests

### 🟥 Fase 5: Teleconsulta Asíncrona (módulo)
- [x] Diseño v5: `docs/plugin-teleconsulta-v1.md` — chat + pago QR/transferencia (confirmación manual); la SOAP la escribe el profesional en el Core; paciente = usuario `role: 'paciente'` con DataKey ausente estructuralmente; DB propia del módulo (`docs/module-contract.md`)
- [x] P0 Core: `unlock_vault` role-aware + `change_password` sin rewrap + `grant_patient_access` (`users.entity_id`, alta lazy desde la ficha) + reset de contraseña sin wrap para paciente + vista paciente = username/contraseña (sin entries/EHR; la deuda `require_session` de fase 4 M1 quedó resuelta) — 2026-10-11
- [ ] Prerequisite de módulos: refactor `commands/` → `CoreApi` + trait `Module`/`PluginHost` (`module-contract.md` §14)
- [ ] Hitos P1–P5 del diseño (P4 — sync — depende de fase 4, M1–M3)

### ⬜ Fase 6: Interoperabilidad (Futuro)
- [ ] Motor de exportación JSON y traducción a estándares FHIR (IA-Driven)

### 📱 Futuro: Mobile (tras Fase 4)

Decisión (2026-10-17): el producto debe poder correr en mobile **después
de la sync** — un celular sin sync es una copla huérfana de la historia
clínica, y el 90% del valor de mobile es justamente multi-dispositivo.

- **Un core, N targets**: el mismo Rust compila a desktop y a
  Android/iOS (Tauri 2); mobile = proyectos `mobile/` + adaptación fina
  de UI. **Nunca fork** — la regla de perfiles Med/Vet se extiende:
  perfiles por vertical, targets por plataforma.
- **Deuda conocida de plugins**: `dialog` funciona en mobile (pickers
  nativos) y `opener.openUrl` también; `revealItemInDir` es
  **desktop-only** → fallback sharesheet/nativo. Respaldos mobile =
  "guardar en Archivos" (SAF), no elegir carpetas de escritorio.
- **Updates en mobile vía tiendas** (Play Store / App Store) — los SO
  prohíben el auto-update fuera de la tienda; `tauri-plugin-updater` es
  solo desktop. El patrón unificado es "notificar + abrir enlace"
  (`openUrl`), que funciona en todos los targets.
