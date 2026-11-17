# Onboarding — Flujo de Instalación y Tutoriales

> **Estado**: v1.0 — 2026-11-17. Documenta el flujo de arranque tal como
> está hoy (3 pasos deliberados) y da tutoriales por perfil de usuario
> (ver `docs/posicionamiento.md`). Sirve de guía de desarrollo y de QA del
> onboarding: cada paso lista el command que corre y el estado que queda.

---

## 1. Por qué tres pasos (y no uno)

El onboarding tiene tres pasos separados por diseño:

1. **Primer administrador** (bootstrap del registro cerrado)
2. **Frase semilla** (master wrap de la DataKey)
3. **Altas de usuarios** (staff y cuentas de paciente)

No se unifican porque cada paso **crea un estado de seguridad distinto** y
es irreversible por separado:

| Paso | Qué crea | Por qué separado |
|---|---|---|
| 1. Admin | La instalación deja de estar abierta: `register_user` solo funciona con 0 usuarios | Si sale mal algo del registro, no hay vuelta atrás salvo borrar la base en dev |
| 2. Seed | `.data_key.master` (Argon2id(seed, sal de dominio `fichio/seed-kdf/v1`)) | Requiere *acción física* (anotar en papel) que ningún asistente de UI puede hacer por el usuario — forzarla dentro de otro asistente la diluiría |
| 3. Usuarios | Wraps personales + vaults (keypair Ed25519) | Cada alta es una decisión de rol (administrador/asistente/profesional/paciente), no un subpaso técnico |

La compensación es documentación + tutorials, no UI mágica. Si algún día
se une la UI, el orden y los estados de estos pasos siguen siendo la
verdad (la UI solo los encadena).

---

## 2. El flujo, paso a paso (nivel commands)

```text
Primer arranque (instalación sin usuarios)
│
├─ PASO 1 — Bootstrap del administrador
│    AuthBox en modo bootstrap (count_users == 0)
│    register_user({ username, password_plain })   → fuerza rol 'administrador'
│    login_user + unlock_vault (fast path: wrap personal recién creado)
│    → App muestra SeedPhraseSetup (isSeedConfigured == false)
│
├─ PASO 2 — Frase semilla (SeedPhraseSetup, 4 sub-pasos)
│    a. generate_seed        → frase de 6 palabras (nunca se persiste)
│    b. (pantalla display)   → anotar en papel / placa de metal
│    c. verify_seed          → el usuario retipe las 6 palabras
│    d. setup_seed_master_wrap → escribe .data_key.master
│    → onComplete → isSeedConfigured = true → App principal
│
└─ PASO 3 — Altas de usuarios (AdminView, solo el administrador)
     a. admin_create_user          → staff (administrador / asistente / profesional)
     b. grant_patient_access       → cuenta de paciente desde la ficha
                                     (alta lazy, 1 acceso activo por ficha,
                                      users.entity_id)
     → cada alta genera vault + wrap personal del nuevo usuario
```

Estados que el flujo deja atrás:

- Registro cerrado: `register_user` rechaza si ya existe algún usuario.
- `isSeedConfigured` es el único flag de la seed en `localStorage` (la
  frase misma nunca toca el frontend en claro más allá del sub-paso d).
- Login posterior: password → wrap personal → DataKey. La seed no
  participa salvo `seed_recovery` (break-glass, audita) o si falta el
  master wrap (`unlock_vault` devuelve `SEED_REQUIRED` → AuthBox la pide).

---

## 3. Tutorial A — Profesional independiente (1 persona)

> Perfil: una sola instalación, un solo usuario. El profesional es su
> propio admin. Tiempo esperado: **~10 minutos**.

1. **Instalá** la app (desktop; el update se instala encima — ver
   `docs/release.md`). Abrila: te recibe "🌱 Crear Primer Administrador".
2. **Creá tu cuenta** (Paso 1): usuario + contraseña (mínimo 8). Esta
   contraseña abre tus datos todos los días — elegí una que puedas
   recordar pero que nadie más conozca. *No es la frase de emergencia.*
3. **Anotá la frase semilla** (Paso 2): la app te muestra 6 palabras.
   **Escribilas a mano en papel** (o placa de metal) y guardalas en un
   lugar seguro **distinto de esta computadora**. Después retipealas para
   verificar. Sin la frase ni la contraseña, los datos son
   irrecuperables; sin la frase *solo* perdiendo la contraseña, la frase
   te rescata ("¿Olvidaste la contraseña?" en el login).
4. **Completá tu perfil profesional** (MedicoView → Perfil): nombre,
   matrícula, especialidad. Esto sale en el export firmado.
5. **Creá tu primer paciente** (Admisión / Entity): filiación completa.
6. **Hacé el primer backup** (AdminView → Respaldos → generar) y
   seguí la checklist de `docs/cumplimientos.md` §9 (backup semanal,
   semilla fuera de la máquina, contraseña única).

> **Sin más pasos**: como sos el único usuario, el Paso 3 (altas) queda
> vacío. Las cuentas de paciente son opcionales (solo si querés que un
> paciente entre a ver lo que le habilites).

---

## 4. Tutorial B — Equipo de 2–4 profesionales

> Perfil: una instalación compartida (misma DataKey, "la historia clínica
> del consultorio"). Tiempo esperado: ~10 min el setup + ~2 min por
> usuario nuevo.

1. **Una sola persona instala y hace el Tutorial A** (pasos 1–2). Esa
   persona es el `administrador` técnico del consultorio.
2. **El administrador da de alta al staff** (Paso 3 → AdminView →
   Usuarios): por cada colega, usuario + contraseña inicial + rol:
   - `profesional` → atiende, ve EHR, entries y export.
   - `asistente` → recepción/admisión de pacientes.
   - `administrador` solo para quien gestione cuentas/credenciales.
3. **Entregá a cada uno sus credenciales en persona** y que cada uno
   cambie su contraseña en el primer ingreso (Perfil → cambiar
   contraseña, con la previa).
4. **Los pacientes con acceso**: desde la ficha de cada paciente
   ("Otorgar acceso", admin o asistente) se crea su cuenta
   (`role: 'paciente'`).
5. **Regla de oro del equipo**: quien pierda la contraseña pide reset al
   administrador (`admin_reset_password` — genera keypair nuevo, las
   entries anteriores siguen verificables por `signing_keys`). La frase
   semilla del papel es **solo del administrador**: es el break-glass si
   se pierde la contraseña del admin, no se comparte con el staff.

> **Qué significa compartir instalación**: todo el staff logueado accede
> a los mismos datos clínicos (el filtrado es por rol de UI). El control
> fino está en los roles y en la auditoría — ver README §Roles.

---

## 5. Tutorial C — Profesional multi-lugar (HC personal)

> Perfil: rotás en varios establecimientos y querés tus propias
> anotaciones. Tiempo esperado: ~10 min una vez.

1. **Instalá Fichio en tu propia computadora** (no en la del
   establecimiento) y hacé el **Tutorial A** completo. Esa instalación es
   tu historia clínica personal: "segunda HC".
2. **Regla de uso**: en la HC institucional de cada lugar registrás lo
   que corresponde a ese lugar; en tu Fichio personal llevás tu
   seguimiento propio del caso. La personal **no reemplaza** el registro
   del establecimiento — ver `docs/cumplimientos.md` §13 (incluye qué
   pasa si el paciente pide copia).
3. **Futuro (Fase 4)**: la misma instalación se replica en tus
   dispositivos (notebook ↔ consultorio) con la sync multi-dispositivo —
   tu HC personal te acompaña a todos lados. Mientras tanto, el export
   `.md` firmado es tu forma de llevar una copia legible.
4. **Guardá la seed y el backup con el rigor del Tutorial A paso 6**: en
   una instalación personal sos también tu propio administrador.

---

## 6. Rutas de recuperación (post-onboarding)

| Situación | Camino | Qué toca |
|---|---|---|
| Cambio voluntario de contraseña | `change_password` (con la previa) | Re-cifra vault, re-envuelve wrap; no toca firma ni master wrap |
| Staff olvidó su contraseña | `admin_reset_password` (admin) | Vault nuevo (keypair nuevo); `signing_keys` mantiene verificables las entries viejas |
| Admin olvidó su contraseña | `seed_recovery` (break-glass, en el login) | Frase semilla → resetea al admin activo más antiguo; audita; la DataKey no cambia |
| Migración / fallo de disco | `restore_backup` (admin) | Manifest firmado; vuelve al login |

---

## 7. Notas de desarrollo y QA

Invariantes que el onboarding debe conservar (ya cubiertos por tests de
`auth_commands` / `admin_commands`):

- `register_user` con usuarios existentes → rechaza (registro cerrado).
- El primer usuario siempre nace `administrador`.
- La frase semilla solo toca el frontend en los sub-pasos 2b–2d; en
  disco solo vive el master wrap (`Argon2id(seed, sal de dominio)`).
- `setup_seed_master_wrap` corre después de `verify_seed` (el usuario
  demostró posesión antes de que exista el wrap).
- `save_user_wrap` nunca para un `user_id` con `role: 'paciente'`.
- Cada alta de usuario crea vault + wrap; el paciente es la excepción
  estructural (sin DataKey).

QA manual del onboarding (checklist rápida, DB nueva en dev):

- [ ] Bootstrap: crear admin → entra directo → SeedPhraseSetup aparece.
- [ ] Skip imposible: sin `isSeedConfigured` no se llega a la app
      principal (App.tsx gate 2).
- [ ] Retipear mal la frase en 2c → error, sin master wrap escrito.
- [ ] Cerrar sesión → login con password → DataKey sin pedir seed.
- [ ] "¿Olvidaste la contraseña?" con la frase → reset del admin →
      aparece en auditoría.
- [ ] Alta de staff desde AdminView → el nuevo usuario loguea con la
      contraseña inicial y ve solo su rol.
- [ ] `grant_patient_access` desde la ficha → login del paciente sin
      DataKey (commands clínicos le fallan).

---

*Decisión (2026-11-17): el onboarding queda en 3 pasos; la simplificación
va por documentación y tutoriales (este doc), no por unificar la UI. Ver
también `docs/posicionamiento.md` (nicho) y `docs/cumplimientos.md` §9
(checklist de cumplimiento del profesional).*
