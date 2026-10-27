# 📖 Guía de Usuario de Fichio

> Guía completa para usar **Fichio Care** y **Fichio Vet** — historia clínica
> digital *offline-first*. Sirve para ambas apps: donde diga "Fichio" aplica
> a los dos perfiles; las diferencias están en
> [§13 Diferencias de Fichio Vet](#13-diferencias-de-fichio-vet).
>
> Los nombres de botones y pantallas están tomados tal cual de la app — si
> algo no coincide, es un bug, reportalo (ver [§12](#12-reportar-problemas)).

**Contenido**

1. [Qué es Fichio y qué no es](#1-qué-es-fichio-y-qué-no-es)
2. [Instalación](#2-instalación)
3. [Primer inicio: administrador y frase semilla](#3-primer-inicio-administrador-y-frase-semilla)
4. [Conocé la pantalla principal](#4-conocé-la-pantalla-principal)
5. [Tu perfil profesional](#5-tu-perfil-profesional)
6. [Pacientes: alta y búsqueda](#6-pacientes-alta-y-búsqueda)
7. [Entries: el registro clínico](#7-entries-el-registro-clínico)
8. [Historia Clínica: la ficha del paciente](#8-historia-clínica-la-ficha-del-paciente)
9. [Exportar la historia clínica](#9-exportar-la-historia-clínica)
10. [Administración](#10-administración)
11. [Acceso y recuperación](#11-acceso-y-recuperación)
12. [Reportar problemas](#12-reportar-problemas)
13. [Diferencias de Fichio Vet](#13-diferencias-de-fichio-vet)
14. [Buenas prácticas de seguridad](#14-buenas-prácticas-de-seguridad)
15. [Glosario](#15-glosario)

---

## 1. Qué es Fichio y qué no es

**Fichio es**: una historia clínica que corre **en tu computadora, sin internet
ni cuentas en la nube**. Los datos clínicos (fichas, entries) se guardan
**cifrados** y cada entry clínica queda **firmada digitalmente**. Nadie — ni
Fichio, ni tu proveedor de internet — puede leerlos; solo quienes tienen la
contraseña de la instalación.

**Fichio no es** (todavía, en esta beta):

- ❌ **No sincroniza entre equipos**: cada computadora es un mundo. El
  **respaldo** es tu única copia.
- ❌ **No tiene auto-actualización**: las versiones nuevas se descargan e
  instalan encima (los datos quedan — [§14](#14-buenas-prácticas-de-seguridad)).
- ❌ **No exporta a PDF** (sí a Markdown firmado — [§9](#9-exportar-la-historia-clínica)).
- ❌ **No es un sistema de turnos ni facturación**: es historia clínica.
- ⚠️ **Las entries no se editan ni se borran** — son inmutables y firmadas
  (integridad clínica). Para corregir un error, creás una entry nueva; la
  anterior queda como constancia.

<!-- CAPTURA: 01-app-hero — Pantalla de login con el título de la app -->

---

## 2. Instalación

### Windows (10 / 11, 64-bit)

1. Descargá el instalador desde la
   [página de la release](https://github.com/enrique-carbo/historia-clinica-tauri/releases)
   (`Fichio Care_0.1.0_x64-setup.exe` o `Fichio Vet_0.1.0_x64-setup.exe`).
2. Doble clic → **Instalar**.
3. ⚠️ **Windows va a advertir** ("Windows protegió tu computadora" o "App no
   reconocida"): el binario aún no tiene certificado comercial de firma.
   Es esperado en la beta — hacé clic en **Más información → Ejecutar de
   todas formas**.
4. Si falta WebView2, el instalador lo descarga solo (única vez que
   necesitás internet).

También podés instalar Care y Vet en la misma PC: son coinstalables.

### Linux (amd64)

| Querés | Usá |
|---|---|
| Debian / Ubuntu | archivo `.deb` |
| Cualquier distro | archivo `.AppImage` |

- **.deb**: doble clic en el gestor de paquetes, o
  `sudo apt install ./Fichio.Care_0.1.0_amd64.deb`
- **AppImage**: `chmod +x *.AppImage` y doble clic (no requiere instalación).
- Si queja de dependencias en Debian/Ubuntu:
  `sudo apt install libwebkit2gtk-4.1-0`

**Requisitos**: ~100 MB de disco; sin internet (salvo instalar/actualizar).

<!-- CAPTURA: 02-instalacion — Icono de Fichio en el menú de aplicaciones de Windows o el gestor de paquetes -->

---

## 3. Primer inicio: administrador y frase semilla

Al abrir Fichio por primera vez la instalación está vacía.

### 3.1 Crear el primer administrador

1. Aparece **"🌱 Crear Primer Administrador"**.
2. Elegí un **usuario** (ej. `admin_principal`) y una **contraseña de mínimo
   8 caracteres**.
3. Botón **"Crear Administrador"** — entra directo al sistema.

> ℹ️ Esta cuenta es la **dueña de la gestión técnica**: da de alta al resto
> del equipo, resetea contraseñas y rota la frase semilla. **No hay
> registro libre después**: todos los demás usuarios los crea el
> administrador.

<!-- CAPTURA: 03-crear-admin — Formulario "Crear Primer Administrador" -->

### 3.2 La frase semilla (muy importante)

La app te pide configurar la **frase semilla de 6 palabras**. Es el respaldo
maestro: si algún día se olvida **toda** la contraseña, esa frase es la única
forma de recuperar los datos.

1. **"Generar Frase Semilla"** — la app muestra 6 palabras.
2. 📝 **Anotalas en papel** (no fotos, no archivos, no WhatsApp). Si perdés la
   frase y la computadora se daña, **los datos son irrecuperables**.
3. **"Continuar"** → re-escribí las 6 palabras para verificar →
   **"Verificar"**.
4. Listo: **"¡Frase Verificada!"**

<!-- CAPTURA: 04-frase-semilla — Pantalla "Tu Frase Semilla" con las 6 palabras (usar palabras de ejemplo, NUNCA una frase real) -->

> 🔒 La frase **no se guarda en ningún lado**: solo existe en tu papel y en la
> RAM mientras usás la app. Si alguien la tiene, tiene acceso a los datos —
> tratala como dinero en efectivo.

---

## 4. Conocé la pantalla principal

<!-- CAPTURA: 05-pantalla-principal — Vista con Perfil o Paciente abierto, mostrando header y pestañas -->

**Arriba (header)**:

- **Fichio Care / Fichio Vet** + tu rol.
- **🌙 / ☀️**: tema oscuro / claro (se esconde en pantallas chicas).
- **"Usuario: …"** y el botón **"🔒 Cerrar"** (cierra la bóveda: borra las
  claves de la memoria — siempre cerrá al terminar).

<!-- CAPTURA: 06-drawer — Menú lateral abierto mostrando los items del rol administrador -->

**Menú lateral (☰)** según tu rol:

| Rol | Menú |
|---|---|
| **Administrador** | Usuarios · Admisión de Pacientes · Auditoría · Frase Semilla · Respaldos |
| **Asistente** | Admisión de Pacientes |
| **Profesional** | Perfil · Paciente · Historia Clínica · Entries |
| **Paciente** | Mi Perfil (solo cambio de contraseña — no ve datos clínicos) |

Los profesionales trabajan en **4 pestañas**: **Perfil** 👨🏻‍⚕️ · **Paciente**
👤 · **Historia Clínica** 📋 · **Entries** ⏱️.

---

## 5. Tu perfil profesional

Pestaña **Perfil** → **"👤 Datos del profesional"** → botón **"Editar"**.

- **Identidad Clínica**: Nombre completo, Profesión (lista: Médico/a,
  Enfermero/a, Lic. en Psicología, Kinesiólogo/a, …), Especialidad.
- **Matrículas Profesionales**: Número (ej. `MP-12345`) y Jurisdicción
  (ej. `Córdoba`); podés agregar varias con **"+ Agregar matrícula"** (la
  primera es la principal).
- **Datos de Contacto**: domicilio, ciudad, país, teléfono de consultorio,
  correo, sitio web / agenda.
- **"Guardar Perfil"**.

Abajo está **"🔒 Seguridad — Cambiar contraseña"**: contraseña actual +
nueva (mínimo 8) + repetir → **"Cambiar Contraseña"**. Tu frase semilla **no
se ve afectada** por este cambio.

> ⚠️ **Completá nombre, profesión y matrícula antes de crear entries**: si
> no, la pestaña Entries te lo pide con un botón **"Completar mi perfil"**.

<!-- CAPTURA: 07-perfil — Perfil profesional en modo edición con matrículas -->

---

## 6. Pacientes: alta y búsqueda

Pestaña **Paciente** (profesionales) o **Admisión de Pacientes**
(administrador / asistente). A la izquierda está el **"Padrón de Pacientes"**.

<!-- CAPTURA: 08-admision — Padrón con lista de pacientes a la izquierda y formulario a la derecha -->

### Buscar

- **"Buscar por nombre, teléfono..."**: búsqueda por texto.
- **"DNI / Documento exacto..."** 🔍 (en Vet: **"N° Registro / Microchip
  exacto..."**): búsqueda **exacta por el documento** (ideal para
  recepción — encuentra siempre al paciente correcto, sin coincidencias
  parciales). El nombre del campo lo define cada perfil.

### Crear una ficha

1. Formulario **"➕ Nuevo Paciente"**.
2. Completá los campos (para **Fichio Care**): Nombre, Apellido,
   **DNI / Documento** (obligatorio), Fecha de Nacimiento, Teléfono,
   Correo, Dirección, Ciudad, Provincia, Seguro Salud, Afiliado y — si
   corresponde — el grupo **Responsable - Tutor**. *En Fichio Vet los
   campos son otros: ver [§13](#13-diferencias-de-fichio-vet).*
3. **"Crear y Cifrar"** → aparece `✅ Creado: ID=…`.

### Editar o seleccionar

- Clic en un paciente de la lista: lo **seleccionás** (queda activo para la
  Historia Clínica y las Entries).
- ✏️ **"Editar paciente"**: carga la ficha en el formulario → cambios →
  **"Guardar Cambios"** (o **"Cancelar Edición"**).

### Acceso del paciente a la app (administrador / asistente)

Con una ficha existente seleccionada aparece **"🎟️ Acceso del paciente"**:

- **"Dar acceso al paciente"** → Usuario + Contraseña temporal (mínimo 8) →
  **"Otorgar"**. Esa persona podrá entrar con rol `paciente`
  ([§11](#11-acceso-y-recuperación)).
- Para quitarlo: desactivalo desde **Usuarios** (no se borra nada).

<!-- CAPTURA: 09-acceso-paciente — Bloque "🎟️ Acceso del paciente" con el formulario -->

---

## 7. Entries: el registro clínico

Las **entries** son la evolución clínica: inmutables, cifradas y firmadas.
Se crean desde la pestaña **Entries** (con un paciente seleccionado).

> ℹ️ Si no tenés perfil completo te lo pide antes (ver [§5](#5-tu-perfil-profesional)).

### Crear una entry

<!-- CAPTURA: 10-nueva-entry — Formulario de Nueva Entry con las 4 categorías -->

1. Pestaña **Entries** → botón **"Nueva Entry"**.
2. Elegí la categoría:

   | Botón | Para qué sirve |
   |---|---|
   | **Nota SOAP** | Evolución estructurada: **S**ubjetivo, **O**bjetivo, **A**nálisis/Evaluación, **P**lan |
   | **Alergia** | Sustancia, tipo de reacción (Leve / Moderada / Severa / Anafilaxia), descripción |
   | **Medicación** | Medicamento, dosis, frecuencia, vía de administración, observaciones |
   | **Diagnóstico** | Diagnóstico, código CIE-10, estado (Activo / En tratamiento / Resuelto / Crónico) |

3. **Título** (resumen) y **Estado** (Activo / Resuelto / Completado) +
   los campos de la categoría.
4. **"Crear Entry"**.
5. Para volver a la lista: **"Ver Timeline"** (el formulario se cierra con
   el mismo botón que lo abrió).

### Leer y filtrar

<!-- CAPTURA: 11-timeline — Timeline con filtros de categoría, búsqueda y una entry desplegada -->

- Filtros: **📊 Todas · 📋 SOAP · ⚠️ Alergias · 💊 Medicación · 🏥
  Diagnóstico**.
- Búsqueda **"Buscar por título..."** (se actualiza mientras escribís).
- **"Cargar más (N cargadas)"** si hay muchas.
- Clic en una card: se despliega con el contenido completo
  (en SOAP: `S — Subjetivo`, `O — Objetivo`, …).

### Firma

Cada card muestra **"Firmado: {nombre} · {profesión}"** y:

- ✅ **Verificada** — la firma digital coincide con la clave del autor.
- ⚠️ **Sin verificar** — la entry fue creada antes del histórico de claves o
  hay un problema de integridad. **Reportalo**.

> 🔒 Las entries **no se pueden editar ni borrar**. Si te equivocaste, creá
> una nueva entry que deje constancia (ej. "Corrección de nota del …").

---

## 8. Historia Clínica: la ficha del paciente

Pestaña **Historia Clínica** — vista unificada del paciente seleccionado.

<!-- CAPTURA: 12-historia-clinica — Vista Historia Clínica con el bloque de alergias y el historial SOAP -->

- **Izquierda**: buscador de pacientes **"Buscar por nombre, DNI /
  Documento o teléfono..."** (elegí acá a quién mirar).
- **Encabezado del paciente**: nombre, documento, edad, y al desplegar los datos
  de contacto (teléfono, correo, dirección, seguro/afiliado, tutor…).
- **⚠️ Alergias** (siempre visible): lista las alergias registradas con su
  severidad y quién la firmó — **"Ninguna registrada"** en verde si no hay.
  Es el primer dato clínico que se mira antes de atender.
- **"Historial — Notas SOAP"**: las evoluciones SOAP en orden cronológico
  inverso. Si está vacío: *"Se crean desde la pestaña Entries"*.

---

## 9. Exportar la historia clínica

Para entregar, archivar o pedir una segunda opinión.

<!-- CAPTURA: 13-export — Botón "Exportar .md" con el banner de éxito debajo -->

1. Seleccioná el paciente → pestaña **Entries**.
2. **"Exportar .md"**.
3. Aparece un banner: `Exportado: {archivo} (N entries) ✓ firmado · sha256 …`
4. **"Revelar en carpeta"** abre la carpeta con el archivo.

El archivo Markdown incluye: filiación, perfil profesional, resumen por
categoría, todas las entries con su estado de firma, un **SHA-256** al pie y
una **firma digital Ed25519** del documento completo — cualquiera puede
verificar que no fue alterado.

> 📁 Los export se guardan en la carpeta de datos de la app, en una subcarpeta
> `exports/` con nombre y fecha. **El archivo contiene datos clínicos en
> claro** (para ser leído): tratalo como un papel de la historia clínica.

---

## 10. Administración

Rol **administrador** — menú lateral.

### 10.1 Usuarios

<!-- CAPTURA: 14-usuarios — Panel "Gestión de Usuarios" con la tabla y el formulario de nuevo usuario -->

**"Gestión de Usuarios"** — *alta, baja lógica y recuperación de
contraseñas; el historial nunca se borra.*

- **"+ Nuevo usuario"** → usuario, contraseña (mínimo 8) y **Rol**:
  - **Profesional** (médico, enfermero, kinesiólogo… — crea entries)
  - **Asistente** (recepción / altas — solo admisión de pacientes)
  - **Administrador** (gestión técnica)
  - **Paciente** (acceso limitado a su cuenta)
  - → **"Crear usuario"**.
- Tabla: Usuario · Rol · Estado (● Activo / ● Desactivado) · Último acceso ·
  Acciones.
- **"Desactivar"**: baja lógica — el usuario deja de entrar pero **nada se
  borra** (no podés desactivarte a vos mismo).
- **"Reset contraseña"**: regenera la bóveda del usuario. Avisa que su
  clave de firma cambia: **las entries anteriores siguen verificándose** y
  las nuevas usan la clave nueva. La contraseña vieja deja de servir.

### 10.2 Auditoría

**"📋 Auditoría"** — registro de acciones sensibles (últimos 200):
Fecha · Acción · Ejecutada por · Sobre · Detalle. Acciones típicas: *Alta
de usuario*, *Baja de usuario*, *Contraseña reseteada*, *Frase semilla
rotada*, *respaldos*.

> ℹ️ La auditoría guarda **solo nombres de acciones y campos — nunca datos
> clínicos ni contraseñas**.

### 10.3 Frase Semilla (rotación)

<!-- CAPTURA: 15-rotar-seed — Panel de rotación con los 2 pasos -->

Para cambiar la frase vieja por una nueva:

1. **"Generar frase nueva"**.
2. 📝 **1. Anotá esta frase en papel** (¡todavía no está aplicada!).
3. **2. Re-escribí la frase** para confirmar → **"Confirmar rotación"**.
4. **"Frase semilla rotada. La frase anterior quedó invalidada."**

> ⚠️ **La rotación recién se aplica al confirmar**: hasta ese paso la frase
> vieja sigue sirviendo. Si no anotás la nueva, quedás sin recuperación.

### 10.4 Respaldos

<!-- CAPTURA: 16-respaldo — Panel de Respaldos con el resumen del manifest y los botones -->

**Generar respaldo**: **"Generar respaldo…"** → elegí una carpeta (disco
externo o nube) → se crea `respaldo-fecha` con la base, vaults y claves, más
un `manifest.json` con huella SHA-256 de cada archivo. Queda en la carpeta
que elegiste — **guardalo junto a tu frase semilla**.

**Restaurar respaldo**: **"Elegir carpeta de respaldo…"** → resumen (fecha,
versión, `integridad ✓ · firma ✓`) → **"Restaurar y cerrar sesión"**. La app
cierra la sesión y te deja en la pantalla de entrada: entrá con una
**contraseña del respaldo** (o la frase semilla).

> ⚠️ Restaurar **reemplaza todos los datos actuales** por los del respaldo.
> Usalo para recuperarte de un problema, no para "deshacer" cambios.

---

## 11. Acceso y recuperación

### Entrar

**"🔑 Iniciar Sesión Core"** → Usuario + Contraseña → **"Ingresar al
Sistema"**. Si la instalación pide la **frase semilla** (primer ingreso de
ese usuario o si se perdió el wrap), tipeala → **"Desbloquear con Seed"**.

### Si olvidaste tu contraseña

- **Si sos un usuario normal**: pedile al **administrador** que te haga
  **"Reset contraseña"** desde Usuarios ([§10.1](#101-usuarios)).
- **Si sos el administrador y olvidaste TODO**: en la pantalla de login,
  **"¿Olvidaste la contraseña? → Recuperar con frase semilla"** → frase de
  6 palabras + nueva contraseña → **"Restablecer y entrar"**.
  - ⚠️ Resetea la contraseña del administrador y **queda registrado en la
    auditoría**. Requiere la frase del papel.

### Rol Paciente

Quien entra con rol `paciente` ve **solo "👤 Mi Cuenta"**: su usuario, rol y
cambio de contraseña. **No ve entries ni fichas** — ni siquiera las suyas
(en esta beta).

<!-- CAPTURA: 17-paciente-vista — Vista del rol paciente ("Mi Cuenta") -->

---

## 12. Reportar problemas

- **Issues**: https://github.com/enrique-carbo/historia-clinica-tauri/issues
  (o directo al equipo del beta).
- Incluí: qué hiciste, qué esperabas, qué pasó y tu sistema operativo.
- 🔒 **El repo es público**: **nunca subas capturas ni datos de pacientes
  reales**. Para probar, usá datos de prueba mientras avanza la beta.
- ¿Falta algo en esta guía? Contanos y la agregamos.

---

## 13. Diferencias de Fichio Vet

Fichio Vet es el **mismo producto** con:

- Otro nombre: el header dice **Fichio Vet**, y tiene base de datos
  propia (podés tener Care y Vet en la misma PC).
- **Campos distintos en la ficha del paciente**: en Vet no hay DNI,
  Seguro Salud ni Afiliado, y **el teléfono y el correo van dentro del
  grupo del tutor** (no tienen campos propios):

  | Campo Vet | Detalle |
  |---|---|
  | Apellido / Tutor | apellido del animal o del tutor |
  | Especie | Perro, Gato, Ave, Roedor, Conejo, Reptil, Otro |
  | Raza · Sexo | — |
  | **N° Registro / Microchip** | el "documento" del paciente |
  | Responsable - Tutor | grupo con **nombre, teléfono y correo** del tutor |

- La búsqueda de documentos en Vet usa el label del perfil:
  **"N° Registro / Microchip exacto..."**.
- Todo lo demás — entries, SOAP, alergias, medicación, export, respaldos,
  auditoría, roles — es **idéntico**.

---

## 14. Buenas prácticas de seguridad

1. **Respaldá cada semana** (Administración → Respaldos) y guardá el
   respaldo **en otro lugar** que la PC: disco externo o nube.
2. **Frase semilla en papel**, en un lugar seguro (no en la PC ni en el
   celular). Podés guardarla en una caja de seguridad.
3. **Cerrá la bóveda** ("🔒 Cerrar") al terminar la jornada — sobre todo en
   consultorios con recepción compartida.
4. **Contraseñas de mínimo 8 caracteres** y una por usuario (nada de
   compartir).
5. **Antes de actualizar** a una versión nueva: respaldá primero.
6. **Perder contraseña + frase semilla + respaldos = los datos no se pueden
   recuperar, por diseño** (están cifrados con algo que nadie más tiene).
   No es un bug: es la misma garantía que los protege.
7. **Sin antivirus/archivos que bloqueen**: si Windows se queja al instalar,
   es la falta de certificado comercial — ver [§2](#2-instalación).

---

## 15. Glosario

| Término | Qué significa |
|---|---|
| **Bóveda (vault)** | Tu llave privada de firma, cifrada con tu contraseña. "Cerrar bóveda" = borrar las llaves de la memoria. |
| **DataKey** | La llave que cifra todos los datos clínicos de la instalación. Vive solo en la memoria mientras hay sesión. |
| **Entry** | Un registro clínico inmutable (SOAP, alergia, medicación o condición), cifrado y firmado. |
| **Firma digital** | Sello criptográfico que demuestra quién creó la entry y que no fue alterada. Se muestra como *Verificada* / *Sin verificar*. |
| **Frase semilla** | Las 6 palabras en papel que permiten recuperar el acceso si se pierden todas las contraseñas. |
| **Blind index** | Índice que permite buscar por el documento **exacto** (DNI / Microchip) sin que esté guardado en claro. |
| **Respaldo** | Copia cifrada de la base + claves con manifiesto de integridad (SHA-256). |
| **Admisión** | Pantalla de alta / edición / selección de pacientes. |

---

*Guía de Fichio v0.1.0 — beta. Si encontrás una discrepancia entre esta
guía y la app, reportala: la app manda, esta guía se corrige.*
