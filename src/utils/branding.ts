// src/utils/branding.ts
//
// Nombre visible del producto según el perfil de build
// (docs/PLUGIN_ARCHITECTURE.md "Perfiles de producto").
// `VITE_FICHIO_PROFILE=vet` (vite --mode vet / .env.vet); default med.
// El backend usa el mismo dato vía `env!("FICHIO_APP_NAME")` (build.rs).

export const APP_NAME: string =
  import.meta.env.VITE_FICHIO_PROFILE === "vet" ? "Fichio Vet" : "Fichio Care";
