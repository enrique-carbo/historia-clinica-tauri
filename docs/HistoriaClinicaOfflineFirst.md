# Historia Clínica Digital Offline First 🩺

Una plataforma de historia clínica, monitoreo clínico y teleconsulta asíncrona estructurada bajo el paradigma **Offline-First**. Diseñada para ofrecer rendimiento extremo (latencia cero), autonomía total de red para el personal médico, soberanía absoluta de los datos mediante una infraestructura autohospedada (*self-hosted*) y control de privacidad total en manos del paciente.

---

## 🚀 Características Principales

* **Autonomía Offline-First:** La aplicación no es un visor web; es un cliente nativo. Funciona al 100% sin conexión a internet. Los médicos pueden consultar historiales y redactar diagnósticos en entornos sin señal; los datos se sincronizan automáticamente al detectar red.
* **Teleconsulta Asíncrona:** Sistema eficiente de gestión de casos estilo bandeja de entrada. El paciente rellena un formulario estructurado de síntomas y el médico responde de manera diferida, optimizando los tiempos de consulta y eliminando la fricción del tiempo real.
* **Bitácora de Variables Clínicas (PGHD):** Zona dedicada para que el paciente registre métricas diarias (presión arterial, glucemia, oximetría, niveles de dolor o prurito). Los datos generan gráficos de tendencia instantáneos en el cliente de forma 100% privada y local.
* **Sincronización Selectiva bajo Demanda:** El paciente es dueño absoluto de sus métricas diarias. La aplicación permite mantener los registros de salud en el ámbito privado local o empaquetarlos y compartirlos con el profesional médico en un rango de fechas específico al iniciar una teleconsulta.
* **Privacidad por Diseño (Cifrado Híbrido a Nivel de Aplicación):** Los datos narrativos sensibles (síntomas, diagnósticos, notas de la bitácora) se cifran en el cliente (Rust) antes de persistirse localmente o subirse a la nube. El servidor nunca posee visibilidad de la información clínica en texto plano.
* **Rendimiento Nativo Extremo:** Consultas y agregaciones analíticas de métricas locales en menos de 1ms gracias al motor de base de datos relacional embebido y gestionado nativamente por el core de Rust. Interfaz de usuario fluida a 60 FPS con React y Tailwind CSS.
* **Infraestructura Ultra-Económica (Self-Hosted):** El backend se ejecuta completamente en un servidor privado virtual (VPS) propio mediante contenedores optimizados, distribuyendo el procesamiento al cliente y reduciendo drásticamente los costos fijos.

---

## 🛠️ Stack Tecnológico

### Cliente (Desktop Nativo)
* **Tauri v2:** Contenedor nativo multiplataforma (ligero, seguro y con mínimo consumo de RAM).
* **Rust (Core Backend Local):** Manejo de la lógica criptográfica (AES-GCM-256), procesamiento de imágenes (compresión nativa a WebP), operaciones de base de datos y motor de sincronización asíncrono en hilos de fondo (`tokio`).
* **SQLite (rusqlite):** Base de datos relacional local embebida de alta velocidad.
* **React + TypeScript + Tailwind CSS:** Interfaz de usuario declarativa y minimalista. Gráficos interactivos de alto rendimiento mediante componentes ligeros en el cliente.

### Servidor (Nube & Sincronización)
* **Dokploy:** Plataforma de orquestación autohospedada en VPS para gestionar el despliegue mediante Docker.
* **Traefik:** Proxy inverso (incluido en Dokploy) para la gestión de enrutamiento y certificados SSL (HTTPS vía Let's Encrypt).
* **PocketBase:** Backend en un solo binario (Go + SQLite interno) que resuelve de forma nativa la Autenticación, Roles, API REST y Almacenamiento de archivos adjuntos.

---

## 🏗️ Arquitectura del Sistema

La aplicación separa estrictamente la capa de presentación de la capa de datos y sincronización:

```text
+------------------------------------------------------------+
| CLIENTE (Tauri App - Escritorio)                           |
|                                                            |
|  [ React UI (Tailwind) ]  <---> [ Rust Core (Tokio Async) ]|
|            |                             |                 |
|     (Llamadas IPC)             (Cifrado AES-GCM-256)       |
|            |                             v                 |
|            +--------------------> [ SQLite Local (.db) ]   |
|                                          |                 |
|                                    (On-Demand Push)        |
+------------------------------------------|-----------------+
                                           |
                                     (HTTPS / JSON)
                                           v
+------------------------------------------------------------+
| SERVIDOR (VPS con Dokploy)                                 |
|                                                            |
|    [ Traefik ] (Maneja el SSL/HTTPS automáticamente)        |
|        v                                                   |
|    [ PocketBase Container ]                                |
|        |---> Auth & Roles (Médico / Paciente)              |
|        |---> SQLite Centralizado                           |
|        +---> Storage (Adjuntos / Imágenes WebP optimizadas) |
+------------------------------------------------------------+

```

---

## 🛡️ Arquitectura de Datos y Seguridad (Offline-First & Zero-Knowledge)

El proyecto está diseñado bajo el paradigma **Offline-First (Local-First)** y seguridad **Zero-Knowledge (Cero Conocimiento)**. El servidor central nunca tiene acceso a las llaves de cifrado ni a los datos médicos en texto plano.

### 🔑 Esquema de Cifrado y Recuperación

1. **Clave Maestra (Master Key):** Al registrarse, el cliente (Tauri/Rust) genera una clave aleatoria de 256 bits (AES-GCM) que cifra la base de datos SQLite local. Esta clave nunca cambia.
2. **Master Key Wrap:** La contraseña del usuario deriva en una *Key Encrypting Key* (KEK) mediante **Argon2**. La KEK cifra la Clave Maestra antes de sincronizarse de forma segura. Esto permite cambiar la contraseña del usuario en <50ms en la RAM local sin necesidad de re-cifrar el histórico de datos clínicos.
3. **Mnemónico de 12 Palabras (BIP-39):** En el onboarding se generan 12 palabras aleatorias (128 bits de entropía). Es el único mecanismo de recuperación en caso de olvido de contraseña. El servidor no posee mecanismos de "Reset Password" tradicionales sobre los datos cifrados.

### ⚙️ Cifrado Selectivo Local e Indexación

Al registrar un diagnóstico o una nota personal en la bitácora, el núcleo de Rust utiliza **AES-GCM-256** para cifrar las cadenas de texto narrativas. Las variables numéricas puras (`sistolica`, `diastolica`, `valor_glucemia`, `nivel_dolor`) e índices temporales se almacenan en texto plano de forma anónima dentro de la base local.

Esto permite que el motor SQLite local calcule promedios, máximos y mínimos de forma instantánea para las gráficas y resuelva consultas como `LIKE '%prurito%'` en menos de 1ms, sin comprometer la identidad ni la seguridad del dato médico en la nube.

### 🔄 Estrategia de Copias de Seguridad (Backups)

El sistema prioriza la soberanía de los datos en el cliente, ofreciendo un enfoque híbrido configurable por el usuario desde la UI (React):

```text
[ SQLite Local ] ──(Backup en Caliente)──> [ .zip Cifrado Cliente ]
                                                    │
             ┌──────────────────────────────────────┴──────────────────────────────────────┐
             ▼                                                                             ▼
   [ Prioridad 1: Local Only ]                                                  [ Prioridad 2: Cloud Support ]
   Guardado en AppData o exportado                                              Subida asíncrona a PocketBase
   a almacenamiento físico (Pendrive/SD).                                       (Almacenamiento frío en R2).

```

1. **Prioridad 1: Respaldo Local (Por Defecto):** El core de Rust utiliza la API nativa de respaldo de SQLite para clonar la base de datos en caliente en momentos de inactividad. El archivo se comprime en un `.zip` sellado con la Clave Maestra. El usuario puede exportarlo físicamente mediante la API de diálogos nativos de Tauri a un almacenamiento externo (Pendrive/SD).
2. **Prioridad 2: Soporte en la Nube (Opt-in):** Si el médico activa la opción de forma explícita, el hilo de fondo de Rust (`tokio`) despacha el `.zip` cifrado a una colección dedicada en PocketBase. El servidor actúa únicamente como un casillero postal de archivos binarios planos resguardados en un Object Storage (Cloudflare R2).

### ❄️ Mecanismos de Recuperación ante Desastres (Cold Start)

Cuando un usuario inicia sesión con sus 12 palabras en un dispositivo totalmente limpio, la aplicación resuelve la reconstrucción del entorno local en base al siguiente orden de contingencia:

| Prioridad | Mecanismo | Flujo Técnico | Impacto en Servidor | Experiencia de Usuario (UX) |
| --- | --- | --- | --- | --- |
| **1 (Alta)** | **Importación Local** | El usuario monta el `.zip` físico desde un pendrive o directorio. Rust levanta el SQLite e inicia un Pull delta (marcas de tiempo). | **Cero** (Tráfico mínimo por API para novedades). | **Inmediata.** El consultorio se restaura en segundos de forma 100% offline. |
| **2 (Media)** | **Descarga Cloud** | La app solicita el último `.zip` alojado en el casillero de PocketBase/R2, lo despaqueta localmente y sincroniza el delta final. | **Bajo** (Descarga de archivo plano optimizado). | **Rápida.** Pantalla de carga limpia mientras se descarga el bloque unificado. |
| **3 (Baja)** | **Lazy Sync / API** | Plan de contingencia si no existen backups. Rust descarga el historial fragmentado por lotes directos de las colecciones a través de la API JSON. | **Alto** (Múltiples queries y payloads JSON pesados). | **Incremental.** Acceso inmediato a la bandeja activa; el historial se puebla en segundo plano. |

### 🛠️ Flujo de Sincronización Celular (Conflictos)

Para evitar colisiones de datos en modificaciones concurrentes entre el cliente y PocketBase, se implementa un modelo de **parcheo asíncrono inmutable**:

* El paciente modifica únicamente campos mutables de su rol (`sintomas_cifrados`).
* El médico interactúa con campos de su competencia (`diagnostico_cifrado`).
* Rust realiza la sincronización utilizando payloads segmentados (Deltas) basados en marcas de tiempo con precisión de milisegundos (`updated_at`), aplicando una estrategia de *Last-Write-Wins* (LWW) por campo individual y no por registro o fila completa.
* **Optimización de Medios:** Antes de sincronizar imágenes adjuntas, Rust las comprime y convierte localmente al formato **WebP**, reduciendo el tamaño del payload en un ~90% para ahorrar ancho de banda y almacenamiento en el VPS.

---

## 📊 Modelo de Datos (Esquema Conceptual de Persistencia)

### 1. Colección / Tabla: `users`
* `id` (TEXT, PK) -> UUID generado en el cliente.
* `email` (TEXT, Único) -> Identificador de cuenta.
* `role` (TEXT: `'medico'` | `'paciente'`) -> Control de acceso y reglas de la API.

### 2. Colección / Tabla: `asynchronous_consultations` (Teleconsulta)
* `id` (TEXT, PK)
* `paciente_id` (TEXT, FK -> users)
* `medico_id` (TEXT, FK -> users, Nullable)
* `motivo_consulta` (TEXT - Plano) -> Breve descripción para indexación rápida de bandejas.
* `sintomas_cifrados` (TEXT - AES-GCM Blob) -> Narrativa inicial del paciente.
* `diagnostico_cifrados` (TEXT - AES-GCM Blob, Nullable) -> Devolución diferida del profesional.
* `status` (TEXT: `'PENDIENTE'`, `'EN_REVISION'`, `'RESPONDIDA'`, `'ARCHIVADA'`)
* `archivos_adjuntos` (JSON) -> Lista de URLs de imágenes WebP optimizadas en el almacenamiento.
* `created_at` / `updated_at` (TIMESTAMP)

### 3. Colección / Tabla: `in_person_consultations` (Consulta Presencial - Estándar SOAP)
* `id` (TEXT, PK)
* `paciente_id` (TEXT, FK -> users)
* `medico_id` (TEXT, FK -> users)
* `s_subjetivo_cifrado` (TEXT - AES-GCM Blob) -> Síntomas, relato e historia clínica contada por el paciente.
* `o_objetivo_cifrado` (TEXT - AES-GCM Blob) -> Hallazgos de exploración física y texto de estudios complementarios.
* `a_analisis_cifrado` (TEXT - AES-GCM Blob) -> Juicio clínico, hipótesis y diagnósticos presuntivos o definitivos.
* `p_plan_cifrado` (TEXT - AES-GCM Blob) -> Tratamiento farmacológico, indicaciones y pautas de alarma.
* `created_at` / `updated_at` (TIMESTAMP)

### 4. Colección / Tabla: `patient_metrics` (Métricas e Índices Clínicos - Genérica EAV)
* `id` (TEXT, PK)
* `paciente_id` (TEXT, FK -> users)
* `metric_type` (TEXT) -> Grupo macro (Ej: `'PRESION'`, `'GLUCEMIA'`, `'ANTROPOMETRIA'`, `'SUEÑO'`).
* `sub_metric` (TEXT) -> Variable específica (Ej: `'SISTOLICA'`, `'DIASTOLICA'`, `'VALOR'`, `'CINTURA'`).
* `value_num` (NUMERIC) -> Valor cuantitativo plano para analíticas y gráficas locales en tiempo real (<1ms).
* `value_text_cifrado` (TEXT, Nullable) -> AES-GCM Blob para notas o aclaraciones contextuales de la toma.
* `measured_at` (TIMESTAMP)
* `is_shared` (BOOLEAN) -> 0 = Local exclusivo del paciente; 1 = Sincronizado y compartido con el médico.

---

## 🔁 Formato de Exportación Intermedia (Paciente Index JSON)

Para interactuar con herramientas de Inteligencia Artificial y asegurar la transferencia de datos con ecosistemas de salud externos, la aplicación genera un esquema unificado de texto plano en memoria RAM (tras el descifrado seguro en el cliente). Este archivo actúa como una **"Single Source of Truth" (Única Fuente de Verdad)** efímera para la IA traductora:

```json
{
  "export_metadata": {
    "timestamp": "2026-06-07T14:00:00Z",
    "schema_version": "1.0.0"
  },
  "patient_context": {
    "internal_id": "usr_9481a7b8"
  },
  "clinical_history": {
    "soap_records": [
      {
        "date": "2026-05-20T10:30:00Z",
        "provider_id": "med_3310",
        "soap_data": {
          "subjective": "Paciente manifiesta persistencia de dolor epigástrico...",
          "objective": "Abdomen blando, doloroso a la palpación en epigastrio. TA: 120/80.",
          "assessment": "Dispepsia no ulcerosa / Sospecha de reflujo gastroesofágico.",
          "plan": "Omeprazol 20mg en ayunas por 28 días. Dieta anti-reflujo."
        }
      }
    ],
    "asynchronous_threads": [
      {
        "opened_at": "2026-05-22T21:15:00Z",
        "reason": "Seguimiento de tratamiento por dispepsia",
        "patient_narrative": "A los dos días de tomar el protector disminuyó la acidez, pero hoy volvió levemente.",
        "doctor_response": "Es normal en la primera semana. Mantener la dosis. Si aparece dolor nocturno, reportar."
      }
    ]
  },
  "biometric_logs": [
    {
      "timestamp": "2026-05-20T08:00:00Z",
      "category": "PRESION",
      "variables": { "SISTOLICA": 120, "DIASTOLICA": 80 }
    }
  ]
}
```

---

## 🗺️ Plan de Ruta (Roadmap)

### Fase 1: Entorno de Producción y Servidor (Dokploy)

* [ ] Instalar Dokploy en el VPS de producción.
* [ ] Desplegar la plantilla oficial de PocketBase en Dokploy.
* [ ] Configurar colecciones, índices y API Rules (Reglas de acceso por rol) en PocketBase.

### Fase 2: Esqueleto de la Aplicación de Escritorio & UI

* [ ] Inicializar proyecto Tauri v2 con React + TypeScript + Tailwind CSS.
* [ ] Diseñar la interfaz de la Bandeja de Entrada (Vista Médico), Formulario de Síntomas (Vista Paciente).
* [ ] Diseñar la interfaz de Bitácora Diaria del Paciente con gráficos de líneas de tendencia y el Tablero de Evolución en la vista del Médico.

### Fase 3: Core de Rust, Base de Datos Local y Cifrado

* [ ] Integrar `rusqlite` y estructurar las tablas locales (`asynchronous_consultations` y `patient_metrics`).
* [ ] Implementar los *Tauri Commands* de lectura/escritura híbrida (cifrado selectivo nativo en Rust con AES-GCM).
* [ ] Agregar el pipeline de procesamiento de imágenes con el crate `image` (Conversión a WebP).

### Fase 4: Sincronización Selectiva e Integración del Sistema

* [ ] Programar el despachador de sincronización selectiva por lotes en Rust y el gestor de backups locales (`.zip`).
* [ ] Configurar reglas de visibilidad en React basadas en el estado del flag `is_shared` e implementar los flujos de *Cold Start*.
* [ ] Integrar notificaciones nativas del sistema operativo vía Tauri al recibir respuestas médicas.
* [ ] **[NUEVO] Subsistema de Interoperabilidad Inteligente (AI-Driven HL7/FHIR):** Programar un motor de exportación en Rust que consolide el historial del paciente en un "Paciente Index" en formato JSON estructurado. Configurar el puente para que un Agente de IA local o API procese este archivo y lo traduzca dinámicamente a estándares clínicos internacionales (HL7 v2/v3 o recursos FHIR).

---

## 🔒 Consideraciones de Seguridad Legal (Compliance)

* **Cero Conocimiento (Zero-Knowledge) Parcial:** Dado que los campos médicos se cifran con una clave de cifrado local simétrica controlada por el usuario antes de salir de la aplicación, el administrador del servidor VPS (o posibles atacantes de la red) no pueden leer el historial clínico de los pacientes ni las notas de su bitácora.
* **Soberanía de Datos Explícita:** La separación estricta entre métricas locales (`is_shared = 0`) y compartidas junto con la gestión autónoma de los backups le otorga al usuario el control legal y práctico sobre qué información decide exponer a la red, cumpliendo con regulaciones estrictas de privacidad de datos de salud (*Privacy by Design*).
