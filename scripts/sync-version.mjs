// scripts/sync-version.mjs
//
// Fuente única de versión: src-tauri/tauri.conf.json (la usa Tauri para el
// bundle y el updater futuro). Este script la sincroniza a package.json y
// src-tauri/Cargo.toml (hoy la versión vive en 3 archivos).
//
//   node scripts/sync-version.mjs          # sincroniza desde tauri.conf.json
//   node scripts/sync-version.mjs --check  # falla si las 3 no coinciden (CI)

import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const check = process.argv.includes("--check");

const confPath = join(root, "src-tauri/tauri.conf.json");
const pkgPath = join(root, "package.json");
const cargoPath = join(root, "src-tauri/Cargo.toml");

const conf = JSON.parse(readFileSync(confPath, "utf8"));
const version = conf.version;
if (typeof version !== "string" || !/^\d+\.\d+\.\d+/.test(version)) {
  console.error(`version inválida en tauri.conf.json: ${version}`);
  process.exit(1);
}

const pkg = JSON.parse(readFileSync(pkgPath, "utf8"));
const cargo = readFileSync(cargoPath, "utf8");
const cargoVersion = cargo.match(/^version = "([^"]+)"/m)?.[1];

const problems = [];
if (pkg.version !== version) problems.push(`package.json: ${pkg.version}`);
if (cargoVersion !== version) problems.push(`Cargo.toml: ${cargoVersion}`);

if (problems.length > 0) {
  if (check) {
    console.error(`versiones desincronizadas (fuente tauri.conf.json=${version}):`);
    for (const p of problems) console.error(`  - ${p}`);
    process.exit(1);
  }
  pkg.version = version;
  writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + "\n");
  const cargoSynced = cargo.replace(
    /^version = "[^"]+"/m,
    `version = "${version}"`,
  );
  writeFileSync(cargoPath, cargoSynced);
  console.log(`version sincronizada a ${version} en package.json y Cargo.toml`);
} else if (!check) {
  console.log(`ok: las 3 versiones ya valen ${version}`);
}
