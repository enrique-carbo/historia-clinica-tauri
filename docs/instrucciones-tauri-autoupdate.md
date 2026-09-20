# Guía de Compilación Multiplataforma y Auto-Updates con Tauri v2, React y GitHub Actions

Esta guía detalla los pasos necesarios para configurar la compilación automatizada multiplataforma (Windows, macOS y Linux) de una aplicación **Tauri v2** con frontend en **React**, integrando además el sistema de actualizaciones automáticas (**Auto-Updates**) utilizando **GitHub Actions**.

---

## 🛠️ Requisitos Previos

1. Tener un proyecto funcional de **Tauri v2** con **React**.
2. Un repositorio en **GitHub** para alojar el código del proyecto.
3. Node.js (versión LTS reciente) y Rust instalados localmente.

---

## 🔑 Paso 1: Generar las Llaves de Firma para el Updater

Tauri v2 requiere de forma obligatoria que todos los paquetes de actualización estén firmados digitalmente para garantizar la seguridad del usuario final.

1. Abre la terminal en la raíz de tu proyecto local y ejecuta:
   ```bash
   npx tauri signer generate
   ```
2. El comando imprimirá dos cadenas de texto en la consola:
   - **Llave pública (`pubkey`)**: Se añade directamente al archivo de configuración de Tauri.
   - **Llave privada**: Es secreta y se utiliza para firmar los paquetes en el entorno de integración continua (CI).

---

## 🔐 Paso 2: Configurar los Secretos en GitHub

Para que GitHub Actions pueda firmar tus compilaciones sin exponer tus credenciales, debes registrar la llave privada como un secreto del repositorio.

1. Ve a tu repositorio en GitHub y navega a **Settings** -> **Secrets and variables** -> **Actions**.
2. Haz clic en **New repository secret** y añade los siguientes valores:
   - **Nombre:** `TAURI_SIGNING_PRIVATE_KEY`
     - **Valor:** Pega la llave privada generada en el Paso 1 (incluyendo las líneas de inicio y fin si las tiene).
   - **Nombre:** `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
     - **Valor:** Si le asignaste una contraseña a tu llave privada, colócala aquí. Si la dejaste vacía, crea este secreto con cualquier valor ficticio o un espacio en blanco (algunas configuraciones de CI requieren que la variable exista).

---

## ⚙️ Paso 3: Configurar `tauri.conf.json`

Debes activar el plugin del actualizador dentro de la configuración de Tauri (`src-tauri/tauri.conf.json`) y apuntar al endpoint estático donde GitHub alojará los archivos de lanzamiento.

Modifica o añade la propiedad `plugins` y asegúrate de incluir tu llave pública:

```json
{
  "bundle": {
    "active": true
  },
  "plugins": {
    "updater": {
      "active": true,
      "pubkey": "AQUÍ_VA_TU_LLAVE_PÚBLICA_GENERADA_EN_EL_PASO_1",
      "endpoints": [
        "https://github.com/TU_USUARIO_GITHUB/TU_REPOSITORIO/releases/latest/download/latest.json"
      ]
    }
  }
}
```

> ⚠️ **Nota:** Reemplaza `TU_USUARIO_GITHUB` y `TU_REPOSITORIO` con los datos reales de tu proyecto. Asegúrate de mantener la estructura exacta del enlace hacia `latest.json`.

---

## 🚀 Paso 4: Crear el Workflow de GitHub Actions

Crea la estructura de carpetas `.github/workflows/` en la raíz de tu proyecto si no existe, y genera un archivo llamado `release.yml`.

### Archivo: `.github/workflows/release.yml`

```yaml
name: 'Publish Release'

on:
  push:
    tags:
      - 'v*' # Se activa automáticamente al empujar tags como v1.0.0, v1.0.1, etc.

jobs:
  publish-tauri:
    permissions:
      contents: write
    strategy:
      fail-fast: false
      matrix:
        include:
          - platform: 'macos-latest'
          - platform: 'ubuntu-22.04'
          - platform: 'windows-latest'

    runs-on: ${{ matrix.platform }}
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: lts/*

      - name: Install Rust base
        uses: dtolnay/rust-toolchain@stable

      - name: Rust cache
        uses: swatinem/rust-cache@v2
        with:
          workspaces: './src-tauri -> target'

      - name: Install system dependencies (Ubuntu Only)
        if: matrix.platform == 'ubuntu-22.04'
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev

      - name: Install Frontend Dependencies
        run: npm install # Cambia por yarn install o pnpm install si corresponde

      - name: Build and Publish
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: 'App ${{ github.ref_name }}'
          releaseBody: 'Actualización automática generada por GitHub Actions.'
          uploadUpdaterJson: true # Obligatorio para generar y subir el archivo latest.json de actualización
          releaseDraft: false
```

---

## 🔄 Paso 5: Flujo de Trabajo para Publicar Actualizaciones

Cada vez que realices cambios en el código de React o Rust y quieras enviar una actualización automática a tus usuarios instalados, sigue estrictamente estos pasos:

1. **Sincronizar versiones:** Modifica la versión de la app en los siguientes dos archivos para que coincidan exactamente:
   - `package.json` (ej. `"version": "1.0.1"`)
   - `src-tauri/tauri.conf.json` (ej. `"version": "1.0.1"`)
2. **Hacer Commit y crear el Tag de Git:**
   Abre tu terminal y ejecuta los comandos para guardar los cambios y asignarle la etiqueta de versión (Tag):
   ```bash
   git add .
   git commit -m "feat: descripción de los nuevos cambios y mejoras"
   git tag v1.0.1
   git push origin main --tags
   ```
3. **Procesamiento automático:** 
   GitHub Actions detectará el nuevo tag `v1.0.1`, ejecutará los entornos virtuales correspondientes, compilará los instaladores firmados y reescribirá el archivo `latest.json` en los Releases de tu repositorio. 

Cualquier instancia cliente de tu aplicación que implemente la escucha del Updater detectará la nueva firma en `latest.json` e iniciará la actualización automática en el equipo del usuario.