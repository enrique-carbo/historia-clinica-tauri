# Posicionamiento y Decisiones de Producto — Fichio Care

> **Estado**: v1.0 — 2026-11-17. Fuente de verdad sobre el nicho objetivo y
> las decisiones que derivan de él. No define features: fija el marco para
> decidir qué entra y qué no. El "porqué" del diseño técnico vive en el
> resto de `docs/`; acá va el de producto.

---

## 1. Nicho objetivo

| Perfil | Qué es en el modelo actual |
|---|---|
| **Profesional independiente** | Una instalación propia, un solo usuario staff (él es admin y profesional). |
| **Equipo chico (2–4 profesionales)** | Una instalación compartida: uso por turnos en una máquina o equipos separados. La DataKey compartida es el modelo mental correcto acá — "la historia clínica de este consultorio". |
| **Profesional multi-lugar (rotas en varios lugares)** | Una instalación personal ("segunda historia clínica") que lo sigue por sus dispositivos + las instalaciones de cada lugar donde trabaja. |

**No son tres productos.** Son la misma arquitectura con distinto N de
usuarios y de dispositivos. Ninguno requiere un modo de producto distinto.

---

## 2. Principio rector

**Cubrir el máximo de casos con la mínima cantidad de mecanismos.** La
unidad es la **instalación**: una instalación = una unidad de datos y de
confianza. Todo lo demás (sync, export, roles, perfiles) se cuelga de esa
única primitiva — la cobertura se logra por composición, no por
acumulación de modos.

---

## 3. Decisiones

| # | Tema | Elección |
|---|---|---|
| D1 | Unidad de datos | **La instalación es la unidad.** Las instalaciones nunca se fusionan ni se comparten entre consultorios/lugares. Profesional solo, equipo de 2–4 y HC personal de un rotante son el mismo objeto: una instalación con N usuarios. |
| D2 | Sync | **Multi-dispositivo de la misma instalación** (diseño completo en `docs/fase4-diseno-sync.md`). El rotante es el caso degenerado: instalación de 1 usuario replicada en sus dispositivos — la lleva al consultorio. **No existe "sync entre instalaciones".** |
| D3 | Caso "segunda HC" | La HC personal del rotante es una instalación más. Los datos de un lugar no cruzan a otro; el intercambio entre instalaciones es por export `.md` firmado. Nota legal del caso: `docs/cumplimientos.md` §13. |
| D4 | Export | El `.md` firmado es la **garantía de salida universal** (anti lock-in) para todos los perfiles — es lo que hace seguro adoptar y es la respuesta a "¿qué pasa si el proyecto muere?". |

---

## 4. Qué NO se hace (anti-creep)

- **No hay modos por nicho**: ni "modo equipo", ni "modo multi-sede", ni
  "modo personal". Son la misma instalación con distinto N.
- Pedidos que no salgan de las primitivas (instalación + sync
  multi-dispositivo + export) → **config o módulo**
  (`docs/module-contract.md`), nunca Core. Mismo criterio que ya aplica
  para Med/Vet.
- **Sync entre instalaciones** (multi-clínica, DataKey compartida entre
  consultorios) queda fuera de alcance explícito — coincide con
  `docs/fase4-diseno-sync.md` §9 "Fuera de alcance". Si algún día hay
  tracción en grupos grandes (5–10), es una capa nueva, no una
  reescritura.

---

## 5. Consecuencias para la Fase 4 (sync)

- El caso crítico para sync es el **rotante**: su instalación personal en
  2+ dispositivos. El policonsultorio por turnos ya funciona hoy local
  (una instalación, una máquina).
- Diseñar M1–M5 pensando primero en el **usuario solo con 2
  dispositivos**: menos superficie (sin admin que gestione dispositivos
  ajenos, sin usernames cruzados). El consultorio con varios
  profesionales en equipos separados es el caso general que ya cubre el
  diseño; el usuario solo lo hereda gratis.
- **Onboarding impecable para el usuario solo**: no tiene admin que lo
  configure — él ES el admin. El flujo queda en 3 pasos (admin bootstrap
  → frase semilla → altas) y se simplifica por documentación, no por
  unificar la UI: tutoriales por perfil en `docs/GUIA-USUARIO/onboarding.md`.

---

## 6. Riesgos operativos del nicho

- **Churn por seed olvidada** (data loss, no competencia): el usuario
  independiente que deja de usar la app y pierde la contraseña sin haber
  anotado la seed. El onboarding debe *hacer* que anote la seed (hoy:
  flag `isSeedConfigured`, la frase nunca toca localStorage) y considerar
  una verificación temprana no intrusiva ("¿podés leer tu frase?").
- **Nota legal del caso "segunda HC"**: el uso personal no exime del
  registro institucional y la HC personal también es requerible por el
  paciente. Detalle en `docs/cumplimientos.md` §13.

---

*Decisiones acordadas en sesión (2026-11-17): nicho = profesionales
independientes + equipos de 2–4 + rotantes con HC personal; la
instalación como única unidad; sync multi-dispositivo como único
mecanismo nuevo; export firmado como salida universal.*
