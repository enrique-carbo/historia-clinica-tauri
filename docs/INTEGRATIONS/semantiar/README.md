# SemantIAr — semilla léxica archivada (v1.1.0)

> **Estado:** activo de investigación archivado. **NO lo consume el Core ni
> ningún build.** No se importa en runtime, no se embebe en el binario, no
> es un placeholder de feature. Su importación a la DB del módulo
> (`fichio-semantiar.db`) es trabajo de **Fase 0 del módulo semantiar**,
> cuando ese módulo exista (ver diseño v2, Gate 0.5).
>
> Se archiva acá, junto a su diseño, por el mismo criterio que
> `src-tauri/tests/fixtures/` congela datos versionados: preservar un
> upstream frágil (repo de 2 commits, maintainer único) con procedencia y
> licencia intactas. Si el repo de módulos se separa, esta carpeta se
> mueve con su diseño.

## Qué es

Repertorio léxico rioplatense del **Proyecto SemantIAr (UNLP)**: 2.016
entradas (siglas, abreviaturas, términos) con expansiones, variantes de
escritura, vínculos a SNOMED CT y reglas de contexto estructuradas
(pistas positivas/negativas, guards, política de abstención, ventana de
tokens). 1.784 SCTIDs distintos. Análisis completo en el **Anexo A** del
diseño v2 (`../SemantIAr_codificacion_clinica_asistida_v2.md`).

## Procedencia

| Campo | Valor |
|---|---|
| Autor | Julián Sánchez Viamonte — Proyecto SemantIAr, Universidad Nacional de La Plata (UNLP) |
| Repo upstream | https://github.com/manwithbarba/Onto-Lex-SemantIAr |
| Web | https://manwithbarba.github.io/Onto-Lex-SemantIAr/ |
| CSV original | https://manwithbarba.github.io/Onto-Lex-SemantIAr/repertorio-lexico-rioplatense.csv |
| Versión | **1.1.0 (2026-11-13)** según LICENSE upstream |
| Fecha de captura | 2026-11-17 |
| SHA-256 del CSV | `d7318d35558c66288920048e90659598c6532657ea16d46fea28045721d43f5b` |

`repertorio-lexico-rioplatense-v1.1.0.csv` es **byte-idéntico** al CSV
upstream (mismo SHA-256). UTF-8 con BOM; columnas JSON en `variantes`,
`vinculos_snomed_ct` y `condiciones_de_contexto`.

## Licencia y atribución (obligatorio)

Texto completo upstream en `LICENSE-semantiar.txt`. Resumen:

- **Materiales propios: CC BY 4.0.** Permite uso, derivados y
  redistribución **con atribución**. La atribución debe viajar con
  cualquier copia o derivado (incluida la futura importación a
  `pack_registry` y la UI del módulo). Texto de atribución sugerido:

  > "SemantIAr · Repertorio léxico rioplatense" por Julián Sánchez
  > Viamonte — Proyecto SemantIAr, Universidad Nacional de La Plata.
  > Licencia CC BY 4.0 (https://creativecommons.org/licenses/by/4.0/).
  > Fuente: https://github.com/manwithbarba/Onto-Lex-SemantIAr

- **Carve-out SNOMED CT®:** los SCTID y términos preferidos (Edición
  Argentina, español) se incluyen solo para interoperabilidad
  terminológica, docencia e investigación. **El uso en sistemas
  productivos o aplicaciones clínicas requiere licencia de SNOMED
  International o del Ministerio de Salud de la Nación.** Este archivo
  es una semilla de investigación, **no un pack de producción
  distribuible**. La distribución de la capa SNOMED está bloqueada por
  el Gate 0.2 del diseño v2.

- **Exención clínica / privacidad:** v1.1.0 declarada libre de texto
  clínico de pacientes e identificadores internos de historia clínica.

- **Core Blind:** prohibido usar el repertorio como ayuda durante rondas
  de anotación humana ciega. El gold standard del dataset de evaluación
  se anota a ciegas, sin consultar este archivo (diseño v2, §7.2).

## Qué NO es

- No es código ni configuración del Core. Nada en `src/` ni `src-tauri/`
  lo referencia.
- No es el pack SNOMED de producción (ese se genera aparte, con licencia
  resuelta — Gate 0.2 / D8 del diseño).
- No es el dataset de evaluación (ese es un activo separado, con su
  propia base legal — Gate 0.3).
