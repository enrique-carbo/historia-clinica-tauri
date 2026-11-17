# Roadmap: Integración de SemantIAr · Codificación Clínica Asistida en Fichio

> **Estado:** Archivada (v1). Superada por
> `SemantIAr_codificacion_clinica_asistida_v2.md`, que reencuadra la
> integración como módulo bajo `docs/module-contract.md`. Se conserva
> como referencia histórica.
> **Ámbito:** Pipeline de normalización léxica y codificación SNOMED CT, 100% local-first
> **Principio rector:** La comodidad del médico en el ingreso de datos es prioridad absoluta. La codificación es invisible, no intrusiva y fail-open.

---

## 1. Objetivo

Permitir que Fichio transforme el texto libre de las notas clínicas (español rioplatense) en **datos estructurados y codificados a SNOMED CT**, sin salir del dispositivo, sin bloquear la escritura y sin degradar la experiencia del profesional.

El recurso léxico de partida es **SemantIAr** (repertorio rioplatense, ~2.016 entradas, ~1.784 códigos SNOMED CT activos), tratado como **corpus semilla** —no como diccionario de producción— y como fuente de metodología de anotación (polaridad, certeza, temporalidad, sujeto).

---

## 2. Principios de Diseño (no negociables)

| Principio | Implicancia |
|---|---|
| **Local-first** | Todo el pipeline corre en el dispositivo. Sin APIs en la nube. Sin latencia de red. |
| **Fail-open** | Si un pack no carga o el modelo falla, Fichio funciona igual, en silencio, sin errores visibles. |
| **No intrusivo** | Sugerencias como *ghost text* inline. Aceptar con Tab, ignorar escribiendo. Cero modales. |
| **Cada fase es útil por sí sola** | Toda fase debe poder desactivarse sin romper la anterior. |
| **Trazabilidad** | Toda nota codificada registra con qué versión de pack/ontología/modelo se procesó. |
| **Medir antes de escalar** | Ninguna fase avanza sin métricas sobre datos reales anotados. |

---

## 3. Decisión de Arquitectura

> **Crate para la lógica · SQLite separado para los datos · Sidecar para el modelo · "Pack" (no plugin) como concepto de extensión.**

### 3.1 Componentes

| Componente | Forma | Responsabilidad | Por qué |
|---|---|---|---|
| **`semantiar-core`** | Crate Rust in-process | Tokenización, lookup léxico, reglas de contexto (negación, temporalidad, sujeto), decisión de match | Instantáneo (µs). Sin IPC. Testeable. |
| **Pack léxico SemantIAr** | SQLite read-only, versionado, firmado | Términos, siglas, `sctid`, reglas de contexto, categorías | Ciclo de vida y cifrado distintos a la base clínica. Actualizable sin recompilar. |
| **Pack ontología SNOMED CT** | SQLite read-only, generado con `sct` | Jerarquía, sinónimos, `sctid` activos | Consultable offline en ms. Sin Java/Elasticsearch. |
| **Motor de inferencia** | Sidecar (proceso hijo, IPC JSON-lines) | Desambiguación de baja confianza, clasificación, extracción de spans | Aislamiento de fallos, release independiente, memoria separada. |
| **Feedback log** | SQLite local (append-only) | Aceptación/rechazo de sugerencias | Alimenta curación del pack y fine-tuning futuro. |

### 3.2 Lo que **NO** se hace

- ❌ **No plugin system.** Fichio no necesita extensiones de terceros; necesita actualizar sus propios packs de datos. Un sistema de plugins (ABI, sandboxing, descubrimiento) es sobre-ingeniería.
- ❌ **No modelo in-process.** Un OOM o panic del modelo no puede tumbar el EMR en medio de una consulta.
- ❌ **No LLM generativo para codificar.** Alucina códigos, no es auditable. Solo modelos de decisión con respuestas tipadas y probabilidades calibradas.
- ❌ **No dependencia de servicios remotos** de terminología (Snowstorm, APIs de SNOMED).

### 3.3 Separación de archivos en disco

```
historia_clinica.db                      # base clínica (cifrada)
packs/
  semantiar_rioplatense_vX.sqlite   # léxico (read-only, firmado, versionado)
  snomed_sct_vYYYYMM.sqlite         # ontología (read-only, generado por sct)
  feedback.db                       # log de aceptación/rechazo
sidecar/
  decision-model                    # binario del motor de inferencia
```

---

## 4. Fases del Roadmap

### Fase 0 — Fundación y Medición (sin modelos)

**Objetivo:** Tener el suelo firme y la línea de base medida antes de escribir lógica inteligente.

- [ ] Construir **dataset de evaluación**: 200–500 notas reales desidentificadas, anotadas por médicos rioplatenses (menciones + `sctid` + polaridad/certeza/temporalidad).
- [ ] Empaquetar SemantIAr v1.2.0 como **pack SQLite** con FTS5.
- [ ] Generar **pack SNOMED CT** local con `sct`.
- [ ] Implementar `semantiar-core` con **lookup exacto** de sigla/forma breve.
- [ ] UI: subrayado pasivo de menciones reconocidas + tooltip con `sctid`.
- [ ] **Medir cobertura y precisión** contra el dataset de evaluación.

**Criterio de salida:** Cobertura ≥ 60% de menciones frecuentes con precisión ≥ 0.90. Si no se alcanza, **no avanzar** hasta entender por qué.

---

### Fase 1 — Reglas de Contexto (sin modelos)

**Objetivo:** Resolver la mayor parte de la ambigüedad real con puro código.

- [ ] Detección de **negación** ("no FA", "niega").
- [ ] Detección de **temporalidad** ("antecedente de", "actual").
- [ ] Detección de **sujeto** ("madre refiere", "el paciente").
- [ ] **Desambiguación por contexto léxico** (ej. FA + "ECG" → fibrilación; FA + "laboratorio" → fosfatasa alcalina).
- [ ] Codificación silenciosa en background al guardar la nota.
- [ ] Panel de **auditoría** (revisión diferida, no en el flujo de escritura).
- [ ] **Medir delta** de precisión respecto a Fase 0.

**Criterio de salida:** Precisión ≥ 0.92 en menciones cubiertas. Fichio es **100% funcional sin sidecar** al finalizar esta fase.

---

### Fase 2 — Sidecar de Inferencia (solo para lo ambiguo)

**Objetivo:** Cubrir el residual que las reglas no resuelven, sin comprometer el flujo.

- [ ] Definir **protocolo IPC** (JSON-lines sobre stdin/stdout o socket local).
- [ ] Integrar **un** modelo de decisión —candidatos evaluados: `d1.2` (Liquid AI), `GLiNER-2.5-Decide`, `MIMIR`, `Beveridge`— seleccionado por benchmark sobre el dataset de evaluación.
- [ ] Invocación **solo** cuando `semantiar-core` reporta baja confianza o ambigüedad no resuelta.
- [ ] Sugerencias del modelo aparecen como **segunda opción** en UI, nunca reemplazan la principal.
- [ ] **Fail-open verificado:** matar el sidecar no afecta a Fichio.
- [ ] Empaquetado y actualización firmada del binario del sidecar.
- [ ] **Medir delta** de cobertura y precisión.

**Criterio de salida:** Ganancia medible de precisión/cobertura sobre Fase 1, con latencia percibida nula en el flujo de escritura.

---

### Fase 3 — Feedback Loop y Curación

**Objetivo:** Que el sistema mejore con el uso, sin infraestructura nueva.

- [ ] Log local de **aceptación/rechazo** de sugerencias.
- [ ] Herramienta de **curación del pack** a partir del feedback (términos faltantes, mapeos erróneos).
- [ ] Pipeline de **fine-tuning** del modelo con corpus SemantIAr + feedback agregado (offline, opt-in).
- [ ] Versionado y **firma de packs** con mecanismo de actualización segura.

**Criterio de salida:** Ciclo de mejora demostrable: nueva versión de pack/modelo mejora métricas sobre el dataset de evaluación.

---

## 5. Estrategia de Evaluación

**El dataset de evaluación es el activo más valioso del proyecto — más que el repertorio mismo.**

| Métrica | Descripción | Umbral Fase 0 | Umbral Fase 2 |
|---|---|---|---|
| Cobertura | % de menciones reales con código asignado | ≥ 60% | ≥ 80% |
| Precisión | % de códigos asignados correctos | ≥ 0.90 | ≥ 0.95 |
| Recall | % de menciones reales detectadas | — | ≥ 0.85 |
| Latencia lookup | tiempo por mención (p95) | < 5 ms | < 5 ms |
| Latencia sidecar | tiempo por invocación (p95) | — | < 200 ms (background) |
| Tasa de abstención | % de casos donde el sistema no sugiere | — | monitorear |

> **Regla:** ningún paper (d1.2, GLiNER, MEDSPANER) reporta rendimiento sobre notas de la HSI bonaerense. Los F1 de 0.9 caen a 0.6–0.7 en datos de otra institución. **Solo el dataset propio es evidencia.**

---

## 6. Riesgos y Mitigaciones

| Riesgo | Impacto | Mitigación |
|---|---|---|
| SemantIAr es pequeño (~2.016 entradas) | Cobertura insuficiente | Tratarlo como semilla; curación continua vía Fase 3 |
| SemantIAr deja de mantenerse (tesis doctoral) | Pérdida de upstream | **Absorber el repertorio como dato propio** bajo licencia adecuada desde el inicio |
| Licencia LFM (umbral 10M USD) | Restricción comercial futura | Preferir Apache 2.0 (GLiNER) o MIMIR si el benchmark es competitivo |
| `sct` es proyecto de un desarrollador | Riesgo de mantenimiento | Congelar versión + generar packs propios; fallback a Snowstorm Lite on-premise |
| Sobre-arquitectura | Nunca se lanza | Disciplina de fases; cada una útil por sí sola |
| Modelo consume memoria en dispositivos modestos | OOM | Sidecar aislado; modelos ≤ 2B; cuantización 4-bit |
| Médico percibe interrupción | Adopción nula | Ghost text, fail-open, revisión diferida, panel de auditoría |

---

## 7. Dependencias Externas

| Dependencia | Uso | Licencia / Riesgo |
|---|---|---|
| SemantIAr v1.2.0 | Corpus léxico semilla | Verificar términos de uso; absorber como dato propio |
| SNOMED CT (Edición Nacional Argentina) | Ontología de referencia | Requiere licencia de Miembros de SNOMED International (vía Argentina) |
| `sct` (pacharanero) | Generación de pack local | Open source; riesgo de maintainer único |
| Modelo de decisión (d1.2 / GLiNER / MIMIR) | Inferencia sidecar | LFM Open License / Apache 2.0 / verificar |
| SQLite + FTS5 / SQLCipher | Almacenamiento | Dominio público / BSD |

---

## 8. Hitos de Decisión (gates)

1. **Gate A (fin Fase 0):** ¿La cobertura del pack SemantIAr justifica seguir? Si no, priorizar curación del léxico antes que inferencia.
2. **Gate B (fin Fase 1):** ¿Las reglas de contexto alcanzan precisión aceptable? Si sí, el sidecar es opcional y puede diferirse.
3. **Gate C (fin Fase 2):** ¿El modelo elegido mejora métricas de forma medible sobre Fase 1? Si no, descartarlo sin culpa.
4. **Gate D (continuo):** ¿La experiencia del médico se mantiene no intrusiva? Cualquier regresión en usabilidad bloquea el avance.

---

## 9. Fuera de Alcance (por ahora)

- Codificación automática sin revisión humana (autonomía total).
- Interoperabilidad con buses provinciales/nacionales (fase posterior).
- Modelos multimodales (audio/voz) — evaluar `d1-omni` solo si hay demanda clínica.
- Plugin system de terceros.
- Fine-tuning en la nube.

---

## 10. Resumen Ejecutivo

La integración se construye en **cuatro fases incrementales**, cada una útil y desactivable de forma independiente. La arquitectura separa **lógica (crate)**, **datos (packs SQLite)** e **inferencia (sidecar)**, evitando tanto el acoplamiento del monolito como la complejidad del plugin system. El éxito no se mide en F1 de papers sino en **cobertura y precisión sobre notas reales rioplatenses** y en **si el médico de guardia tolera el sistema a las 3 de la mañana**. La disciplina de medir antes de escalar y de absorber el repertorio como dato propio son las dos decisiones que determinan la viabilidad a largo plazo.

