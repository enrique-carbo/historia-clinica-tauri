fn main() {
    tauri_build::build();

    // Perfiles de producto Med/Vet (docs/PLUGIN_ARCHITECTURE.md).
    // FICHIO_PROFILE=med|vet (default med). Copia el entity_schema del perfil
    // a OUT_DIR (lo embebe config_schema.rs) y exporta FICHIO_APP_NAME para
    // el branding del export. Selección = datos, sin #[cfg] en el Core.
    let profile =
        std::env::var("FICHIO_PROFILE").unwrap_or_else(|_| "med".to_string());
    println!("cargo:rerun-if-env-changed=FICHIO_PROFILE");

    let manifest_dir =
        std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let profile_dir = manifest_dir.join("../profiles").join(&profile);

    let schema_src = profile_dir.join("entity_schema.json");
    println!("cargo:rerun-if-changed={}", schema_src.display());
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let schema_dst = std::path::Path::new(&out_dir).join("entity_schema.json");
    std::fs::copy(&schema_src, &schema_dst).unwrap_or_else(|e| {
        panic!(
            "No se pudo copiar el schema del perfil '{}' ({}): {}",
            profile,
            schema_src.display(),
            e
        )
    });

    let branding_src = profile_dir.join("branding.json");
    println!("cargo:rerun-if-changed={}", branding_src.display());
    let branding: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&branding_src)
            .unwrap_or_else(|e| panic!("No se pudo leer {}: {}", branding_src.display(), e)),
    )
    .unwrap_or_else(|e| panic!("branding.json del perfil '{}' inválido: {}", profile, e));
    let app_name = branding["app_name"]
        .as_str()
        .unwrap_or_else(|| panic!("branding.json del perfil '{}' sin app_name", profile));
    println!("cargo:rustc-env=FICHIO_APP_NAME={}", app_name);
}
