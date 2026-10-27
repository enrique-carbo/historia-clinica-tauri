// Bloque "Acceso del paciente" (admin/asistente): consulta, alta y estado
// del usuario con el que el paciente entra a la app. Autocontenido: su
// estado y sus invokes viven acá; solo depende de editingId.
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "./ui/Button";

interface PatientAccess {
  user_id: string;
  username: string;
  entity_id: number;
  is_active: boolean;
}

interface PatientAccessPanelProps {
  editingId: number | null;
  canManageAccess: boolean;
}

export function PatientAccessPanel({
  editingId,
  canManageAccess,
}: PatientAccessPanelProps) {
  const [access, setAccess] = useState<PatientAccess | null>(null);
  const [accessLoading, setAccessLoading] = useState(false);
  const [showAccessForm, setShowAccessForm] = useState(false);
  const [accessUsername, setAccessUsername] = useState("");
  const [accessPassword, setAccessPassword] = useState("");
  const [accessMsg, setAccessMsg] = useState<{
    type: "ok" | "err";
    text: string;
  } | null>(null);

  // Cargar estado del acceso al editar una ficha existente
  useEffect(() => {
    if (!editingId || !canManageAccess) return;
    let cancelled = false;
    setAccess(null);
    setAccessMsg(null);
    setShowAccessForm(false);
    setAccessLoading(true);
    invoke<PatientAccess | null>("get_entity_access", { entityId: editingId })
      .then((res) => {
        if (!cancelled) setAccess(res);
      })
      .catch((e) => {
        if (!cancelled) setAccessMsg({ type: "err", text: String(e) });
      })
      .finally(() => {
        if (!cancelled) setAccessLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [editingId, canManageAccess]);

  const handleGrantAccess = async () => {
    if (!editingId) return;
    setAccessMsg(null);
    if (accessPassword.length < 8) {
      setAccessMsg({
        type: "err",
        text: "La contraseña debe tener al menos 8 caracteres.",
      });
      return;
    }
    setAccessLoading(true);
    try {
      const res = await invoke<PatientAccess>("grant_patient_access", {
        entityId: editingId,
        username: accessUsername,
        passwordPlain: accessPassword,
      });
      setAccess(res);
      setShowAccessForm(false);
      setAccessUsername("");
      setAccessPassword("");
      setAccessMsg({ type: "ok", text: `Acceso otorgado: @${res.username}` });
    } catch (e) {
      setAccessMsg({ type: "err", text: String(e) });
    } finally {
      setAccessLoading(false);
    }
  };

  if (!editingId || !canManageAccess) return null;

  return (
    <div className="shrink-0 rounded-xl border border-border bg-surface p-4 sm:p-6">
      <div className="flex items-center justify-between gap-3 mb-4">
        <h3 className="font-semibold text-text-primary uppercase tracking-wider text-sm">
          <span aria-hidden="true">🎟️</span> Acceso del paciente
        </h3>
        {accessMsg && (
          <span
            className={`text-xs font-medium ${
              accessMsg.type === "ok" ? "text-success-text" : "text-danger-text"
            }`}
          >
            {accessMsg.type === "ok" ? "✓ " : "✕ "}
            {accessMsg.text}
          </span>
        )}
      </div>

      {accessLoading ? (
        <div className="flex items-center gap-2 text-sm text-text-muted">
          <span className="animate-spin inline-block h-4 w-4 rounded-full border-b-2 border-focus"></span>
          Consultando…
        </div>
      ) : access && access.is_active ? (
        <p className="text-sm text-text-secondary">
          Esta ficha ya tiene acceso activo:{" "}
          <span className="font-mono text-text-primary">@{access.username}</span>.
          Para revocarlo, desactivalo desde el panel de Usuarios.
        </p>
      ) : (
        <>
          {access && !access.is_active && (
            <p className="text-sm text-text-muted mb-3">
              Acceso anterior <span className="font-mono">@{access.username}</span>{" "}
              (inactivo).
            </p>
          )}
          {showAccessForm ? (
            <div className="grid grid-cols-1 sm:grid-cols-[1fr_1fr_auto] gap-3 items-end">
              <div>
                <label
                  htmlFor="access-username"
                  className="block text-xs font-semibold text-text-muted mb-1.5 uppercase"
                >
                  Usuario
                </label>
                <input
                  id="access-username"
                  value={accessUsername}
                  onChange={(e) => setAccessUsername(e.target.value)}
                  placeholder="ej. maria.gomez"
                  className="w-full rounded-lg bg-field border border-border-strong px-3 py-2.5 text-sm text-text-primary focus:border-focus outline-none transition-colors"
                />
              </div>
              <div>
                <label
                  htmlFor="access-password"
                  className="block text-xs font-semibold text-text-muted mb-1.5 uppercase"
                >
                  Contraseña temporal
                </label>
                <input
                  id="access-password"
                  type="password"
                  value={accessPassword}
                  onChange={(e) => setAccessPassword(e.target.value)}
                  placeholder="mínimo 8 caracteres"
                  className="w-full rounded-lg bg-field border border-border-strong px-3 py-2.5 text-sm text-text-primary focus:border-focus outline-none transition-colors"
                />
              </div>
              <div className="flex gap-2">
                <Button
                  variant="secondary"
                  onClick={() => {
                    setShowAccessForm(false);
                    setAccessMsg(null);
                  }}
                >
                  Cancelar
                </Button>
                <Button
                  onClick={handleGrantAccess}
                  disabled={accessLoading || !accessUsername.trim()}
                >
                  Otorgar
                </Button>
              </div>
            </div>
          ) : (
            <div className="flex items-center justify-between gap-3">
              <p className="text-sm text-text-muted">
                Todavía no tiene usuario para entrar a la app.
              </p>
              <Button onClick={() => setShowAccessForm(true)}>
                Dar acceso al paciente
              </Button>
            </div>
          )}
        </>
      )}
    </div>
  );
}
