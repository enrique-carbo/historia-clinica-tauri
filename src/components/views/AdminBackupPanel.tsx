import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { Button } from "../ui/Button";
import { Alert } from "../ui/Alert";
import { useAuthStore } from "../../stores/useAuthStore";
import { formatDateTime } from "../../utils/dateUtils";

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
  signer_user_id: string;
  signature: string;
}

interface BackupValidation {
  manifest: Manifest;
  signature_status: "verified" | "unverifiable";
  signature_warning: string | null;
}

interface BackupInfo {
  path: string;
  app_version: string;
  schema_version: number;
  created_at: string;
  file_count: number;
  total_bytes: number;
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

  const [pending, setPending] = useState<
    | {
        path: string;
        manifest: Manifest;
        signatureStatus: BackupValidation["signature_status"];
        signatureWarning: string | null;
      }
    | null
  >(null);
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
      const validation = await invoke<BackupValidation>("validate_backup", { srcPath: src });
      setPending({
        path: src,
        manifest: validation.manifest,
        signatureStatus: validation.signature_status,
        signatureWarning: validation.signature_warning,
      });
    } catch (e) {
      setError(String(e));
    }
  };

  const handleConfirmRestore = async () => {
    if (!pending) return;
    setRestoring(true);
    setError(null);
    try {
      await invoke("restore_backup", {
        request: {
          srcPath: pending.path,
          forceUnverified: pending.signatureStatus !== "verified",
        },
      });
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
        <h2 className="text-lg font-semibold text-text-primary">Respaldos</h2>
        <p className="text-xs text-text-muted mt-1">
          Copia de seguridad de la base de datos, vaults y wraps de claves. Todo el contenido va
          cifrado en reposo.
        </p>
      </div>

      {error && (
        <Alert variant="error" className="my-0">
          <div className="flex justify-between gap-4">
            <span>{error}</span>
            <Button size="icon" variant="ghost" onClick={() => setError(null)} aria-label="Cerrar">
              ✕
            </Button>
          </div>
        </Alert>
      )}

      {notice && (
        <Alert variant="success" className="my-0">
          <div className="flex justify-between gap-4">
            <span>{notice}</span>
            <Button size="icon" variant="ghost" onClick={() => setNotice(null)} aria-label="Cerrar">
              ✕
            </Button>
          </div>
        </Alert>
      )}

      {/* Generar respaldo */}
      <section className="bg-surface border border-border rounded-xl p-5 flex flex-col gap-4">
        <h3 className="text-sm font-bold text-text-primary uppercase tracking-wide">
          Generar respaldo
        </h3>
        <p className="text-xs text-text-muted">
            Crea una carpeta <code className="text-text-secondary">respaldo-fecha</code> con la base de
          datos, los vaults, los wraps de claves y la sal de instalación, más un{" "}
          <code className="text-text-secondary">manifest.json</code> con la huella SHA-256 de cada
          archivo. Guardalo en un disco externo o nube junto a tu frase semilla.
        </p>
        <div className="flex justify-end">
          <Button onClick={handleCreate} disabled={creating}>
            {creating ? "Generando…" : "Generar respaldo…"}
          </Button>
        </div>

        {created && (
          <div className="border border-border-strong rounded-lg p-4 bg-canvas text-xs text-text-secondary flex flex-col gap-2">
            <div className="flex justify-between gap-4">
              <span className="text-text-muted">Ruta</span>
              <span className="text-text-primary break-all text-right">{created.path}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-text-muted">Fecha</span>
                      <span className="text-text-primary">{formatDateTime(created.created_at)}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-text-muted">Archivos</span>
              <span className="text-text-primary">
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
      <section className="bg-surface border border-border rounded-xl p-5 flex flex-col gap-4">
        <h3 className="text-sm font-bold text-text-primary uppercase tracking-wide">
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
          <div className="border border-border-strong rounded-lg p-4 bg-canvas flex flex-col gap-3">
            <div className="text-xs text-text-secondary flex flex-col gap-2">
              <div className="flex justify-between gap-4">
                <span className="text-text-muted">Carpeta</span>
                <span className="text-text-primary break-all text-right">{pending.path}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-text-muted">Creado</span>
                      <span className="text-text-primary">{formatDateTime(pending.manifest.created_at)}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-text-muted">Versión / esquema</span>
                <span className="text-text-primary">
                  app {pending.manifest.app_version} · esquema v{pending.manifest.schema_version}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-text-muted">Contenido</span>
                <span className="text-text-primary">
                  {pending.manifest.files.length} archivos · {formatBytes(pendingTotal)} ·
                  integridad ✓ ·{" "}
                  {pending.signatureStatus === "verified"
                    ? `firma ✓ (${pending.manifest.signer_user_id})`
                    : "firma ⚠"}
                </span>
              </div>
            </div>
            {pending.signatureStatus !== "verified" && (
              <Alert variant="info" className="my-0">
                ⚠ Firma no verificable por esta instalación:{" "}
                {pending.signatureWarning ?? "motivo desconocido"}. Restaurar solo si confía en
                el origen de este respaldo (p. ej. un respaldo legítmo creado antes de esta
                versión o en otra instalación).
              </Alert>
            )}
            <div className="flex justify-end gap-2">
              <Button variant="secondary" onClick={() => setPending(null)} disabled={restoring}>
                Cancelar
              </Button>
              <Button onClick={handleConfirmRestore} disabled={restoring}>
                {restoring
                  ? "Restaurando…"
                  : pending.signatureStatus === "verified"
                    ? "Restaurar y cerrar sesión"
                    : "Restaurar de todos modos y cerrar sesión"}
              </Button>
            </div>
          </div>
        )}
      </section>
    </div>
  );
}
