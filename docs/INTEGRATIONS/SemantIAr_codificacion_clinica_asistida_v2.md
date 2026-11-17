# SemantIAr · Codificación Clínica Asistida — Diseño v2 (esqueleto)

> **Estado:** Borrador v2 para revisión. Reemplaza a
> `SemantIAr_codificacion_clinica_asistida.md` (v1, archivada como referencia).
> **Naturaleza:** **Módulo** bajo `docs/module-contract.md` (Ruta 1: compilado,
> feature flags, paywall build-time). **No es Core.** `module-contract.md` §1
> lista "terminología (LOINC/SNOMED)" e "IA/OCR" como territorio de módulo.
> **Principio rector (heredado de v1):** la comodidad del médico al ingresar
> datos es prioridad absoluta. La codificación es invisible, no intrusiva y
> fail-open.
> **Ubicación en el roadmap:** Fase 6+ (interoperabilidad). Prerequisite duro:
> `CoreApi` + `PluginHost` existentes (contrato §14) — teleconsulta es el
> primer módulo y valida el contrato antes que este.

---

## 0. Cambios respecto de v1

| Tema | v1 | v2 |
|---|---|---|
| Arquitectura | Crate + packs + sidecar con ciclo de vida propio ("no plugin system") | Módulo Ruta 1: trait `Module`, `module_execute`, tabla `modules`, paywall build-time |
| Dato codificado | Sin definir dónde vive | Capa de anotaciones en la DB del módulo, referenciando `entries.id` (UUID v4). **Nunca** se toca la entry firmada |
| Feedback | `feedback.db` SQLite plano (texto clínico en claro) | Sin texto libre en reposo: offsets + códigos + decisión. Crypto opt-in vía `ctx.wrap_with_datakey` solo si se justifica |
| Léxico | Pack SQLite externo actualizable | Semilla SemantIAr **importada a la DB del módulo** con procedencia; la curación es dato propio del módulo |
| Versión semilla | "v1.2.0" (sin verificar) | **v1.1.0 verificada** (LICENSE, 2026-11-13); CSV oficial analizado completo (Anexo A) |
| Ontología SNOMED | Pack externo | Pack read-only externo, nombrado para entrar al backup (`fichio-*.db`) |
| Fine-tuning | Fase 3 (offline, opt-in) | **Fuera de alcance**: uso secundario de datos sensibles, requiere propuesta legal propia (ver §7) |
| Dataset de evaluación | Tarea de Fase 0 | **Gate legal previo a Fase 0** (consentimiento/desidentificación, Ley 25.326) |
| Mobile | No abordado | Fases 0–1 mobile-ready; sidecar **desktop-only** con fallback explícito |
| Dependencias | Nombres sueltos (d1.2, GLiNER-2.5-Decide, MIMIR, Beveridge, `sct`) | Gate 0: verificar existencia, link, versión pineada y licencia de cada una antes de invertir |
| SQLCipher | Listado como dependencia | Eliminado: no está en el stack (el Core usa cifrado selectivo de campos AES-GCM) |
| Métricas | Cobertura / precisión / recall | + **tasa de error de polaridad/sujeto** (métrica de seguridad, gate duro) |

Se conservan de v1: los principios no negociables (§2), la separación
lógica/datos/inferencia, el orden reglas-primero-modelo-después, los gates por
fase y la regla "solo el dataset propio es evidencia".

---

## 1. Objetivo

Transformar el texto libre de las notas clínicas (español rioplatense) en
**anotaciones estructuradas codificadas a SNOMED CT**, 100% local, sin
bloquear la escritura y sin degradar la experiencia del profesional.

SemantIAr **v1.1.0** (Proyecto SemantIAr, UNLP; **2.016 entradas, 1.784
SCTIDs distintos — verificado sobre el CSV oficial, Anexo A**) es **corpus
semilla**, no diccionario de producción. Aporta variantes de escritura,
expansiones léxicas, pistas de contexto y políticas de abstención. La
polaridad, certeza, temporalidad y sujeto **no vienen en la semilla**: son
la capa de reglas del módulo (Fase 1).

Encaje legal: Ley 27.706 / Res. 189/2018 exigen SNOMED CT para
interoperabilidad (`cumplimientos.md` §8). Este módulo es el camino hacia
Fase 6; el export FHIR en sí es **otro módulo** (no se mezcla acá).

---

## 2. Principios (no negociables, heredados de v1 + contrato)

| Principio | Implicancia |
|---|---|
| **Local-first** | Todo el pipeline corre en el dispositivo. Sin APIs de terminología en la nube. El módulo no tiene `reqwest`; red solo vía `ctx.http_fetch` (allowlist) — en v1 no se usa red para nada |
| **Fail-open** | Módulo inactivo, pack ausente o sidecar muerto → Fichio funciona igual, en silencio. La UI degrada a "no hay sugerencias", nunca a error |
| **No intrusivo** | Ghost text inline, aceptar con Tab, ignorar escribiendo. Cero modales en el flujo de escritura. Revisión diferida en panel aparte |
| **Entries intocables** | El módulo no crea ni modifica entries (contrato §1/§13). Las anotaciones son una capa aparte, append-only, referenciando `entries.id` |
| **Sin PHI en claro fuera del Core** | La DB del módulo no guarda texto clínico libre. El audit del módulo usa variantes cerradas sin PHI |
| **Cada fase útil por sí sola** | Toda fase se desactiva sin romper la anterior. Fase 1 completa = producto entregable sin sidecar |
| **Trazabilidad** | Cada anotación registra versión de léxico, release SNOMED, versión del motor (crate) y del modelo si aplica |
| **Medir antes de escalar** | Ninguna fase avanza sin métricas sobre el dataset propio anotado |

---

## 3. Arquitectura — módulo Ruta 1

### 3.1 Componentes

| Componente | Forma | Responsabilidad |
|---|---|---|
| **`fichio-semantiar`** | Crate de módulo (feature flag, compilado en ediciones pagas) | Implementa `Module`: `descriptor`, `migrate`, `handle`. Dispatch por `action` strings |
| **`semantiar-core`** | Librería Rust pura dentro del crate (sin deps de Tauri) | Tokenización, lookup léxico, reglas de contexto (negación, temporalidad, sujeto), decisión de match. µs, testeable, compila a mobile |
| **Léxico** | Tablas en la DB del módulo (`fichio-semantiar.db`) | Semilla SemantIAr importada con procedencia + curación local. FTS5 opcional para búsqueda de sinónimos; el lookup exacto de siglas usa índice normalizado (plain table) |
| **Pack SNOMED CT** | SQLite read-only externo, versionado por release, firmado | Jerarquía, sinónimos, `sctid` activos. Generado offline (herramienta a verificar en Gate 0: `sct` u otra). Sin Java/Elasticsearch en runtime |
| **Motor de inferencia** | Sidecar **desktop-only** (Fase 2), lanzado por el módulo | Desambiguación de baja confianza. IPC JSON-lines por stdin/stdout. Aislamiento de OOM/panic. Release independiente |
| **Anotaciones + feedback** | Tablas en la DB del módulo | Capa codificada sobre entries + log de aceptación/rechazo. Alimenta curación |

### 3.2 Lo que NO se hace

- ❌ Plugin system de terceros (ya descartado por el contrato; Ruta 2 es
  transporte, no extensibilidad).
- ❌ Commands Tauri por acción del módulo. **Todo por `module_execute`**
  (contrato §5). No se registra nada nuevo en `lib.rs` para este módulo.
- ❌ Modelo in-process (un OOM no puede tumbar el EMR en consulta).
- ❌ LLM generativo para codificar (alucina códigos, no auditable). Solo
  modelos de decisión con respuestas tipadas y probabilidades calibradas.
- ❌ Servicios remotos de terminología (Snowstorm, APIs SNOMED).
- ❌ SQLCipher / segundo mecanismo de crypto en disco.
- ❌ Tocar `entries`, `signing_keys`, el export `.md` firmado o la DataKey.
- ❌ Fine-tuning con datos de usuarios (ver §7.3).

### 3.3 Archivos en disco

```
app_local_data_dir/
  fichio-core.db                        # Core (el módulo NUNCA lo abre)
  fichio-semantiar.db                   # DB del módulo: léxico, anotaciones,
                                        #   feedback, pack_registry, settings
  fichio-semantiar-snomed-<release>.db  # pack ontología read-only, firmado
  sidecar/                              # Fase 2, desktop-only
    semantiar-decision-model            # binario firmado, update manual
```

- El nombre del pack SNOMED matchea el glob `fichio-*.db` → **entra al
  backup automáticamente** (contrato §7), sin excepción al manifest.
  **DECISIÓN PROPUESTA (a confirmar):** se acepta el costo de tamaño del
  backup a cambio de cero excepciones al contrato. Optimización futura
  posible: dedup por hash en el manifest (los packs son inmutables por
  versión). No se diseña ahora.
- El léxico vive **dentro** de la DB del módulo: es dato curado que crece
  con el feedback, no un artefacto inmutable. Su procedencia (versión
  semilla SemantIAr, licencia, hash del archivo original) queda registrada
  en `pack_registry`.
- El sidecar y su binario siguen la política de update manual
  (`docs/release.md`): notificar + enlace, nunca auto-update silencioso.
  En mobile no existe sidecar (ver §9).

### 3.4 Descriptor y gate

```rust
ModuleDescriptor {
    id: "semantiar",                      // A CONFIRMAR: ¿"semantiar" o "terminologia"?
    name: "Codificación clínica asistida",
    version: env!("CARGO_PKG_VERSION"),
    scope: ModuleScope::InstallationAndProfessional,  // DECISIÓN PROPUESTA
    allowed_roles: vec![ActorRole::Profesional, ActorRole::Administrador],
}
```

- **Scope propuesto:** `InstallationAndProfessional` — cada profesional
  decide si quiere sugerencias en su editor (el principio "no intrusivo"
  es por persona, no por instalación). El admin activa el módulo; cada
  profesional recibe acceso explícito (`module_access`).
- **Roles:** `profesional` (codifica y revisa), `administrador` (packs,
  activación, curación técnica). `asistente` y `paciente` **no** ven el
  módulo (fail-closed).
- **Ediciones:** Fichio Médico / Full. **No** en Free (paywall build-time).
  **No** en Vet v1 (SemantIAr es repertorio humano rioplatense; la extensión
  veterinaria de SNOMED CT es otro problema, otro día).
- El gate completo (instalado ∧ activo ∧ rol ∧ acceso) corre en
  `PluginHost::dispatch` **antes** del handle, con actor derivado de
  `require_session()`. Nada de esto lo reimplementa el módulo.

---

## 4. Modelo de datos — DB del módulo

> Migraciones vía `Module::migrate`, `PRAGMA user_version` propio,
> append-only. El Core no conoce este schema (contrato §2).
> **Regla dura: ninguna tabla guarda texto clínico libre.** Solo
> referencias, offsets, códigos y metadatos.

```sql
-- Capa de anotaciones sobre entries (NUNCA modifica la entry)
CREATE TABLE annotations (
    id TEXT PRIMARY KEY NOT NULL,              -- UUID v4
    entry_id TEXT NOT NULL,                    -- entries.id (UUID v4, estable
                                               --   cross-instalación/sync/restore)
    field_path TEXT NOT NULL,                  -- campo del template (ej. "subjetivo")
    span_start INTEGER NOT NULL,               -- offsets en caracteres sobre el
    span_end INTEGER NOT NULL,                 --   texto del campo (sin guardar el texto)
    sctid TEXT NOT NULL,
    display TEXT NOT NULL,                     -- descripción SNOMED (dato público,
                                               --   no PHI del paciente)
    polarity TEXT NOT NULL CHECK(polarity IN ('present','negated','uncertain')),
    temporality TEXT NOT NULL CHECK(temporality IN ('current','history','family','other')),
    subject TEXT NOT NULL CHECK(subject IN ('patient','family','other')),
    certainty TEXT NOT NULL CHECK(certainty IN ('confirmed','suspected','ruled_out')),
    confidence REAL NOT NULL,
    source TEXT NOT NULL CHECK(source IN ('rule','model','manual')),
    status TEXT NOT NULL DEFAULT 'suggested'
        CHECK(status IN ('suggested','accepted','rejected','superseded')),
    lexicon_version TEXT NOT NULL,             -- trazabilidad completa
    snomed_release TEXT NOT NULL,
    engine_version TEXT NOT NULL,              -- versión del crate semantiar-core
    model_version TEXT,                        -- NULL si source != 'model'
    created_by TEXT NOT NULL,                  -- user_id de la sesión (no del request)
    created_at TEXT NOT NULL,
    reviewed_by TEXT,
    reviewed_at TEXT
);
CREATE INDEX idx_annotations_entry ON annotations(entry_id);

-- Feedback: decisión sobre una anotación, sin texto
CREATE TABLE feedback (
    id TEXT PRIMARY KEY NOT NULL,
    annotation_id TEXT NOT NULL REFERENCES annotations(id),
    decision TEXT NOT NULL CHECK(decision IN ('accepted','rejected','corrected')),
    corrected_sctid TEXT,                      -- solo si decision='corrected'
    decided_by TEXT NOT NULL,
    decided_at TEXT NOT NULL
);

-- Léxico: semilla importada + curación local.
-- Shape alineado al CSV real de SemantIAr v1.1.0 (Anexo A): una fila por
-- entrada, N sentidos por entrada. El multi-sentido es real: 362 entradas
-- con ≥2 vínculos SNOMED; 321 sin vínculo (lexical_only / excluded).
CREATE TABLE lexicon (
    id INTEGER PRIMARY KEY,
    seed_id TEXT,                              -- id del CSV (SEM-xxxx / IP-... / LXN-... / R4-...)
    term TEXT NOT NULL,                        -- termino (sigla / forma breve)
    normalized TEXT NOT NULL,                  -- forma normalizada para lookup exacto
    expansion TEXT NOT NULL,                   -- expansión léxica (material propio CC BY 4.0)
    variantes TEXT,                            -- JSON: variantes de escritura del CSV
    category TEXT NOT NULL,                    -- Hallazgo clínico / Procedimiento / Fármaco / ...
    modo TEXT NOT NULL CHECK(modo IN ('direct','context','lexical_only','excluded')),
    tema TEXT,                                 -- tema clínico del CSV (14 temas)
    context_rules TEXT NOT NULL,               -- JSON: positive/negative/guards/policy/
                                               --   uppercase/window/cross_line (+ senses)
    criterio_editorial TEXT,                   -- criterio del repertorio (documentación)
    origin TEXT NOT NULL,                      -- 'semantiar-v1.1.0' | 'curacion-local'
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active','retired')),
    created_at TEXT NOT NULL
);
CREATE INDEX idx_lexicon_normalized ON lexicon(normalized);

-- Sentidos SNOMED por entrada. Tabla separada a propósito: es la capa
-- sujeta a licencia SNOMED (G0.2). La capa léxica de arriba (CC BY 4.0)
-- puede distribuirse sin esta si la licencia no está resuelta.
CREATE TABLE lexicon_senses (
    id INTEGER PRIMARY KEY,
    lexicon_id INTEGER NOT NULL REFERENCES lexicon(id),
    sctid TEXT NOT NULL,
    snomed_term TEXT NOT NULL,                 -- término preferido SNOMED (licenciado)
    relation TEXT NOT NULL,                    -- sense_preferido / sense_contextual / sense_aceptable
    use_condition TEXT,                        -- pistas de contexto por sentido
    origin TEXT NOT NULL
);
CREATE INDEX idx_senses_lexicon ON lexicon_senses(lexicon_id);
CREATE INDEX idx_senses_sctid ON lexicon_senses(sctid);

-- Procedencia de packs y semilla (auditoría de licencias y versiones)
CREATE TABLE pack_registry (
    pack_id TEXT PRIMARY KEY NOT NULL,         -- 'semantiar-seed' | 'snomed'
    version TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    signature TEXT,                            -- firma del vendor del pack
    license TEXT NOT NULL,                     -- texto/licencia verificada en Gate 0
    imported_by TEXT NOT NULL,
    imported_at TEXT NOT NULL
);

-- Config por profesional (el contrato §6.6: config del módulo en su DB)
CREATE TABLE settings (
    user_id TEXT PRIMARY KEY NOT NULL,
    suggestions_enabled INTEGER NOT NULL DEFAULT 1,
    updated_at TEXT NOT NULL
);
```

**Notas de diseño:**

- `entry_id` referencia `entries.id`, que ya es UUID v4
  (`entry_commands.rs`: `Uuid::new_v4()`), estable ante restore/sync. No
  hace falta `external_id` en entries ni tocar el schema del Core. La
  referencia es **blanda** (sin FK cross-DB, igual que el contrato trata
  `entity_id`).
- Los offsets (`span_start/end`) se calculan sobre el texto del campo del
  template. Si una entry futura cambia el template, la anotación queda
  asociada a la versión del texto original vía `entry_id` (las entries son
  inmutables → los offsets nunca quedan huérfanos de significado).
- `display` es terminología pública (SNOMED), no dato del paciente: no
  viola la regla de PHI. **A CONFIRMAR en revisión:** ¿se considera PHI
  la combinación `entry_id + sctid`? Mitigación: la DB del módulo vive en
  el mismo `app_local_data_dir` protegido por el vault del Core; si se
  exige cifrado en reposo para esta capa, usar `ctx.wrap_with_datakey`
  sobre las filas (opt-in, decidido antes de Fase 1).
- Anotaciones append-only con transiciones de `status`; una corrección
  crea anotación nueva (`superseded` la anterior), nunca `UPDATE` del
  código original. Mismo espíritu que las entries.

---

## 5. Superficie de integración

### 5.1 Extensión de `ModuleContext` (aditiva, contrato §4)

El módulo necesita leer el texto de una entry para codificarla en
background. El contrato hoy no expone entries. Se agrega **un método
aditivo** con `default → Err(Unsupported)` (no rompe proxies ni mocks):

```rust
/// Proyección acotada: texto descifrado de los campos del template.
/// El Core descifra internamente; el módulo nunca ve la DataKey.
async fn fetch_entry_text(&self, entry_id: &str)
    -> Result<EntryTextDto, ApiError>;

// core::api::dto
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryTextDto {
    pub entry_id: String,
    pub category: String,
    pub subject_id: i64,
    pub timestamp: String,
    pub fields: BTreeMap<String, String>,   // campo del template → texto plano
}
```

- `BTreeMap` (regla de mapas serializados de AGENTS.md): orden
  determinístico para tests y para el material que el módulo procesa.
- Gate del Core: el actor debe tener acceso a la entity de la entry
  (mismo scoping que `fetch_filiation`; actor `paciente` nunca llega acá
  porque el descriptor no lo permite).
- **Solo lectura.** No se expone escritura de entries a módulos, nunca.
- Vault cerrado → `VaultLocked` propagado; el módulo lo maneja sin panic
  y la UI degrada en silencio (fail-open).

### 5.2 Acciones (`module_execute`, `protocol_version: 1`)

| Action | Actor | Payload | Respuesta | Notas |
|---|---|---|---|---|
| `suggest` | profesional | `{ text, field_path, subject_id? }` | spans + candidatos tipados | Texto **en curso de escritura** viaja en el payload (RAM, transitorio). No se persiste. Debounce ≥ 300 ms en el frontend |
| `code_entry` | profesional | `{ entry_id }` | anotaciones creadas | Background al guardar. El módulo lee el texto vía `ctx.fetch_entry_text` |
| `list_annotations` | profesional | `{ entry_id }` o `{ subject_id }` | anotaciones + estado | Para el panel diferido y el badge en la timeline |
| `review_annotation` | profesional | `{ annotation_id, decision, corrected_sctid? }` | ok | Escribe feedback + transición de status. `decided_by` de la sesión |
| `curate_lexicon` | profesional/administrador | `{ term, sctid, rules?, retire_id? }` | ok | **A CONFIRMAR:** ¿curación solo administrador? Propuesta: profesional puede proponer, administrador aprueba (tabla `status='proposed'`). No implementar el flujo de aprobación hasta Fase 3 |
| `pack_status` | administrador | `{}` | versiones, hashes, firmas | Para el panel de packs |
| `import_pack` | administrador | `{ pack_id, path }` | ok / error tipado | Verifica firma + SHA-256 contra `pack_registry` antes de importar. Solo packs del vendor |
| `get_settings` / `set_settings` | profesional | `{ suggestions_enabled? }` | settings | Toggle por profesional |

- Todo input/output `Serialize + Deserialize` (disciplina Ruta 2,
  contrato §11). Sin excepciones.
- Errores: `ModuleError` tipado; los que crucen al frontend se mapean a
  códigos `ERR_*` con entrada en `formatError()` (`src/utils/errors.ts`).
  Nada de texto de usuario viajando desde Rust.

### 5.3 Auditoría (enum cerrado, sin PHI — contrato §4)

Variantes nuevas propuestas (PR con revisión anti-PHI, extiende el test
existente):

```rust
SemantiarPackImported { pack_id: String, version: String, sha256: String },
SemantiarEntryCoded { entry_id: String, annotation_count: u32 },
SemantiarLexiconCurated { added: u32, retired: u32 },
```

- IDs y conteos son metadatos (precedente: `SolicitudIssued`,
  `TeleconsultaMessageSent`). **Nunca** términos del paciente, texto de
  notes ni `sctid` asociados a una persona en el audit.
- Activación/desactivación ya las cubre `ModuleActivated/Deactivated`.

### 5.4 Frontend (contrato §9: UI en el repo Core, render condicional)

- **`EntryForm`**: ghost text inline solo si
  `is_module_available("semantiar")` ∧ `settings.suggestions_enabled`.
  Llama `module_execute("semantiar", { action: "suggest", ... })` con
  debounce; Tab acepta, escribir ignora. Módulo inactivo → el editor es
  exactamente el de hoy (cero costo, cero error).
- **`EntryTimeline` / `PatientEhrView`**: badge discreto "codificada" +
  acceso al **panel de revisión diferida** (vista del módulo, nunca modal
  durante la escritura).
- **`AdminView`**: tab del módulo para packs (versiones, firmas, importar)
  y curación. Solo `administrador`.
- **Nunca** `invoke("semantiar_*")`: el test que greppea commands tipados
  por módulo (contrato §12) se extiende a `"semantiar_"`.
- Strings nuevos centralizados (convención i18n Fase 0, `docs/i18n.md`).

---

## 6. Fases y gates

### Gate 0 — Legal y prerequisites (sin código, bloquea todo)

- [x] **G0.1 Licencia SemantIAr — VERIFICADO (2026-11-17, Anexo A):**
      existe, es **v1.1.0** (2026-11-13), 2.016 entradas / 1.784 SCTIDs.
      Materiales propios bajo **CC BY 4.0** → absorción como semilla +
      curación derivada + redistribución permitidas **con atribución**.
      Los SCTIDs + términos preferidos SNOMED CT tienen carve-out propio:
      uso productivo/clínico sujeto a licencia SNOMED (→ G0.2).
      Semilla archivada en `docs/INTEGRATIONS/semantiar/` (CSV v1.1.0
      byte-idéntico + LICENSE upstream + README de procedencia).
      Pendiente: atribución en `pack_registry` y en la UI del módulo al
      importar (Fase 0, requisito CC BY 4.0).
- [ ] **G0.2 SNOMED CT Argentina:** resolver el modelo de distribución
      comercial (afiliación por instalación vía Ministerio de Salud vs.
      acuerdo de distribución del vendor). Es restricción de go-to-market,
      no nota al pie. Sin esto no se distribuye la capa SNOMED.
      **Confirmado por el propio recurso:** el LICENSE de SemantIAr
      restringe los SCTIDs + términos preferidos a interoperabilidad,
      docencia e investigación, y sujeta el uso productivo/clínico a la
      licencia de SNOMED International o del Centro Nacional de Referencia
      (Ministerio de Salud de la Nación). Nota: la capa léxica (CC BY 4.0)
      y la capa SNOMED (licenciada) son separables — sin G0.2 resuelto, el
      módulo puede degradar a subrayado + expansión léxica, sin `sctid`.
- [ ] **G0.3 Dataset de evaluación:** base legal para uso secundario de
      200–500 notas reales (consentimiento o anonimización irreversible,
      Ley 25.326 arts. 4/7, Ley 26.529). Protocolo de desidentificación de
      texto libre escrito y revisado. Actualizar `cumplimientos.md` §2.2
      (hoy afirma "no hay uso secundario") **antes** de juntar notas.
- [ ] **G0.4 Dependencias externas:** verificar existencia, link, versión
      pineada y licencia de: herramienta de generación del pack SNOMED
      (`sct`/pacharanero u alternativa), y cada modelo candidato de Fase 2
      (d1.2/Liquid AI, GLiNER, MIMIR, Beveridge — nombres de v1 sin
      verificar). Tabla §8 con evidencia, no con nombres sueltos.
- [ ] **G0.5 Prerequisite de módulos:** `CoreApi` + `PluginHost` +
      `module_execute` implementados y validados por teleconsulta
      (contrato §14, roadmap Fase 5). Este módulo no los estrena.

### Fase 0 — Esqueleto del módulo + semilla + medición

- [ ] Crate `fichio-semantiar` con `Module` implementado (descriptor,
      migrate, handle), feature flag, fuera del binario Free.
- [ ] Importador del CSV semilla (UTF-8 **con BOM**, columnas JSON) →
      `lexicon` + `lexicon_senses`, con procedencia y atribución CC BY 4.0.
      Respeta `modo_codificacion`: `lexical_only` y `excluded` (321
      entradas) **nunca codifican**.
- [ ] `semantiar-core`: lookup exacto de sigla/forma breve (µs).
- [ ] Extensión `fetch_entry_text` en `ModuleContext` (aditiva, §5.1).
- [ ] UI: subrayado pasivo de menciones reconocidas + tooltip con `sctid`
      (action `suggest`, sin persistencia todavía).
- [ ] Dataset de evaluación construido bajo G0.3 (activo del proyecto,
      custodia propia, **nunca** en la DB del módulo ni en el binario).
- [ ] Medir cobertura y precisión contra el dataset.

**Criterio de salida (Gate A):** cobertura ≥ 60% de menciones frecuentes
de problemas (denominador definido en §7) con precisión ≥ 0.90 y tasa de
error de polaridad/sujeto ≈ 0. Si no se alcanza → curación del léxico
antes que inferencia. No avanzar sin entender por qué.

### Fase 1 — Reglas de contexto + codificación silenciosa (sin modelos)

- [ ] Negación ("no FA", "niega"), temporalidad ("antecedente de"),
      sujeto ("madre refiere"), desambiguación léxica. El repertorio ya
      trae las pistas por sentido: FA (SEM-0385) → `49436004` fibrilación
      auricular (pistas ECG/ritmo) vs `88810008` fosfatasa alcalina
      (pistas hepatograma/U/L), política "Abstenerse si falta contexto",
      ventana de 8 tokens sin cruzar líneas. El motor consume
      `positive/negative/guards/policy/window/cross_line` de la semilla.
- [ ] `code_entry` en background al guardar → anotaciones en la DB del
      módulo con trazabilidad completa.
- [ ] Panel de auditoría diferida + `review_annotation` + feedback.
- [ ] Badge en timeline; export `.md` **sin cambios** (ver §6.1).
- [ ] Medir delta de precisión respecto de Fase 0.

**Criterio de salida (Gate B):** precisión ≥ 0.92 en menciones cubiertas;
error de polaridad/sujeto ≤ 0.5% (gate duro de seguridad, ver §7).
Fichio es 100% funcional sin sidecar. **Esta fase es mobile-ready y
entregable por sí sola.** Si las reglas alcanzan, el sidecar se difiere
indefinidamente sin culpa.

### Fase 2 — Sidecar de inferencia (desktop-only, solo lo ambiguo)

- [ ] Protocolo IPC JSON-lines (stdin/stdout), lanzado y supervisado por
      el módulo. **Sin logs de payload a disco. Sin red en el sidecar.**
- [ ] UN modelo de decisión, seleccionado por benchmark sobre el dataset
      propio (candidatos verificados en G0.4). ≤ 2B, cuantización 4-bit.
- [ ] Invocación solo ante baja confianza / ambigüedad no resuelta por
      reglas. Sugerencias del modelo = segunda opción en UI, nunca
      reemplazan la principal. `source='model'` + `model_version`.
- [ ] Fail-open verificado: matar el sidecar no afecta a Fichio (test).
- [ ] Binario firmado, update manual (`release.md`).
- [ ] **Mobile: se queda en Fase 1** (fallback explícito, §9).
- [ ] Medir delta de cobertura/precisión.

**Criterio de salida (Gate C):** ganancia medible sobre Fase 1, latencia
percibida nula en escritura, p95 sidecar < 200 ms (background). Si no hay
ganancia → descartar el modelo sin culpa.

### Fase 3 — Curación y ciclo de vida de packs

- [ ] Herramienta de curación del léxico desde feedback (términos
      faltantes, mapeos erróneos) con flujo de aprobación (a definir).
- [ ] Updates firmados de semilla léxica y de packs SNOMED (nuevo release
      → nuevo archivo, el anterior queda para trazabilidad de anotaciones
      viejas).
- [ ] Versionado completo en `pack_registry` + audit.
- [ ] **Fine-tuning: excluido.** Ver §7.3.

**Criterio de salida (Gate D continuo):** ciclo de mejora demostrable
(nueva versión mejora métricas sobre el dataset) + la experiencia del
médico sigue no intrusiva. Cualquier regresión de usabilidad bloquea.

### 6.1 Export e interoperabilidad

- El export `.md` firmado del Core **no cambia**: los códigos no se
  inyectan en el Markdown (rompería el material firmado y mezclaba
  responsabilidades). El export sigue siendo el documento humano.
- Los códigos se consumen desde el panel del módulo y, a futuro, desde
  el **módulo de export FHIR** (Fase 6, otro crate), que leerá las
  anotaciones vía su propio contrato. Cada módulo renderiza sus propios
  documentos (contrato §13); si este módulo necesita un "reporte de
  codificación", lo renderiza él, no el Core.

---

## 7. Estrategia de evaluación

**El dataset de evaluación es el activo más valioso del proyecto — más
que el repertorio mismo.** Ningún paper reporta rendimiento sobre notas
rioplatenses reales; los F1 de 0.9 caen a 0.6–0.7 fuera de institución.
**Solo el dataset propio es evidencia.**

| Métrica | Definición | Fase 0 | Fase 1 | Fase 2 |
|---|---|---|---|---|
| Cobertura | % de menciones de problemas anotadas con código asignado (nivel mención, denominador = menciones del gold standard) | ≥ 60% | ≥ 70% | ≥ 80% |
| Precisión | % de códigos asignados correctos (exact match o ancestro aceptado, según gold standard) | ≥ 0.90 | ≥ 0.92 | ≥ 0.95 |
| Recall | % de menciones del gold standard detectadas | — | medir | ≥ 0.85 |
| **Error polaridad/sujeto** | % de menciones codificadas con polaridad o sujeto incorrecto (ej. "no FA" → presente). **Métrica de seguridad: gate duro, no promedio** | ≈ 0 | ≤ 0.5% | ≤ 0.5% |
| Latencia lookup | p95 por mención | < 5 ms | < 5 ms | < 5 ms |
| Latencia sidecar | p95 por invocación (background) | — | — | < 200 ms |
| Abstención | % de casos sin sugerencia | monitorear | monitorear | monitorear |

- Breakdown obligatorio por clase (negación / temporalidad / sujeto /
  siglas ambiguas). El promedio esconde el error clínicamente grave.
- **La abstención es de diseño del recurso, no un agregado nuestro:** 219
  entradas de la semilla traen política "Abstenerse si falta contexto" +
  guards (Anexo A). El motor debe respetarla, no "adivinar".
- Alcance v1: **menciones de problemas** (categorías `SOAP_NOTE` y
  `CONDITION`). Medicación y laboratorio quedan para una iteración
  posterior (léxico y reglas distintas, no se mezclan en las métricas).

### 7.1 Privacidad del pipeline

- Texto en curso (`suggest`) y texto de entry (`fetch_entry_text`) viven
  en RAM mientras se procesan. **Nunca se persisten** en la DB del módulo
  ni en logs. El sidecar recibe texto por IPC y devuelve spans tipados;
  su stdout/stderr no se loguea a disco con payload.
- La DB del módulo guarda offsets + códigos + metadatos (§4). Sin texto
  libre en reposo.
- El módulo no ve la DataKey; el Core descifra y proyecta (contrato §8).

### 7.2 Dataset de evaluación

- 200–500 notas reales desidentificadas, anotadas por médicos
  rioplatenses (menciones + `sctid` + polaridad/certeza/temporalidad/sujeto).
- Base legal y protocolo de desidentificación: **G0.3, previo a Fase 0**.
- **Restricción del recurso semilla (LICENSE v1.1.0, "Core Blind"):**
  prohibido usar SemantIAr como ayuda durante rondas de anotación humana
  ciega. El gold standard se anota **a ciegas, sin consultar el
  repertorio**; SemantIAr se evalúa contra el gold, no lo alimenta.
- La semilla v1.1.0 está declarada libre de texto clínico de pacientes e
  identificadores internos de HC → no introduce PHI al producto.
- Custodia separada del producto (activo de investigación del vendor).
  No se distribuye en el binario ni en packs. Su existencia y uso se
  documentan en `cumplimientos.md` cuando se concrete.

### 7.3 Fine-tuning: fuera de alcance

El fine-tuning con feedback agregado (v1 Fase 3) es **uso secundario de
datos sensibles** y contradice la postura actual de `cumplimientos.md`
§2.2 ("no hay uso secundario, cubierto por diseño local-first"), incluso
offline y opt-in. Requiere: consentimiento específico, análisis Ley
25.326 art. 4, y actualización del marco legal. **Se excluye de este
diseño.** Si algún día se justifica, es una propuesta aparte con su gate
legal — no una tarea de Fase 3.

---

## 8. Dependencias externas (verificación en Gate 0; estado por fila)

| Dependencia | Uso | Estado / riesgo |
|---|---|---|
| SemantIAr **v1.1.0** — ✅ verificado (Anexo A) | Corpus semilla | Proyecto SemantIAr, UNLP (J. Sánchez Viamonte). Repo `manwithbarba/Onto-Lex-SemantIAr`: 2 commits, maintainer único. 2.016 entradas / 1.784 SCTIDs confirmados sobre el CSV. Materiales propios **CC BY 4.0** (absorción + derivados con atribución); **SCTIDs + términos SNOMED con carve-out**: uso productivo requiere licencia SNOMED (G0.2) |
| SNOMED CT Edición Nacional Argentina | Ontología de referencia | Licencia de miembro vía Argentina; modelo de distribución comercial a resolver (G0.2) |
| Herramienta de generación del pack (`sct`/pacharanero u otra) | Pack SQLite offline | **Verificar link, maintainer, licencia.** Plan B: generar el pack con tooling propio desde el RF2 oficial (el formato RF2 es público; el riesgo de maintainer único se mitiga con generador propio + versión congelada) |
| Modelo de decisión (candidatos v1: d1.2, GLiNER-2.5-Decide, MIMIR, Beveridge) | Sidecar Fase 2 | **Nombres sin verificar.** Antes del benchmark: link, versión, licencia (LFM Open License umbral 10M USD vs Apache 2.0) y evidencia de que existen y hacen lo que v1 afirma |
| SQLite + FTS5 | Storage | Dominio público. FTS5 solo si la búsqueda de sinónimos lo justifica; lookup exacto = índice normalizado |

---

## 9. Mobile (regla mobile-aware de AGENTS.md)

| Fase | Mobile |
|---|---|
| 0–1 | ✅ `semantiar-core` es Rust puro sin deps de escritorio → compila a Android/iOS. UI ghost text = webview, sin APIs desktop-only. Packs SQLite = archivos locales, sin problema |
| 2 (sidecar) | ❌ **Desktop-only.** iOS/Android no permiten spawn de procesos hijos con binarios descargados. Fallback explícito: mobile se queda en Fase 1 (reglas), sin sugerencias de modelo. La UI degrada en silencio (fail-open), nunca muestra error |
| 3 | ✅ Curación y packs son datos + UI webview |

- Updates de packs/sidecar en mobile: vía tiendas o el patrón unificado
  "notificar + abrir enlace" (`openUrl`), nunca updater desktop.
- El modelo ≤ 2B 4-bit (~1–2 GB RAM) es inviable en gama media mobile:
  otra razón para el fallback, no solo la restricción de procesos.

---

## 10. Sync (Fase 4) y restore

- Las anotaciones/feedback son datos del módulo → **cola de sync propia
  del módulo** (contrato §13: el engine de Fase 4 itera módulos). Diseño
  concreto cuando Fase 4 aterrice; no se inventa ahora.
- `entry_id` es UUID v4 estable → las anotaciones sobreviven restore desde
  otra instalación y sync multi-dispositivo sin remapeo.
- Conflictos: anotaciones append-only con transiciones de status; la
  revisión manual gana sobre la sugerencia automática (last-writer con
  precedencia por `source='manual'`, a definir en el diseño de sync).
- Restore: la DB del módulo viaja en el backup (glob `fichio-*.db`), el
  pack SNOMED también (§3.3). `pack_registry` permite verificar que las
  versiones de trazabilidad de las anotaciones restauradas existen.
- Blind index / DataKey: el módulo no participa. Si la instalación
  restaurada cambia la DataKey, el Core re-envuelve; las anotaciones no
  contienen material criptográfico propio (salvo el opt-in de §4).

---

## 11. Riesgos y mitigaciones (actualizado)

| Riesgo | Impacto | Mitigación |
|---|---|---|
| SemantIAr pequeño (2.016 entradas, 321 sin código SNOMED) | Cobertura insuficiente | Tamaño y licencia verificados (G0.1, Anexo A); semilla + curación continua; el dataset propio es el activo, no el repertorio |
| SemantIAr deja de mantenerse (proyecto UNLP; repo con 2 commits y maintainer único — verificado) | Pérdida de upstream | Absorción del CSV pineado por SHA-256 (Anexo A) con procedencia, licencia y atribución en `pack_registry`; la curación local es propia desde el día 1 |
| Licencia SNOMED / distribución comercial | Bloqueo de go-to-market | G0.2: resolver afiliación/acuerdo antes de distribuir packs |
| Licencia LFM (umbral 10M USD) del modelo | Restricción comercial | Preferir Apache 2.0 si el benchmark es competitivo; verificar en G0.4 |
| Error de polaridad/sujeto (codificar negado como presente) | **Daño clínico potencial** | Métrica de seguridad con gate duro (§7); revisión diferida; nunca autonomía total |
| Sobre-arquitectura | Nunca se lanza | Fases con gates; Fase 1 entregable sola; sidecar diferible sin culpa |
| OOM del modelo en dispositivos modestos | Tumbar la consulta | Sidecar aislado (desktop); mobile sin modelo; fail-open testeado |
| PHI en claro en la capa de anotaciones | Romper postura de cifrado | Sin texto libre en la DB del módulo (§4/§7.1); opt-in wrap si se justifica |
| Uso secundario de datos (dataset, fine-tuning) | Incumplimiento Ley 25.326 | G0.3 previo; fine-tuning excluido (§7.3); `cumplimientos.md` actualizado antes de juntar notas |
| Médico percibe interrupción | Adopción nula | Ghost text, fail-open, revisión diferida, scope por profesional |
| Prerequisite de módulos inexistente | Fork de la arquitectura | G0.5: teleconsulta valida el contrato primero; este módulo no estrena `PluginHost` |

---

## 12. Fuera de alcance

- Codificación automática sin revisión humana (autonomía total).
- Export FHIR / buses provinciales/nacionales (módulo aparte, Fase 6).
- LOINC / laboratorio (iteración posterior del léxico, no se mezcla).
- Extensión veterinaria de SNOMED CT (perfil Vet no incluye este módulo v1).
- Modelos multimodales (audio/voz).
- Fine-tuning con datos de usuarios (§7.3).
- Plugin system de terceros.
- Modificar entries, su firma o el export `.md` del Core.
- SQLCipher o cualquier segundo mecanismo de crypto en disco.

---

## 13. Decisiones abiertas (marcadas, no bloquean el esqueleto)

| # | Decisión | Propuesta | Cuándo resolver |
|---|---|---|---|
| D1 | `module_id`: ¿`semantiar` o `terminologia`? | `semantiar` (Core mínimo: no generalizar "por si acaso" llega LOINC) | Antes de Fase 0 |
| D2 | Scope: ¿`Installation` o `InstallationAndProfessional`? | `InstallationAndProfessional` (no intrusivo por persona) | Antes de Fase 0 |
| D3 | Cifrado en reposo de la capa de anotaciones | Sin texto libre alcanza; wrap opt-in solo si la revisión lo exige | Revisión de Fase 1 |
| D4 | Curación: ¿profesional propone + admin aprueba, o admin directo? | Propone/aprueba (flujo en Fase 3, no antes) | Fase 3 |
| D5 | Pack SNOMED en backup: aceptar tamaño vs. dedup por hash | Aceptar tamaño (cero excepciones al contrato); dedup es optimización futura | Antes de Fase 0 |
| D6 | Shape final de `EntryTextDto` y gate de acceso por entity | §5.1 como base; se define con el refactor `CoreApi` | Con G0.5 |
| D7 | Variantes de `AuditEvent` definitivas | §5.3; PR con test anti-PHI | Fase 0 |
| D8 | Herramienta de generación del pack SNOMED | Verificar `sct`; plan B generador propio desde RF2 | G0.4 |

---

## 14. Resumen ejecutivo

SemantIAr se construye como **módulo Ruta 1** (`fichio-semantiar`, feature
flag, paywall build-time, ediciones Médico/Full), no como subsistema
paralelo al Core. La lógica vive en un crate Rust puro (mobile-ready), los
datos en la DB del módulo (léxico curado + anotaciones sin texto libre) y
un pack SNOMED read-only que entra al backup sin excepciones; la
inferencia, si algún día se justifica, es un sidecar desktop-only que
mobile ignora en silencio. Las entries firmadas nunca se tocan: los
códigos son una capa de anotaciones que referencia `entries.id` (UUID v4)
con trazabilidad completa de versiones. El gate legal (licencias SemantIAr
y SNOMED, base legal del dataset) es previo a cualquier código, y el
prerequisite es que teleconsulta haya validado el contrato de módulos
primero. El éxito se mide en cobertura, precisión y **error de
polaridad/sujeto ≈ 0** sobre notas rioplatenses reales, y en si el médico
de guardia tolera el sistema a las 3 de la mañana. Fase 1 sola ya es
producto; el modelo es una apuesta opcional que el Gate B puede diferir
sin culpa.

---

## Anexo A — Evidencia de verificación de SemantIAr (G0.1, 2026-11-17)

> Verificación hecha sobre el recurso publicado (no sobre paper/tesis).
> El CSV se descargó y analizó completo; el SHA-256 pinea la versión exacta.

### A.1 Identidad y procedencia

| Campo | Valor verificado |
|---|---|
| Recurso | SemantIAr · Repertorio léxico rioplatense (Onto-Lex-SemantIAr) |
| Institución | Proyecto SemantIAr — Universidad Nacional de La Plata (UNLP) |
| Autor | Julián Sánchez Viamonte |
| Repo | https://github.com/manwithbarba/Onto-Lex-SemantIAr |
| Web | https://manwithbarba.github.io/Onto-Lex-SemantIAr/ |
| CSV | https://manwithbarba.github.io/Onto-Lex-SemantIAr/repertorio-lexico-rioplatense.csv |
| Versión | **1.1.0 (2026-11-13)** según LICENSE. La v1 del doc decía "v1.2.0": corregido |
| Estado del repo | 2 commits, 0 stars, 0 forks, maintainer único → riesgo de upstream confirmado |
| SHA-256 del CSV | `d7318d35558c66288920048e90659598c6532657ea16d46fea28045721d43f5b` |
| Copia archivada | `docs/INTEGRATIONS/semantiar/repertorio-lexico-rioplatense-v1.1.0.csv` (byte-idéntica, SHA-256 verificado) + `LICENSE-semantiar.txt` + `README.md` de procedencia |

### A.2 Licencia (texto completo en `LICENSE` del repo)

- **Materiales propios: CC BY 4.0.** Cubre criterios editoriales, reglas de
  delimitación sintáctica (Familias 1–5), expansiones léxicas rioplatenses,
  clasificación en 4 ejes taxonómicos, esquemas OntoLex-Lemon / SHACL /
  JSON Schema, exportaciones de grafo, interfaz web y frecuencias agregadas.
  → Absorción como semilla, curación derivada y redistribución **permitidas
  con atribución** (debe viajar en `pack_registry` y en la UI del módulo).
- **Carve-out SNOMED CT®:** los SCTID y términos preferidos (Edición
  Argentina, español) se incluyen solo para interoperabilidad terminológica,
  docencia e investigación científica reproducible. **El uso en sistemas
  productivos o aplicaciones clínicas está sujeto a la licencia de SNOMED
  International o del Centro Nacional de Referencia (Ministerio de Salud de
  la Nación).** → G0.2 es el gate real de distribución, confirmado por el
  propio recurso.
- **Exención de uso clínico directo:** v1.1.0 declarada libre de texto
  clínico de pacientes e identificadores internos de historia clínica.
  Recurso de referencia y estandarización documental.
- **Core Blind:** prohibido usar el repertorio como ayuda durante rondas de
  anotación humana ciega → restricción metodológica del dataset (§7.2).

### A.3 El CSV real (schema y estadísticas)

UTF-8 **con BOM** (importar como `utf-8-sig`), 2.016 filas, 10 columnas:

```
id, termino, expansion, categoria, modo_codificacion, tema,
variantes (JSON), vinculos_snomed_ct (JSON),
condiciones_de_contexto (JSON), criterio_editorial
```

| Dimensión | Valores |
|---|---|
| `modo_codificacion` | `direct` 1.569 · `context` 126 · `lexical_only` 305 · `excluded` 16 |
| `categoria` | Hallazgo clínico 927 · Procedimiento 609 · Fármaco 159 · Sin categoría 305 · No marcar 16 |
| Temas | 14 (Cardiología 199 · Respiratorio 196 · Escalas/Semiología 464 · Estructura documental 260 · …) |
| SCTIDs distintos | **1.784** (coincide con lo que v1 afirmaba) |
| Relaciones | `sense_preferido` 1.695 · `sense_aceptable` 400 · `sense_contextual` 97 |
| Multi-sentido | 362 entradas con ≥2 vínculos (257×2, 79×3, 22×4, 4×5); 321 sin vínculo SNOMED |
| Familias de id | SEM 1.115 · IP 813 · LXN 41 · R4 47 |

**Reglas de contexto** (presentes en las 2.016 entradas): `positive`,
`negative`, `guards`, `policy`, `uppercase`, `window` (típicamente 8
tokens), `cross_line` (false); 41 entradas traen además `senses`.
Políticas frecuentes: "Asignar el concepto principal" (1.526),
**"Abstenerse si falta contexto" (219)**, "Mapear el parámetro y dejar la
cifra fuera del fragmento" (118), "Conservar la expansión léxica" (58),
"Suprimir la colisión o abstenerse" (54).

### A.4 Ejemplo canónico: FA (SEM-0385) y el SCTID del deep-link

El ejemplo de desambiguación que usaban v1/v2 es una entrada real del
repertorio:

- **FA** (SEM-0385, `context`) → `49436004` fibrilación auricular
  (`sense_preferido`, pistas: ECG, ventricular, latidos, sostenida, ritmo)
  vs `88810008` medición de fosfatasa alcalina (`sense_contextual`,
  pistas: U/L, GGT, bilirrubina, hepatograma, enzimas). Política:
  **"Abstenerse si falta contexto"**, ventana 8 tokens, sin cruzar líneas.
- **`88810008`** (deep-link compartido) = "medición de fosfatasa alcalina":
  sentido directo de **FAL** (SEM-0387) y de **alp** (IP-ABBR-0323, que
  además tiene acepción contextual "lóbulo pulmonar" con guard de
  abstención).

### A.5 Consecuencias de diseño (ya reflejadas en este documento)

1. **Multi-sentido es real** → `lexicon` + `lexicon_senses` (§4), no una
   fila por término con un solo `sctid`.
2. **La abstención es de diseño del recurso** → el motor respeta
   `policy`/`guards`; la métrica de abstención (§7) es obligatoria.
3. **`lexical_only`/`excluded` nunca codifican** (321 entradas) → el
   importador y el lookup respetan `modo_codificacion`.
4. **Capas separables por licencia**: la capa léxica (CC BY 4.0) puede
   distribuirse sin la capa SNOMED (licenciada). Degradación posible si
   G0.2 no se resuelve: subrayado + expansión léxica, sin `sctid`.
5. **Polaridad/temporalidad/sujeto no vienen en la semilla** → son la capa
   de reglas de Fase 1 del módulo, no datos importados.
6. **Atribución CC BY 4.0** obligatoria en `pack_registry` + UI del módulo.
7. **Core Blind**: el gold standard se anota a ciegas sin consultar el
   repertorio (§7.2).
