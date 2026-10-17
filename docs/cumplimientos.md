# Cumplimiento Legal — Fichio Care & Fichio Vet

> **Estado**: Borrador v1.0. Fuente de verdad sobre el marco legal aplicable a la
> historia clínica digital en Argentina. No es asesoramiento legal; es una guía
> técnica de qué garantiza el sistema, qué queda a cargo del profesional, y qué
> está fuera de alcance.
> **Revisión**: Anual o ante cambios normativos relevantes.

---

## 1. Principio general: el software no cumple la ley, el profesional sí

Ninguna aplicación garantiza cumplimiento legal por sí sola. La Ley 26.529
obliga al **profesional y al establecimiento** a custodiar la historia clínica,
garantizar el acceso del paciente, y firmar las actuaciones. Fichio es una
herramienta que **facilita** ese cumplimiento, pero no lo sustituye.

**Qué hace Fichio**:
- Almacena la historia clínica de forma cifrada y local (soberanía del dato).
- Permite firmar digitalmente cada entry y el export completo (integridad).
- Genera una copia firmada y auditada para el paciente (derecho de acceso).
- Impide la modificación de entries (append-only, no-repudio).

**Qué NO hace Fichio**:
- No reemplaza el criterio del profesional sobre qué registrar.
- No garantiza que el profesional custodie la semilla en papel (eso es
  responsabilidad de la instalación).
- No gestiona la interoperabilidad con el sistema público (HSI/FHIR) en v1
  (ver §8).
- No es un sistema de receta electrónica homologado por el ReNaPDiS (ver §7).

---

## 2. Marco legal aplicable — Medicina Humana (Fichio Care)

### 2.1 Ley 26.529 — Derechos del Paciente, Historia Clínica y Consentimiento Informado

**Qué regula**: Los derechos del paciente en su relación con los profesionales
e instituciones de salud. Es la ley central de la historia clínica en Argentina.

**Qué exige y cómo se relaciona con Fichio**:

| Exigencia legal | Cómo lo aborda Fichio | Estado |
|---|---|---|
| **Titularidad**: el paciente es titular de su historia clínica (art. 14). | La copia firmada del export se entrega al paciente. El paciente no necesita acceso al sistema para ejercer su derecho. | ✅ Cubierto |
| **Acceso**: copia autenticada en soporte digital o papel, gratuita, en 48 horas (art. 14). | `export_history` genera un `.md` firmado digitalmente (Ed25519) y auditado. El profesional puede imprimirlo o entregarlo digital. | ✅ Cubierto (el flujo es manual, pero la herramienta está) |
| **Conservación**: plazo mínimo de 10 años desde la última actuación (art. 18). | Fichio no borra datos automáticamente. El backup local permite conservar más allá de 10 años. | ⚠️ Depende del backup del profesional |
| **Integridad**: no se puede modificar ni borrar la historia clínica. | Entries append-only, firmadas Ed25519. No hay `UPDATE` ni `DELETE`. | ✅ Cubierto por diseño |
| **Confidencialidad**: secreto profesional, datos sensibles (art. 2 inc. d). | Base local cifrada (AES-GCM-256), DataKey custodiada por el profesional. | ✅ Cubierto (con límites de §6) |
| **Firma**: las actuaciones deben estar firmadas por el profesional. | Firma Ed25519 por entry y del export completo. | ✅ Cubierto digitalmente (ver §4) |

**Nota sobre el plazo de conservación**: La ley establece 10 años como mínimo.
La jurisprudencia reciente (CNCAF, 2025) ha extendido este plazo en casos de
derecho a la identidad (donantes de gametos), considerando que 10 años puede
ser exiguo. **Recomendación**: conservar indefinidamente salvo instrucción
legal en contrario. El backup local lo permite sin costo adicional.

### 2.2 Ley 25.326 — Protección de Datos Personales

**Qué regula**: El tratamiento de datos personales, con protección reforzada
para **datos sensibles** (salud, vida sexual, origen étnico, etc.).

**Qué exige y cómo se relaciona con Fichio**:

| Exigencia legal | Cómo lo aborda Fichio | Estado |
|---|---|---|
| **Datos de salud son sensibles** (arts. 2 y 7): protección más rigurosa. | Cifrado AES-GCM-256 en reposo. DataKey no persiste en claro. | ✅ Cubierto |
| **Consentimiento** para el tratamiento (art. 5). | Fuera del alcance del software. Es responsabilidad del profesional obtenerlo. | ⚠️ Responsabilidad del profesional |
| **Seguridad y confidencialidad**: medidas técnicas y organizativas (art. 9). | Cifrado, control de acceso por rol, auditoría de acciones sensibles. | ✅ Cubierto técnicamente |
| **Secreto profesional** subsiste después de la relación (art. 10). | Los datos permanecen cifrados en la base local. El profesional mantiene la custodia. | ✅ Cubierto |
| **Uso para fines incompatibles** requiere consentimiento (art. 4). | Fichio no envía datos a servidores externos. No hay uso secundario. | ✅ Cubierto por diseño local-first |

**Límite importante**: la Ley 25.326 obliga al **responsable de la base de datos**
(el profesional o la institución) a adoptar medidas de seguridad. Fichio provee
las medidas técnicas (cifrado, control de acceso), pero **no puede garantizar**
que el profesional custodie la semilla, mantenga el backup actualizado, o no
pierda la contraseña. Eso es responsabilidad de la instalación.

### 2.3 Ley 25.506 — Firma Digital

**Qué regula**: El uso de firma digital y firma electrónica, y su eficacia
jurídica.

**Diferencia clave**:
- **Firma digital** (art. 2): usa criptografía asimétrica con certificado
  emitido por un certificador licenciado. Tiene **presunción de autoría y
  integridad** (misma validez que firma manuscrita). La carga de la prueba
  recae en quien impugna.
- **Firma electrónica** (art. 5): cualquier otro método. Tiene **menor eficacia
  probatoria**; la carga de la prueba recae en quien la invoca.

**Cómo se relaciona con Fichio**:
- La firma Ed25519 de Fichio es **firma electrónica** (no es firma digital
  con certificado licenciado). Es criptográficamente sólida, pero **no tiene
  la presunción legal automática** de la firma digital.
- **Para historia clínica electrónica**: la Ley 27.706 (ver §2.4) permite
  que la HCE sea refrendada con firma digital **y/o electrónica**, en los
  términos de la Ley 25.506. Es decir, la firma electrónica es **admisible**
  como medio de prueba, aunque con menor fuerza probatoria.

**Recomendación**: Fichio es **mejor que no firmar** (entries sin firma no
tienen presunción de autoría), pero si el profesional necesita **presunción
legal fuerte**, debe usar firma digital con certificado licenciado (ej.
certificado de la ONTI o de un certificador acreditado). Fichio podría
integrar un módulo de firma digital licenciada en el futuro; hoy no lo hace.

### 2.4 Ley 27.706 — Programa Federal Único de Informatización y Digitalización de Historias Clínicas

**Qué regula**: Crea el programa para digitalizar las historias clínicas en
todo el país, con estándares de interoperabilidad y acceso.

**Qué exige**:
- La HCE puede ser refrendada con firma digital y/o electrónica (Decreto
  393/2023, art. 1).
- Estándares de interoperabilidad: HL7 FHIR como protocolo de intercambio,
  SNOMED CT para terminología (Resolución 189/2018).
- Conjunto de datos mínimos para la HCE.

**Cómo se relaciona con Fichio**:
- ✅ **Firma**: Fichio usa firma electrónica (Ed25519), que la ley admite.
- ⚠️ **Interoperabilidad**: Fichio **no implementa** HL7 FHIR ni SNOMED CT en
  v1. Es un módulo futuro (Fase 6 del roadmap). Para cumplimiento pleno con
  el programa federal, el export a FHIR es necesario, pero no bloqueante para
  la operación local.
- ✅ **Datos mínimos**: el `entity_schema.json` y las entries cubren
  filiación, evolución, prescripciones, estudios. El export `.md` incluye
  todo lo que el profesional registró.

---

## 3. Marco legal aplicable — Veterinaria (Fichio Vet)

### 3.1 Particularidad: no aplica la Ley 26.529

Los animales **no son sujetos de derecho** en el sentido de la Ley 26.529.
No hay "derecho de acceso del paciente" ni "consentimiento informado del
paciente". El tutor o responsable del animal no tiene el mismo estatus legal
que un paciente humano.

**Consecuencia**: Fichio Vet tiene **menos obligaciones legales de datos
humanos**, lo que reduce la fricción regulatoria y permite una adopción más
rápida.

### 3.2 SENASA — Receta Veterinaria Electrónica (RVE)

**Qué regula**: La prescripción de productos veterinarios (medicamentos,
vacunas, etc.) y su trazabilidad. La receta electrónica es obligatoria
para ciertos productos (Resoluciones SENASA 461/2025 y 654/2025).

**Qué exige**:
- La RVE debe ser emitida a través de **SIGTRAZAVET** (sistema de SENASA),
  con un **Código Único de Verificación Electrónica (CUVE)**.
- El profesional debe tener matrícula vigente y estar habilitado por el
  colegio profesional de su jurisdicción.
- La receta debe incluir datos del animal, del tutor, del profesional, y
  del producto prescrito.

**Cómo se relaciona con Fichio**:
- ❌ **Fichio no emite RVE** en v1. La receta veterinaria electrónica es un
  sistema externo de SENASA. Fichio registra la prescripción en la historia
  clínica, pero **no reemplaza** el SIGTRAZAVET.
- ✅ **Historia clínica**: Fichio Vet cumple con el registro de la atención
  (filiación del animal, tutor, profesional, evolución). Es un **complemento**
  a la RVE, no un sustituto.
- ⚠️ **Futuro**: un módulo de integración con SIGTRAZAVET podría automatizar
  la emisión de RVE desde Fichio. No está en el roadmap inmediato.

### 3.3 Proyecto de Ley de Historia Clínica Electrónica Veterinaria (CABA)

**Estado**: Proyecto en la Legislatura de CABA (no vigente). Si se aprueba,
exigiría una HCEV obligatoria, unificada, con plataforma pública y firma
digital por asiento.

**Cómo se relaciona con Fichio**:
- Fichio Vet ya tiene los elementos técnicos (firma por entry, registro
  cronológico, export). Si el proyecto se convierte en ley, Fichio podría
  adaptarse con un módulo de cumplimiento.
- Hoy no es obligatorio.

---

## 4. Firma Digital y No-Repudio — Estado real

| Aspecto | Fichio | Nota |
|---|---|---|
| **Firma por entry** | Ed25519 (firma electrónica) | Criptográficamente sólida, pero no es firma digital licenciada. |
| **Firma del export** | Ed25519 del documento completo | Verificable con la pública del profesional. |
| **Presunción legal** | Firma electrónica (art. 5, Ley 25.506) | Menor fuerza probatoria que firma digital licenciada. |
| **No-repudio** | Fuerte a nivel técnico (clave privada nunca sale de RAM) | Depende de que el profesional custodie su contraseña/vault. |
| **Integridad** | SHA-256 + Ed25519 | Cualquier alteración rompe la firma. |
| **Timestamp** | Fecha de la entry + `created_at` | No hay sello de tiempo de un TSA licenciado. |

**Recomendación para el profesional**: si necesita **presunción legal fuerte**
(ej. para juicios, pericias), use firma digital licenciada. Fichio es
suficiente para la operación diaria y para la copia del paciente, pero no
reemplaza un certificado licenciado en contextos de alta disputa legal.

---

## 5. Derecho de Acceso del Paciente — Cómo se ejerce con Fichio

La Ley 26.529 (art. 14) obliga a entregar copia autenticada en 48 horas.
Fichio lo facilita así:

1. El profesional (o el admin) abre la ficha del paciente en Fichio.
2. Ejecuta `export_history`.
3. El sistema genera un `.md` firmado digitalmente, con:
   - Datos del paciente.
   - Perfil del profesional.
   - Todas las entries cronológicas.
   - Footer con SHA-256 y firma Ed25519.
4. El profesional imprime el `.md` o lo entrega en digital.
5. La emisión queda registrada en `audit_log` (`export_emitted`).

**Lo que el paciente recibe**: un documento legible, completo, con firma
verificable. Es **mejor que un PDF sin firma** y **equivalente** a una copia
en papel firmada a mano (en términos de autenticidad).

**Lo que el paciente NO recibe**: acceso directo al sistema. El rol
`paciente` en Fichio no tiene DataKey y no puede leer las entries. El acceso
es **mediado por el profesional**, que es quien ejerce la custodia.

**Limitación honesta**: el derecho de acceso es del paciente, pero Fichio
requiere que el profesional ejecute el export. Si el profesional se niega,
el paciente debe recurrir a la justicia (acción de amparo). Fichio no
resuelve el conflicto humano; solo provee la herramienta para cumplir.

---

## 6. Límites de la protección criptográfica (honestidad)

Fichio protege el **disco**: sin contraseña o semilla, la base local es
incomprensible para alguien con acceso físico. **No protege**:

| Escenario | Qué pasa |
|---|---|
| **Co-usuarios de la misma instalación** | La DataKey es compartida por diseño. Un `asistente` con la bóveda abierta puede, técnicamente, leer entries clínicas. El control es de UI, no criptográfico. |
| **Volcado de memoria con bóveda abierta** | Las llaves viven en RAM. Un dump del proceso expone las llaves. `zeroize` limpia al liberar, pero no mientras están en uso. |
| **Semilla en papel perdida o robada** | Quien tenga la semilla puede hacer `seed_recovery` y resetear la contraseña del admin. Está auditado, pero no prevenido. |
| **Swap / hibernación** | Si el OS swappea el proceso, las llaves pueden quedar en disco sin cifrar. Fichio no usa `mlock`. |

Estos límites están documentados en el README (§"Límites de zeroize") y son
inherentes a cualquier aplicación que necesite usar las llaves en RAM.

---

## 7. Receta Electrónica (Medicina Humana) — Fuera de alcance v1

La Ley 27.553 y la Resolución 2214/2025 exigen que las prescripciones se
emitan **exclusivamente en formato electrónico** a través de plataformas
registradas en el **ReNaPDiS** (Registro Nacional de Plataformas Digitales
de Salud), con una **Clave Única de Identificación de Receta (CUIR)**.

**Fichio no es una plataforma de receta electrónica** en v1:
- No está registrado en ReNaPDiS.
- No emite CUIR.
- No se integra con farmacias.

**Qué puede hacer el profesional**:
- Usar Fichio para la historia clínica y la evolución.
- Emitir la receta electrónica a través de una plataforma separada (ej.
  Receta Digital del colegio médico, o una plataforma privada registrada).
- Registrar en Fichio que se emitió una receta (entry `MEDICATION`), con
  referencia al número de receta externa.

**Futuro**: un módulo de integración con ReNaPDiS podría habilitar la emisión
de recetas desde Fichio. No está en el roadmap inmediato (Fase 5+).

---

## 8. Interoperabilidad (HSI / FHIR) — Fuera de alcance v1

La Ley 27.706 y la Resolución 189/2018 establecen estándares de
interoperabilidad: **HL7 FHIR** para intercambio, **SNOMED CT** para
terminología, y un **conjunto de datos mínimos** para la HCE.

**Fichio no implementa FHIR en v1**:
- El export es `.md` (legible, firmado, pero no interoperable).
- No hay mapeo a SNOMED CT (las entries usan texto libre + templates JSON).
- No hay conexión con la HSI (Historia de Salud Integrada) ni con el
  federador nacional de pacientes.

**Impacto**: para un consultorio que solo necesita su historia local, no es
bloqueante. Para una institución que debe reportar al sistema público, Fichio
no cumple todavía.

**Futuro**: el módulo de export FHIR es Fase 6 del roadmap. La arquitectura
de módulos (remote-ready) permite agregarlo sin tocar el core.

---

## 9. Checklist de cumplimiento para el profesional

Antes de usar Fichio en un consultorio real:

- [ ] **Semilla en papel**: generada, anotada, guardada en lugar seguro
      (caja fuerte, caja de seguridad). No en la misma máquina.
- [ ] **Backup periódico**: configurado y probado (restore). Frecuencia
      recomendada: semanal.
- [ ] **Contraseña del admin**: fuerte, única, no compartida.
- [ ] **Roles**: `administrador` solo para quien gestiona usuarios; el
      `profesional` para quien atiende; `asistente` para recepción.
- [ ] **Consentimiento informado**: obtenido por fuera de Fichio (papel o
      sistema aparte). Fichio no lo gestiona.
- [ ] **Receta electrónica**: emitida por plataforma ReNaPDiS (Fichio no
      la emite).
- [ ] **Copia al paciente**: solicitada por el paciente, generada con
      `export_history`, entregada en 48h.
- [ ] **Conservación**: 10 años mínimo; recomendado indefinido. El backup
      local lo permite.
- [ ] **Firma digital licenciada**: si se necesita presunción legal fuerte,
      complementar con certificado licenciado.

---

## 10. Lo que Fichio NO garantiza (resumen)

| Área | Lo que no garantiza |
|---|---|
| **Cumplimiento legal** | No reemplaza al profesional. La ley obliga al profesional, no al software. |
| **Firma digital licenciada** | La firma Ed25519 es electrónica, no digital con certificado licenciado. Menor presunción legal. |
| **Receta electrónica** | No emite recetas ReNaPDiS (Ley 27.553). |
| **Interoperabilidad** | No implementa FHIR/SNOMED/HSI en v1. |
| **Consentimiento informado** | No lo gestiona. Es responsabilidad del profesional. |
| **Custodia de la semilla** | No puede impedir que el profesional la pierda o la comparta. |
| **Backup** | No lo ejecuta automáticamente. Depende del profesional. |
| **Co-usuarios** | La DataKey es compartida. El filtrado por rol es de UI, no criptográfico. |
| **Plazo de conservación** | No borra automáticamente, pero tampoco garantiza que el profesional no borre la base. |

---

## 11. Glosario mínimo

- **HCE**: Historia Clínica Electrónica.
- **RVE**: Receta Veterinaria Electrónica (SENASA).
- **CUIR**: Clave Única de Identificación de Receta (medicina humana).
- **CUVE**: Código Único de Verificación Electrónica (veterinaria).
- **ReNaPDiS**: Registro Nacional de Plataformas Digitales de Salud.
- **SIGTRAZAVET**: Sistema de trazabilidad de SENASA para productos
  veterinarios.
- **HSI**: Historia de Salud Integrada (sistema público de interoperabilidad).
- **FHIR**: Fast Healthcare Interoperability Resources (estándar HL7).
- **Firma digital vs. electrónica**: digital = certificado licenciado
  (presunción legal fuerte); electrónica = cualquier otro método (menor
  presunción).

---

## 12. Referencias normativas

- **Ley 26.529** — Derechos del Paciente, Historia Clínica y Consentimiento
  Informado.
- **Ley 25.326** — Protección de Datos Personales.
- **Ley 25.506** — Firma Digital.
- **Ley 27.553** — Receta Electrónica.
- **Ley 27.706** — Programa Federal Único de Informatización y Digitalización
  de Historias Clínicas. Decreto 393/2023.
- **Resolución 189/2018** — Estándares de interoperabilidad (HL7 FHIR,
  SNOMED CT).
- **Resolución 2214/2025** — Receta electrónica (ReNaPDiS, CUIR).
- **Resoluciones SENASA 461/2025 y 654/2025** — Receta Veterinaria
  Electrónica (RVE, SIGTRAZAVET, CUVE).
- **Proyecto de Ley HCEV (CABA)** — Historia Clínica Electrónica Veterinaria
  (no vigente).
