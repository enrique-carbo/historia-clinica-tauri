// src/components/ChangePasswordForm.tsx
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export function ChangePasswordForm() {
  const [pwdForm, setPwdForm] = useState({
    current: "",
    new: "",
    confirm: "",
  });
  const [pwdLoading, setPwdLoading] = useState(false);
  const [pwdMsg, setPwdMsg] = useState<{ type: "ok" | "err"; text: string } | null>(
    null
  );

  const handleChangePassword = async () => {
    setPwdMsg(null);

    if (pwdForm.new.length < 8) {
      setPwdMsg({
        type: "err",
        text: "La nueva contraseña debe tener al menos 8 caracteres.",
      });
      return;
    }
    if (pwdForm.new !== pwdForm.confirm) {
      setPwdMsg({ type: "err", text: "Las contraseñas nuevas no coinciden." });
      return;
    }

    setPwdLoading(true);
    try {
      await invoke("change_password", {
        currentPassword: pwdForm.current,
        newPassword: pwdForm.new,
      });
      setPwdMsg({ type: "ok", text: "Contraseña cambiada correctamente." });
      setPwdForm({ current: "", new: "", confirm: "" });
    } catch (e) {
      setPwdMsg({ type: "err", text: String(e) });
    } finally {
      setPwdLoading(false);
    }
  };

  return (
    <div>
      <h3 className="text-xs font-bold text-warning-text uppercase tracking-widest mb-4">
        <span aria-hidden="true">🔒</span> Seguridad — Cambiar contraseña
      </h3>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-5 max-w-3xl">
        <div>
          <label
            htmlFor="pwd-current"
            className="block text-xs font-semibold text-text-muted mb-1.5 uppercase"
          >
            Contraseña Actual
          </label>
          <input
            id="pwd-current"
            type="password"
            autoComplete="current-password"
            value={pwdForm.current}
            onChange={(e) => setPwdForm((p) => ({ ...p, current: e.target.value }))}
            className="w-full rounded-lg bg-field border border-border-strong px-3 py-2.5 text-sm text-text-primary focus:border-focus outline-none transition-colors"
          />
        </div>
        <div>
          <label
            htmlFor="pwd-new"
            className="block text-xs font-semibold text-text-muted mb-1.5 uppercase"
          >
            Nueva Contraseña
          </label>
          <input
            id="pwd-new"
            type="password"
            autoComplete="new-password"
            value={pwdForm.new}
            onChange={(e) => setPwdForm((p) => ({ ...p, new: e.target.value }))}
            className="w-full rounded-lg bg-field border border-border-strong px-3 py-2.5 text-sm text-text-primary focus:border-focus outline-none transition-colors"
          />
        </div>
        <div>
          <label
            htmlFor="pwd-confirm"
            className="block text-xs font-semibold text-text-muted mb-1.5 uppercase"
          >
            Repetir Nueva
          </label>
          <input
            id="pwd-confirm"
            type="password"
            autoComplete="new-password"
            value={pwdForm.confirm}
            onChange={(e) => setPwdForm((p) => ({ ...p, confirm: e.target.value }))}
            className="w-full rounded-lg bg-field border border-border-strong px-3 py-2.5 text-sm text-text-primary focus:border-focus outline-none transition-colors"
          />
        </div>
      </div>

      <div className="mt-4 flex items-center gap-4">
        <button
          onClick={handleChangePassword}
          disabled={
            pwdLoading || !pwdForm.current || !pwdForm.new || !pwdForm.confirm
          }
          className="rounded-lg bg-amber-600 px-5 py-2.5 text-sm font-bold text-text-inverse hover:bg-amber-700 disabled:opacity-50 transition-colors"
        >
          {pwdLoading ? "Cambiando..." : "Cambiar Contraseña"}
        </button>

        {pwdMsg && (
          <span
            className={`text-sm font-medium ${
              pwdMsg.type === "ok" ? "text-success-text" : "text-danger-text"
            }`}
          >
            {pwdMsg.type === "ok" ? "✓ " : "✕ "}
            {pwdMsg.text}
          </span>
        )}
      </div>

      <p className="mt-3 text-xs text-text-muted">
        Cambiar la contraseña re-cifra tu bóveda local con la nueva llave. Tu
        frase semilla en papel no se ve afectada.
      </p>
    </div>
  );
}
