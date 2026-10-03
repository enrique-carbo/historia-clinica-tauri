import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

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

function formatDate(iso: string): string {
  try {
    return new Date(iso).toLocaleString();
  } catch {
    return iso;
  }
}

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
        <h2 className="text-lg font-semibold text-zinc-300">Auditoría</h2>
        <p className="text-xs text-zinc-500 mt-1">
          Registro de acciones sensibles del administrador (últimos 200 eventos).
        </p>
      </div>

      {error && <p className="text-sm text-red-400">{error}</p>}

      <div className="bg-zinc-900 border border-zinc-800 rounded-xl overflow-hidden">
        {loading ? (
          <p className="p-6 text-sm text-zinc-500">Cargando auditoría…</p>
        ) : (
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-zinc-800 text-left text-xs uppercase tracking-wider text-zinc-500">
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
                  className="border-b border-zinc-800/60 last:border-0 text-zinc-300"
                >
                  <td className="px-4 py-3 text-zinc-400 whitespace-nowrap">
                    {formatDate(e.created_at)}
                  </td>
                  <td className="px-4 py-3 font-semibold">
                    {ACTION_LABELS[e.action] || e.action}
                  </td>
                  <td className="px-4 py-3">{e.actor_username || "—"}</td>
                  <td className="px-4 py-3">{e.target_username || "—"}</td>
                  <td className="px-4 py-3 text-zinc-500 text-xs">{e.detail || "—"}</td>
                </tr>
              ))}
              {entries.length === 0 && (
                <tr>
                  <td colSpan={5} className="px-4 py-6 text-center text-zinc-500">
                    Sin eventos registrados.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
