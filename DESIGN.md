# 🎨 DESIGN.md — Reglas Visuales del Frontend

Fuente de verdad de las decisiones de diseño de la UI (`src/components/`). Todo componente nuevo o cambio visual debe ajustarse a estas reglas.

> **Estado actual**: tema oscuro único con colores codificados en duro sobre Tailwind v4. Los tokens de este documento ya están nombrados de forma semántica para migrar a **doble tema (claro/oscuro) sin reescribir componentes** — ver §12.

---

## 1. Principios

1. **Dark-first, flat, border-first.** La profundidad se define con capas de fondo + bordes (`zinc-950 → zinc-900 → zinc-800` + `border-zinc-800`), no con sombras. Las sombras se reservan para superficies flotantes (§7).
2. **Densidad legible.** Es una app clínica con mucha información: cuerpo `text-sm`, metadatos `text-xs`, jerarquía por peso/color, no por tamaño gigante.
3. **Semántica antes que estética.** Un color significa una cosa (§9). No se usan colores decorativos fuera del set semántico.
4. **Un token por rol.** Si dos elementos cumplen la misma función, usan el mismo token/clase.
5. **Los componentes de `src/components/ui/` son la ley.** Para botones, cards, alertas, inputs, tabs y drawer se usa el componente, no clases sueltas (§10).

---

## 2. Tokens de Color (semánticos)

Los nombres de la columna *Token* son los que deben existir en la migración (§12). La columna *Dark (hoy)* es la clase actual a usar mientras no existan los tokens.

| Token | Rol | Dark (hoy) | Light (futuro) |
|---|---|---|---|
| `canvas` | Fondo raíz de la app | `zinc-950` | `zinc-50` |
| `surface` | Paneles, cards, drawer, modales | `zinc-900` | `white` |
| `surface-raised` | Hover/elevación sobre `surface` | `zinc-800` | `zinc-100` |
| `field` | Fondo de inputs, selects, textareas | `zinc-950` | `white` |
| `border` | Borde de superficies (cards, inputs, filas) | `zinc-800` | `zinc-200` |
| `border-strong` | Borde en hover/estado activo, divisores | `zinc-700` | `zinc-300` |
| `text-primary` | Texto principal, títulos, contenido | `zinc-200` | `zinc-900` |
| `text-secondary` | Labels, texto de apoyo, ítems de nav | `zinc-400` | `zinc-600` |
| `text-muted` | Meta (timestamps, contadores, estados) | `zinc-500` | `zinc-500` |
| `text-placeholder` | Placeholder de campos | `zinc-700` | `zinc-400` |
| `text-inverse` | Texto sobre fondos de acción | `white` | `white` |
| `action-primary` | Fondo de acción primaria (hover: `+1 step`) | `blue-600` | `blue-600` |
| `action-primary-hover` | Hover de acción primaria | `blue-700` | `blue-700` |
| `focus` | Anillo de foco (`ring`) | `blue-500` | `blue-500` |

### Semánticos de estado

Cada estado tiene 3 variantes: **texto/icono** (`-400`), **tint de fondo** (`-950` o `-900` con opacidad) y **borde** (`-800/50`).

| Estado | Texto | Tint de fondo | Borde | Uso |
|---|---|---|---|---|
| Éxito (`success`) | `emerald-400` / `green-400` | `green-950/40` o `green-900/30` | `green-800` | Export exitoso, guardados OK |
| Error (`danger`) | `red-400` | `red-950/30` o `red-950/40` | `red-800` | Errores, acciones destructivas |
| Warning (`warning`) | `amber-400` / `yellow-400` | `yellow-900/30` | `yellow-800/50` | Advertencias, firmas pendientes |
| Info (`info`) | `blue-400` | `blue-950/30` o `blue-900/30` | `blue-800/50` | Información, tabs activos |

En modo claro (futuro) se sustituyen los tints por versiones aclaradas (`-50`/`-100` con el mismo `-500`/`-600` de texto) — conservar siempre la fórmula *texto + tint + borde*.

---

## 3. Superficies y Capas

Niveles de fondo, de menor a mayor jerarquía:

| Nivel | Dark (hoy) | Dónde |
|---|---|---|
| 0 — canvas | `zinc-950` | Fondo de la ventana, fondos de tab, `zinc-950/50` para overlays |
| 1 — surface | `zinc-900` | Cards, drawer, paneles de formulario |
| 2 — raised | `zinc-800` | Hover de card, superficies activas, botones secundarios |
| 3 — strong | `zinc-700` | Bordes interactivos, botones deshabilitados de fondo |

**Regla**: nunca se salta un nivel en la misma vista ni se inventan niveles intermedios (`bg-zinc-850` no existe).

---

## 4. Tipografía

Sin fuente custom (stack del sistema de Tailwind). Si se suma una font-family futura, se define como token `--font-sans` en `@theme` (§12), no en componentes.

| Rol | Clase | Peso | Ejemplo real |
|---|---|---|---|
| Meta / labels / badges | `text-xs` | `font-medium` o `font-semibold` | Labels de `Input`, contadores |
| **Cuerpo (default)** | `text-sm` | `font-normal` | Todo el contenido de formularios, listas, cards |
| Títulos de sección | `text-lg` | `font-semibold` | `h2` de tabs (`Entries (Append-Only)`) |
| Títulos de panel | `text-xl` / `text-2xl` | `font-bold` | Títulos de vistas y modales |
| Acciones (botones) | `text-sm` | `font-semibold` | `Button` |

- **Nunca** `font-bold` en texto corrido; bold solo en títulos.
- Texto secundario se distingue con color (`text-secondary`/`text-muted`), no con tamaños chicos repetidos.

---

## 5. Espaciado y Densidad

Base 4px (escala Tailwind). Set permitido en uso:

| Valor | Uso típico |
|---|---|
| `gap-2` / `gap-3` | Entre ítems hermanos inline (botones, badges) |
| `gap-4` / `gap-6` | Entre bloques de un formulario o vista |
| `p-3` | Alertas, filas compactas |
| `p-5` | Cards (`Card`) |
| `p-6` | Paneles principales, encabezados de vista |
| `px-3 py-2` / `px-4 py-2` | Inputs (`Input`) / botones |
| `space-y-4` | Stack vertical de campos |

Las tablas/timelines son densas: separar con `border-b border` + `py-2..3`, no con fondos alternados.

---

## 6. Radios

| Clase | Rol | Ejemplos |
|---|---|---|
| `rounded` | Controles pequeños: inputs, botones, badges, alerts | `Input`, `Button` |
| `rounded-lg` | Superficies medias: cards, items de nav, filas | `Card`, items del `Drawer`, botones inline |
| `rounded-xl` | Contenedores grandes: paneles, modales, drawer interno | Panel de `EntryForm` (`p-6 rounded-xl`) |
| `rounded-full` | Elementos puntuales: avatares, pills, botones circulares | Avatar de usuario |

**Regla**: un componente usa **un** radio (no mezclar `rounded-lg` + `rounded-xl` dentro de lo mismo, salvo remates explícitos como tabs `rounded-t-lg`).

---

## 7. Bordes, Sombras y Elevación

- **Todo borde visible usa `border` + el token `border` (`zinc-800`)**; en hover/interacción sube a `border-strong` (`zinc-700`).
- **Sombras solo en flotantes**: `shadow-lg`/`shadow-2xl` para modales, drawer y popovers (superficies que se superponen y necesitan separarse del canvas). Cards y paneles en línea **no llevan sombra**.
- Overlays: `bg-zinc-950/50` + `backdrop` opcional; el panel flotante encima va con borde + sombra.
- En modo claro la elevación necesita sombra proporcional (`shadow-sm`/`shadow-md`) porque el borde solo no alcanza contraste — preverlo al diseñar componentes flotantes.

---

## 8. Estados Interactivos

| Estado | Regla |
|---|---|
| **Hover** | Sube un nivel de superficie (`surface → surface-raised`) o intensifica el color de acción (`blue-600 → blue-700`). Siempre con `transition-colors`. |
| **Focus** | Obligatorio y visible: `focus:outline-none focus:ring-1 focus:ring-blue-500` (patrón de `Input.tsx`). Nunca eliminar el focus ring sin reemplazarlo. |
| **Active/selected** | Fondo tint `bg-blue-900/30` + borde `border-blue-700` + texto `text-blue-300` (patrón de tabs y filtros de categoría). |
| **Disabled** | `disabled:opacity-50 disabled:cursor-not-allowed` + sin hover. |
| **Loading** | El botón conserva su ancho y cambia la etiqueta ("Creando…", "Procesando…") — `isLoading` de `Button`. |
| **Hover en listas** | Card con prop `hover`: `hover:bg-zinc-800 hover:border-zinc-700 cursor-pointer transition-colors`. |

---

## 9. Semántica de Color

| Color | Significado único | No se usa para |
|---|---|---|
| **Azul (`blue`)** | Acción primaria, foco, selección/estado activo, info | Éxitos, decoración |
| **Emerald/Verde** | Éxito confirmado (export, guardado) | Acciones neutras |
| **Rojo (`red`)** | Error, validación fallida, acción destructiva | Énfasis decorativo |
| **Ambar/Amarillo** | Advertencia, pendiente (ej: firma sin verificar) | Éxitos |
| **Purple** | Solo como acento de ícono de sección (timeline) | Fondo de superficie |
| **Zinc** | Toda la escala neutral y los estados por defecto | Mensajes semánticos |

---

## 10. Componentes y Variantes

### Set base (`src/components/ui/`)

| Componente | Variantes | Notas |
|---|---|---|
| `Button` | `primary`, `secondary`, `ghost`, `success`, `danger` + `isLoading` | Única forma de botón con acción de texto. `primary` es el default. En `className` solo layout (`w-full`, `sm:w-auto`) — nunca clases que choquen con la variante (padding, tamaño, color) |
| `Card` | prop `hover` para filas clicables | Borde + `p-5` + `rounded-lg` |
| `Alert` | `error`, `success`, `info` | Tint de estado (§2); **el margen lo pone quien lo renderiza** |
| `Input` / `Textarea` | `label` opcional | `field` + borde + focus ring azul |
| `Navbar` | tabs con ícono + label | Activo: tinte azul + `rounded-t-lg` |
| `Drawer` | abierto/cerrado | `shadow-2xl` + `transition-transform` |

### Reglas de uso

1. **Acciones con texto → `Button`.** No se escriben botones con clases sueltas en vistas (`<button className="px-4 py-2 bg-blue-600…">` está prohibido en UI nueva). Excepción: controles **icon-only** (`✕`, `✏️`) pueden ser `<button>` puro, siempre con `aria-label` y tokens de §2/§8 (ver deuda §14).
2. **Feedback → `Alert`** en el flujo de formularios; los banners de resultado de una acción (como el export) usan la fórmula *tint + borde + texto semántico* de §2, con botón de cierre `✕` si son descartables.
3. **Una card = una entidad/concepto.** Contenedores decorativos para agrupar usan panel (`rounded-xl`) o borde simple, no un `Card`.
4. Los iconos son **emoji** (📌 regla provisional): `text-lg`/`text-2xl` junto a títulos, `text-lg` en botones de selector. Migrar a SVG requiere definir token de tamaño primero.

---

## 11. Movimiento

- **Microinteracciones** (colores, opacidad): `transition-colors` — default para todo.
- **Transformaciones espaciales** (drawer, modal): `transition-transform duration-300 ease-in-out` + `transition-opacity duration-300` para overlays.
- **Evitar `transition-all`**: declarar solo la propiedad que cambia (`transition-colors` como default). Excepción conocida: cuando se animan colores **y** opacidad juntos (ej: reveal de un botón con `md:group-hover:opacity-100`), usar `transition-[color,background-color,border-color,opacity]`.
- Respetar `prefers-reduced-motion` en cualquier animación nueva (deshabilitar desplazamientos; los fades cortos están bien).

---

## 12. Estrategia de Temas Claro y Oscuro

### Enfoque: tokens semánticos + Tailwind v4 `@theme`

Tailwind v4 ya está activo (`src/index.css` = `@import "tailwindcss"`). El plan:

```css
/* src/index.css */
@import "tailwindcss";

/* Tema controlado por clase en <html class="dark">, no solo por SO */
@custom-variant dark (&:where(.dark, .dark *));

:root {
  /* tema CLARO (futuro default) */
  --surface: #ffffff;
  --text-primary: #18181b;
  --border: #e4e4e7;
  /* … resto de la tabla §2 … */
}

.dark {
  /* tema OSCURO = valores actuales de §2 */
  --surface: #18181b;      /* zinc-900 */
  --text-primary: #e4e4e7; /* zinc-200 */
  --border: #27272a;       /* zinc-800 */
  /* … */
}

/* `inline` = las utilities leen la variable viva → cambian con .dark */
@theme inline {
  --color-surface: var(--surface);
  --color-text-primary: var(--text-primary);
  --color-border: var(--border);
  /* genera bg-surface, text-text-primary, border-border, ring-focus… */
}
```

Con esto, `bg-surface` significa "superficie" y se resuelve según el tema activo — los componentes no cambian.

### Modo claro: reglas de inversión

| Elemento | Dark | Light |
|---|---|---|
| Capas | `zinc-950 → 900 → 800` | `zinc-50 → white → zinc-100` |
| Bordes | `zinc-800` | `zinc-200` (interfaces) / `zinc-300` (divisores) |
| Sombras | Mínimas (los bordes bastan) | **Obligatorias** en flotantes (`shadow-sm/md`) |
| Tints de estado | `*-950/30` oscuros | `*-50` / `*-100` claros con texto `*-600`/`*-700` |
| Focus ring | Azul 500 visible sobre oscuro | Azul 500 (verificar contraste 3:1) |
| `text-inverse` sobre acciones | `white` sobre `blue-600` | igual (verificar 4.5:1) |

### Fases de migración

1. **Fase A — Tokens sin cambio visual**: crear `:root`/`.dark`/`@theme inline` en `index.css` con los valores actuales (oscuro) y migrar **primero los componentes de `ui/`** a las utilities semánticas. Nadie nota diferencias.
2. **Fase B — Vistas**: reemplazar clases sueltas en `src/components/**/*.tsx` por tokens semánticos. Cazar residuos con `rg "bg-zinc|text-zinc|border-zinc" src/`.
3. **Fase C — Tema claro**: valores reales en `:root`, toggle de tema (clase `.dark` en `<html>`, preferencia persistida + respeto de `prefers-color-scheme` inicial), QA de contraste en ambos temas.
4. **Fase D — Componentes exploratorios**: Drawer, modales y sombras; estados hover/active en light.

---

## 13. Accesibilidad

- **Contraste**: texto principal y secundario ≥ 4.5:1; placeholders y texto decorativo ≥ 3:1; texto sobre acciones semánticas ≥ 4.5:1. En light, verificar especialmente los `*-600` sobre `*-50`.
- **Foco visible** en todo elemento interactivo (§8) — nunca `outline-none` sin reemplazo.
- **Targets**: controles interactivos mínimos ~40px de alto efectivo (inputs `py-2.5`, botones `py-2` + área).
- **Íconos/emoji** decorativos: `aria-hidden="true"`; los botones solo-ícono llevan `aria-label` (patrón `✕` del export).
- No comunicar estado solo con color: siempre acompaña texto (ej: firma ✓/✗ con label, no solo color).

---

## 14. Deuda de Diseño Conocida

Registrar y evitar replicar; se limpia al migrar (Fase B):

| Deuda | Dónde | Estado |
|---|---|---|
| ~~Botones inline con clases sueltas~~ | `MedicoView`, `EntryTimeline`, `Entity`, `EntryForm` + variante `success` nueva en `Button` | ✅ Migrados. `AuthBox` ya usaba `Button` |
| ~~`transition-all` (19 usos)~~ | `ProfileView`, `Entity`, `PatientEhrView`, `Navbar` | ✅ Todos a `transition-colors` (el reveal de `✏️` usa lista arbitrary) |
| Botones icon-only (`✕`, `✏️`) y logout del `Drawer` (danger ghost sin variante) | Vistas y `Drawer` | ⏳ Excepción aceptada: `<button>` puro con `aria-label` + tokens. Agregar `size`/`ghost-danger` a `Button` las eliminará |
| Botones de acción con clases sueltas (fase B) | `ProfileView`, `SeedPhraseSetup`, `DashboardLayout` | ⏳ Migrar al tocar cada vista |
| ~~`Alert` con `m-3` interno~~ | `Alert.tsx` + `AuthBox` | ✅ `m-3` eliminado; `Alert` ahora acepta `className` y el consumidor pone su margen (`my-3`) |
| ~~`text-zinc-600/700` en texto no-placeholder~~ | 13 usos en 8 archivos (meta, empty states, iconos decorativos) | ✅ Todos a `text-zinc-500` (`text-muted`). Placeholders (`zinc-700`) intactos por diseño |
| ~~Animaciones muertas (`animate-in` sin plugin)~~ | `Entity:355`, `PatientEhrView:155` | ✅ Instalado `tw-animate-css` (compatible Tailwind v4) + import en `index.css` |
| Sombras ad-hoc sin token (`shadow-md`, `shadow-emerald-*`) | Puntuales | ⏳ Mapear a §7 al migrar |
| Doble fuente de verdad de `schema.json` (frontend/backend) | `src/config/` + `src-tauri/config/` | ⏳ Unificar en Fase B (fuera del alcance visual) |

---

## 15. Checklist de Contribución Visual

Antes de mergear UI nueva:

- [ ] ¿Los colores salen de la tabla §2/§9? (sin colores ad-hoc)
- [ ] ¿La acción usa `Button` con su variante correcta?
- [ ] ¿La superficie está en el nivel de capas correcto (§3) con su borde `border`?
- [ ] ¿El radio corresponde al tamaño del elemento (§6)?
- [ ] ¿Tiene hover + focus visible + disabled/loading si aplica (§8)?
- [ ] ¿El texto usa pesos/tamaños de §4 y color semántico para secundario?
- [ ] ¿El feedback de error/éxito usa `Alert` o la fórmula tint+borde+texto?
- [ ] ¿Funciona con la paleta actual y **sin asumir fondo oscuro en el código** (clases semánticas, no `zinc-900` hardcodeado cuando existan los tokens)?
- [ ] ¿Sin `transition-all` y respetando `prefers-reduced-motion`?
