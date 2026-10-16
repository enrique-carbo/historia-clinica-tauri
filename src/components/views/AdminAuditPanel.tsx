import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { formatDateTime } from "../../utils/dateUtils";
import { Alert } from "../ui/Alert";
import { Button } from "../ui/Button";

interface AuditRow {
  id: number;
  actor_user_id: string | null;
  actor_username: string | null;
  action: string;
  target_user_id: string | null;
  target_username: string | null;
  detail: string | null;
  created_at: string;
}

const ACTION_LABELS: Record<string, string> = {
  user_create: "Alta de usuario",
  user_activate: "Reactivación",
  user_deactivate: "Baja de usuario",
  password_reset: "Contraseña reseteada",
  seed_rotate: "Frase semilla rotada",
};

export function AdminAuditPanel() {
  const [entries, setEntries] = useState<AuditRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<AuditRow[]>("list_audit_log")
      .then(setEntries)
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, []);

  return (
    <div className="max-w-5xl mx-auto flex flex-col gap-4">
      <div>
        <h2 className="text-lg font-semibold text-text-primary">Auditoría</h2>
        <p className="text-xs text-text-muted mt-1">
          Registro de acciones sensibles del administrador (últimos 200 eventos).
        </p>
      </div>

      {error && (
        <Alert variant="error">
          <div className="flex justify-between gap-4">
            <span>{error}</span>
            <Button size="icon" variant="ghost" onClick={() => setError(null)} aria-label="Cerrar">
              ✕
            </Button>
          </div>
        </Alert>
      )}

      <div className="bg-surface border border-border rounded-xl overflow-hidden">
        {loading ? (
          <div className="p-6 flex items-center gap-2 text-sm text-text-muted">
            <span className="animate-spin inline-block h-4 w-4 rounded-full border-b-2 border-focus"></span>
            Cargando auditoría…
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-border text-left text-xs uppercase tracking-wider text-text-muted">
                <th className="px-4 py-3">Fecha</th>
                <th className="px-4 py-3">Acción</th>
                <th className="px-4 py-3">Ejecutada por</th>
                <th className="px-4 py-3">Sobre</th>
                <th className="px-4 py-3">Detalle</th>
              </tr>
            </thead>
            <tbody>
              {entries.map((e) => (
                <tr
                  key={e.id}
                  className="border-b border-border/60 last:border-0 text-text-primary"
                >
                  <td className="px-4 py-3 text-text-secondary whitespace-nowrap">
                      {formatDateTime(e.created_at)}
                  </td>
                  <td className="px-4 py-3 font-semibold">
                    {ACTION_LABELS[e.action] || e.action}
                  </td>
                  <td className="px-4 py-3">{e.actor_username || "—"}</td>
                  <td className="px-4 py-3">{e.target_username || "—"}</td>
                  <td className="px-4 py-3 text-text-muted text-xs">{e.detail || "—"}</td>
                </tr>
              ))}
              {entries.length === 0 && (
                <tr>
                  <td colSpan={5} className="px-4 py-6 text-center text-text-muted">
                    Sin eventos registrados.
                  </td>
                </tr>
              )}
            </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
