# Release y actualizaciones

Procedimiento operativo para publicar una versión. La política de update
(al usuario final) vive en README §"Versiones y actualizaciones" — acá va
el checklist.

## Checklist de release

1. **Bump de versión** (semver) solo en la fuente única:
   `src-tauri/tauri.conf.json` → `pnpm version:sync`.
   - `major`: rompe compatibilidad de datos o de respaldos.
   - `minor`: features nuevas (incluye migraciones append-only).
   - `patch`: arreglos; idealmente sin migraciones.
2. **Verificación completa** (AGENTS.md §Verificación):
   ```bash
   cd src-tauri && cargo test && FICHIO_PROFILE=vet cargo test && cargo clippy --all-targets
   cd .. && npx tsc --noEmit && pnpm test && pnpm version:check
   pnpm build && pnpm build:vet:frontend
   ```
   - Licencia: SPDX `AGPL-3.0-or-later` presente en `package.json` y
     `src-tauri/Cargo.toml`; `LICENSE` en la raíz.
3. **Commit** con la versión (ej. `Release v0.2.0`) y tag
   `v0.2.0`.
4. **Builds de bundle** por plataforma (cada perfil, en su SO):
   ```bash
   pnpm build:med    # Fichio Care  → src-tauri/target/release/bundle/
   pnpm build:vet    # Fichio Vet   → ídem (productName distinto)
   ```
   Targets Linux: `.deb` + AppImage (evitar `targets: "all"` en CI —
   compila cross sin probar).
5. **Smoke test del instalador en una máquina/entorno limpio**: instalar,
   abrir, login con un usuario existente (la base debe sobrevivir la
   reinstalación), crear 1 entry, exportar.
6. **Publicar**: release con instaladores + notas de versión (qué cambió
   para el usuario, si hay migración avisarlo explícitamente) + aviso de
   **respaldar antes de actualizar**.

## Después del release

- Verificar `user_version` en una base real actualizada (migración
  corrió) y que un backup anterior restaure sobre la versión nueva.
- Reglas: migraciones append-only (nunca editar una versión ya
  aplicada); las claves de usuario (DataKey/seed) **nunca** rotan en un
  update.

## Pendiente: auto-updater (fase futura, solo desktop)

Cuando haya ~3+ instalaciones reales: `tauri-plugin-updater` con
artefactos firmados y `latest.json` estático por perfil
(`med/` / `vet/` — productos con endpoints separados). Clave privada en
secreto de CI + copia offline (perderla = sin updates; filtrarla =
atacante pushea binarios a todos). Primer rollout: notificar y abrir la
página de release, no auto-instalar en silencio.

**Solo desktop**: iOS/Android prohíben el auto-update fuera de sus
tiendas — en mobile el update lo gestiona Play Store / App Store (ver
README §"Futuro: Mobile"). El patrón unificado es "notificar + abrir
enlace" (`openUrl`), que corre en todos los targets.
