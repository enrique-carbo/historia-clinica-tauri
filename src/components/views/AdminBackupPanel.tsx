import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { Button } from "../ui/Button";
import { Alert } from "../ui/Alert";
import { useAuthStore } from "../../stores/useAuthStore";

interface BackupFile {
  name: string;
  sha256: string;
  bytes: number;
}

interface Manifest {
  app_version: string;
  schema_version: number;
  created_at: string;
  files: BackupFile[];
}

interface BackupInfo {
  path: string;
  app_version: string;
  schema_version: number;
  created_at: string;
  file_count: number;
  total_bytes: number;
}

function formatDate(iso: string): string {
  try {
    return new Date(iso).toLocaleString();
  } catch {
    return iso;
  }
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function AdminBackupPanel() {
  const lockVault = useAuthStore((s) => s.lockVault);

  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const [created, setCreated] = useState<BackupInfo | null>(null);
  const [creating, setCreating] = useState(false);

  const [pending, setPending] = useState<{ path: string; manifest: Manifest } | null>(null);
  const [restoring, setRestoring] = useState(false);

  const handleCreate = async () => {
    setError(null);
    setNotice(null);
    setCreating(true);
    try {
      const dest = await open({
        directory: true,
        title: "Elegir dónde guardar el respaldo",
      });
      if (typeof dest !== "string") return;
      const info = await invoke<BackupInfo>("create_backup", { destParent: dest });
      setCreated(info);
    } catch (e) {
      setError(String(e));
    } finally {
      setCreating(false);
    }
  };

  const handlePickRestore = async () => {
    setError(null);
    setNotice(null);
    try {
      const src = await open({
        directory: true,
        title: "Elegir la carpeta del respaldo (respaldo-…)",
      });
      if (typeof src !== "string") return;
      const manifest = await invoke<Manifest>("validate_backup", { srcPath: src });
      setPending({ path: src, manifest });
    } catch (e) {
      setError(String(e));
    }
  };

  const handleConfirmRestore = async () => {
    if (!pending) return;
    setRestoring(true);
    setError(null);
    try {
      await invoke("restore_backup", { srcPath: pending.path });
      // El backend ya cerró sesión, llaves y conexión: volver al login
      lockVault();
    } catch (e) {
      setError(String(e));
      setRestoring(false);
      setPending(null);
    }
  };

  const pendingTotal = pending
    ? pending.manifest.files.reduce((acc, f) => acc + f.bytes, 0)
    : 0;

  return (
    <div className="flex flex-col gap-6 max-w-3xl">
      <div>
        <h2 className="text-lg font-semibold text-zinc-300">Respaldos</h2>
        <p className="text-xs text-zinc-500 mt-1">
          Copia de seguridad de la base de datos, vaults y wraps de claves. Todo el contenido va
          cifrado en reposo.
        </p>
      </div>

      {error && (
        <Alert variant="error" className="my-0">
          <div className="flex justify-between gap-4">
            <span>{error}</span>
            <button onClick={() => setError(null)} className="font-bold" aria-label="Cerrar">
              ✕
            </button>
          </div>
        </Alert>
      )}

      {notice && (
        <Alert variant="success" className="my-0">
          <div className="flex justify-between gap-4">
            <span>{notice}</span>
            <button onClick={() => setNotice(null)} className="font-bold" aria-label="Cerrar">
              ✕
            </button>
          </div>
        </Alert>
      )}

      {/* Generar respaldo */}
      <section className="bg-zinc-900 border border-zinc-800 rounded-xl p-5 flex flex-col gap-4">
        <h3 className="text-sm font-bold text-zinc-300 uppercase tracking-wide">
          Generar respaldo
        </h3>
        <p className="text-xs text-zinc-500">
            Crea una carpeta <code className="text-zinc-400">respaldo-fecha</code> con la base de
          datos, los vaults, los wraps de claves y la sal de instalación, más un{" "}
          <code className="text-zinc-400">manifest.json</code> con la huella SHA-256 de cada
          archivo. Guardalo en un disco externo o nube junto a tu frase semilla.
        </p>
        <div className="flex justify-end">
          <Button onClick={handleCreate} disabled={creating}>
            {creating ? "Generando…" : "Generar respaldo…"}
          </Button>
        </div>

        {created && (
          <div className="border border-zinc-700 rounded-lg p-4 bg-zinc-950 text-xs text-zinc-400 flex flex-col gap-2">
            <div className="flex justify-between gap-4">
              <span className="text-zinc-500">Ruta</span>
              <span className="text-zinc-200 break-all text-right">{created.path}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-zinc-500">Fecha</span>
              <span className="text-zinc-200">{formatDate(created.created_at)}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-zinc-500">Archivos</span>
              <span className="text-zinc-200">
                {created.file_count} · {formatBytes(created.total_bytes)} · esquema v
                {created.schema_version}
              </span>
            </div>
            <div className="flex justify-end gap-2 mt-1">
              <Button variant="secondary" onClick={() => setCreated(null)}>
                Listo
              </Button>
              <Button onClick={() => revealItemInDir(created.path)}>Revelar en carpeta</Button>
            </div>
          </div>
        )}
      </section>

      {/* Restaurar respaldo */}
      <section className="bg-zinc-900 border border-zinc-800 rounded-xl p-5 flex flex-col gap-4">
        <h3 className="text-sm font-bold text-zinc-300 uppercase tracking-wide">
          Restaurar respaldo
        </h3>
        <Alert variant="info" className="my-0">
          La restauración reemplaza la base de datos y las claves de esta instalación por las del
          respaldo. Se cerrará la sesión y los datos en memoria; después volvés a iniciar sesión con
          una contraseña del respaldo (o con la frase semilla).
        </Alert>

        {!pending ? (
          <div className="flex justify-end">
            <Button onClick={handlePickRestore}>Elegir carpeta de respaldo…</Button>
          </div>
        ) : (
          <div className="border border-zinc-700 rounded-lg p-4 bg-zinc-950 flex flex-col gap-3">
            <div className="text-xs text-zinc-400 flex flex-col gap-2">
              <div className="flex justify-between gap-4">
                <span className="text-zinc-500">Carpeta</span>
                <span className="text-zinc-200 break-all text-right">{pending.path}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-zinc-500">Creado</span>
                <span className="text-zinc-200">{formatDate(pending.manifest.created_at)}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-zinc-500">Versión / esquema</span>
                <span className="text-zinc-200">
                  app {pending.manifest.app_version} · esquema v{pending.manifest.schema_version}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-zinc-500">Contenido</span>
                <span className="text-zinc-200">
                  {pending.manifest.files.length} archivos · {formatBytes(pendingTotal)} ·
                  integridad ✓
                </span>
              </div>
            </div>
            <div className="flex justify-end gap-2">
              <Button variant="secondary" onClick={() => setPending(null)} disabled={restoring}>
                Cancelar
              </Button>
              <Button onClick={handleConfirmRestore} disabled={restoring}>
                {restoring ? "Restaurando…" : "Restaurar y cerrar sesión"}
              </Button>
            </div>
          </div>
        )}
      </section>
    </div>
  );
}
