# 🔐 Estrategia de Cifrado y Privacidad — Historia Clínica Electrónica

Este documento detalla la arquitectura de seguridad, los algoritmos criptográficos y el flujo de datos del Core. El sistema está diseñado bajo el principio de **Zero-Knowledge (Conocimiento Cero)** y **Local-First**, garantizando que los datos de salud e identificación de los pacientes nunca sean expuestos en texto plano fuera del dispositivo local del profesional autorizado.

---

## 🏗️ 1. Pilares Criptográficos

El Core de Rust utiliza librerías de nivel de producción (`aes-gcm`, `argon2`, `sha2`) para ejecutar tres tipos de operaciones criptográficas complementarias en la memoria RAM:

| Operación | Algoritmo | Propósito | Implementación |
| :--- | :--- | :--- | :--- |
| **Cifrado Simétrico** | `AES-GCM-256` | Blindar textos clínicos (SOAP) y datos filiatorios (Nombres). | Cifrado Autenticado con datos asociados (AEAD). |
| **Derivación de Claves (KDF)** | `Argon2id` | Hasheo de contraseñas locales y control de roles. | Ganador de la Password Hashing Competition (PHC). |
| **Índice Ciego (Blind Index)** | `SHA-256 + Salt` | Búsqueda y prevención de registros duplicados de forma anónima. | Hashing determinista e irreversible. |

---

## 🔒 2. Ciclo de Vida de los Datos Clínicos (SOAP)

Cada vez que un médico redacta una evolución en el formulario, el Core de Rust fragmenta el registro para evitar ataques de análisis de frecuencia o patrones.

```text
[ React Frontend ] (Texto Plano en RAM)
       │
       ▼ (IPC - Tauri Bridge)
[ Rust Core Engine ] 
       │──► 1. Genera un Nonce aleatorio de 12 bytes por cada campo (S, O, A, P).
       │──► 2. Inyecta la Clave Maestra de 32 bytes (RAM-only).
       │──► 3. Ejecuta AES-GCM-256 Cipher.
       ▼
[ SQLite Local / PocketBase ] (Cifrado Hexadecimal + Nonce en Disco)

```

### Reglas de Implementación en Disco (Esquema SQLite)

Para garantizar la inmutabilidad de los datos, las columnas se almacenan emparejadas con su respectivo vector de inicialización (`nonce`):

* `subjetivo_ciphertext` + `subjetivo_nonce`
* `objetivo_ciphertext` + `objetivo_nonce`
* `analisis_ciphertext` + `analisis_nonce`
* `plan_ciphertext` + `plan_nonce`

> ⚠️ **Nota de Seguridad:** Dos campos con el mismo texto (por ejemplo, dos análisis que digan *"Paciente estable"*) generarán cadenas de bytes completamente diferentes en la base de datos debido a que los `nonces` se generan mediante un generador de números aleatorios criptográficamente seguro (`rand::rngs::OsRng`) en cada guardado.

---

## 🚫 3. Estrategia de No-Duplicación: Índices Ciegos (Blind Indexing)

En un ecosistema web tradicional, el servidor evita pacientes duplicados aplicando un índice `UNIQUE` a la columna de identidad (DNI o Pasaporte). En el desarrollo actual el DNI nunca llega al servidor ni se guarda en texto plano en el disco.

Para resolver esto sin romper el modelo Zero-Knowledge, el Core implementa un **Índice Ciego Determinista**:

1. El usuario ingresa el documento de identidad en la interfaz.
2. El Core de Rust normaliza el texto (remueve espacios, guiones y lo transforma a minúsculas).
3. Se concatena con una clave secreta del sistema (`BLIND_INDEX_SALT`).
4. Se procesa a través de **SHA-256** y se convierte a Hexadecimal.

$$\text{Blind Index} = \text{SHA-256}(\text{DNI Normalizado} + \text{BLIND\_INDEX\_SALT})$$

```sql
-- El hash resultante se guarda en la columna 'identity_blind_index' con restricción UNIQUE.
-- Si se intenta ingresar el mismo DNI, SQLite o PocketBase rebotarán la query por duplicación,
-- sin que el motor de base de datos sepa jamás qué DNI originó ese hash.

```

---

## 🔑 4. Gestión de Claves en Memoria

La seguridad del sistema descansa sobre el manejo estricto de las llaves en la memoria del hardware:

* **Persistencia Cero:** Ninguna clave de descifrado simétrico se escribe en archivos de configuración, variables de entorno del sistema operativo o almacenamiento local del navegador (`localStorage`).
* **Aislamiento de Procesos:** El frontend (React/Vite) nunca tiene acceso a las llaves criptográficas ni al texto cifrado crudo; solo recibe los datos ya descifrados desde el proceso nativo de Rust a través del canal seguro de IPC de Tauri.
* **Evolución del Módulo (Roadmap):** La actual `MOCK_MASTER_KEY` será reemplazada en la Fase 3 por una clave derivada dinámicamente en RAM al momento del inicio de sesión, utilizando la contraseña maestra del profesional de la salud mediante `Argon2id`.

---

## ☁️ 5. Flujo de Sincronización Segura (Híbrido)

Cuando se implemente el puente de sincronización con PocketBase, el servidor remoto actuará como un mero casillero de almacenamiento ciego:

1. **Validación de Canales:** Los datos viajarán sobre TLS (HTTPS), pero aunque el canal TLS fuese vulnerado (Man-in-the-Middle), los payloads ya viajan encriptados desde el origen.
2. **Estructura del Payload Remoto:** El servidor en la nube solo almacenará:
* UUIDs relacionales generados aleatoriamente.
* Bloques de texto cifrado incomprensibles.
* Nonces públicos.
* El `identity_blind_index` para indexación y búsquedas opacas.


3. **Filosofía:** El proveedor de la nube (o cualquier atacante que acceda al servidor central) solo verá metadatos correlativos, haciendo imposible la reconstrucción de la historia clínica de un paciente o su identificación legal.
