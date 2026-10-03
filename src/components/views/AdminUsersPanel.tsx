import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "../ui/Button";
import { Alert } from "../ui/Alert";
import { Input } from "../ui/Input";
import { useAuthStore } from "../../stores/useAuthStore";

interface UserRow {
  id: string;
  username: string;
  role: string;
  is_active: boolean;
  created_at: string;
  last_login_at: string | null;
}

const ROLE_LABELS: Record<string, string> = {
  administrador: "Administrador",
  asistente: "Asistente",
  medico: "Médico",
  enfermeria: "Enfermería",
  paciente: "Paciente",
};

const ROLE_BADGE: Record<string, string> = {
  administrador: "bg-purple-900/40 text-purple-300 border-purple-700/50",
  asistente: "bg-sky-900/40 text-sky-300 border-sky-700/50",
  medico: "bg-blue-900/40 text-blue-300 border-blue-700/50",
  enfermeria: "bg-cyan-900/40 text-cyan-300 border-cyan-700/50",
  paciente: "bg-zinc-800 text-zinc-300 border-zinc-700",
};

function formatDate(iso: string | null): string {
  if (!iso) return "—";
  try {
    return new Date(iso).toLocaleString();
  } catch {
    return iso;
  }
}

export function AdminUsersPanel() {
  const { activeUser } = useAuthStore();
  const [users, setUsers] = useState<UserRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const [showCreate, setShowCreate] = useState(false);
  const [newUsername, setNewUsername] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [newRole, setNewRole] = useState("medico");
  const [creating, setCreating] = useState(false);

  const [resetTarget, setResetTarget] = useState<UserRow | null>(null);
  const [resetPassword, setResetPassword] = useState("");
  const [resetting, setResetting] = useState(false);

  const load = async () => {
    try {
      setUsers(await invoke<UserRow[]>("list_users"));
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    load();
  }, []);

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    setCreating(true);
    setNotice(null);
    setError(null);
    try {
      await invoke("admin_create_user", {
        username: newUsername.trim(),
        passwordPlain: newPassword,
        role: newRole,
      });
      setNotice(`Usuario «${newUsername.trim()}» creado.`);
      setNewUsername("");
      setNewPassword("");
      setNewRole("medico");
      setShowCreate(false);
      await load();
    } catch (err) {
      setError(String(err));
    } finally {
      setCreating(false);
    }
  };

  const handleToggleActive = async (user: UserRow) => {
    setNotice(null);
    setError(null);
    try {
      await invoke("set_user_active", { userId: user.id, active: !user.is_active });
      setNotice(
        `Usuario «${user.username}» ${user.is_active ? "desactivado" : "activado"}.`
      );
      await load();
    } catch (err) {
      setError(String(err));
    }
  };

  const handleReset = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!resetTarget) return;
    setResetting(true);
    setError(null);
    try {
      await invoke("admin_reset_password", {
        targetUserId: resetTarget.id,
        newPassword: resetPassword,
      });
      setNotice(
        `Contraseña de «${resetTarget.username}» reseteada. Debe usar la contraseña nueva para entrar.`
      );
      setResetTarget(null);
      setResetPassword("");
      await load();
    } catch (err) {
      setError(String(err));
    } finally {
      setResetting(false);
    }
  };

  return (
    <div className="max-w-5xl mx-auto flex flex-col gap-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold text-zinc-300">Gestión de Usuarios</h2>
          <p className="text-xs text-zinc-500 mt-1">
            Alta, baja lógica y recuperación de contraseñas. El historial nunca se
            borra.
          </p>
        </div>
        <Button onClick={() => setShowCreate((v) => !v)}>
          {showCreate ? "Cancelar" : "+ Nuevo usuario"}
        </Button>
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

      {/* Alta de usuarios */}
      {showCreate && (
        <form
          onSubmit={handleCreate}
          className="bg-zinc-900 border border-zinc-800 rounded-xl p-5 flex flex-col gap-4"
        >
          <h3 className="text-sm font-bold text-zinc-300 uppercase tracking-wide">
            Nuevo usuario
          </h3>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <Input
              type="text"
              label="Usuario"
              value={newUsername}
              onChange={(e) => setNewUsername(e.target.value)}
              required
              placeholder="ej: dra.perez"
            />
            <Input
              type="password"
              label="Contraseña"
              value={newPassword}
              onChange={(e) => setNewPassword(e.target.value)}
              required
              minLength={8}
              placeholder="mínimo 8 caracteres"
            />
            <div className="relative">
              <label className="block text-xs text-zinc-500 mb-1 font-semibold">
                Rol
              </label>
              <select
                value={newRole}
                onChange={(e) => setNewRole(e.target.value)}
                className="w-full bg-zinc-950 text-zinc-200 border border-zinc-800 rounded px-3 py-2.5 pr-8 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 appearance-none"
              >
                <option value="medico">Médico (Consultorio / SOAP)</option>
                <option value="enfermeria">Enfermería</option>
                <option value="asistente">Asistente (Recepción / Altas)</option>
                <option value="administrador">Administrador (Gestión técnica)</option>
                <option value="paciente">Paciente</option>
              </select>
              <div className="pointer-events-none absolute inset-y-0 right-0 flex items-center pr-3 top-7">
                <svg
                  className="h-4 w-4 text-zinc-500"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M19 9l-7 7-7-7"
                  />
                </svg>
              </div>
            </div>
          </div>
          <div className="flex justify-end">
            <Button type="submit" isLoading={creating}>
              Crear usuario
            </Button>
          </div>
        </form>
      )}

      {/* Listado */}
      <div className="bg-zinc-900 border border-zinc-800 rounded-xl overflow-hidden">
        {loading ? (
          <p className="p-6 text-sm text-zinc-500">Cargando usuarios…</p>
        ) : (
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-zinc-800 text-left text-xs uppercase tracking-wider text-zinc-500">
                <th className="px-4 py-3">Usuario</th>
                <th className="px-4 py-3">Rol</th>
                <th className="px-4 py-3">Estado</th>
                <th className="px-4 py-3">Último acceso</th>
                <th className="px-4 py-3 text-right">Acciones</th>
              </tr>
            </thead>
            <tbody>
              {users.map((u) => {
                const isSelf = u.id === activeUser?.user_id;
                return (
                  <tr
                    key={u.id}
                    className={`border-b border-zinc-800/60 last:border-0 ${
                      u.is_active ? "" : "opacity-50"
                    }`}
                  >
                    <td className="px-4 py-3 font-medium text-zinc-200">
                      {u.username}
                      {isSelf && (
                        <span className="ml-2 text-[10px] text-zinc-500 uppercase">
                          (vos)
                        </span>
                      )}
                    </td>
                    <td className="px-4 py-3">
                      <span
                        className={`inline-block text-[10px] font-bold uppercase border px-1.5 py-0.5 rounded ${
                          ROLE_BADGE[u.role] || ROLE_BADGE.paciente
                        }`}
                      >
                        {ROLE_LABELS[u.role] || u.role}
                      </span>
                    </td>
                    <td className="px-4 py-3">
                      <span
                        className={`text-xs font-semibold ${
                          u.is_active ? "text-emerald-400" : "text-red-400"
                        }`}
                      >
                        ● {u.is_active ? "Activo" : "Desactivado"}
                      </span>
                    </td>
                    <td className="px-4 py-3 text-zinc-400">
                      {formatDate(u.last_login_at)}
                    </td>
                    <td className="px-4 py-3 text-right">
                      <div className="inline-flex gap-2">
                        <button
                          onClick={() => handleToggleActive(u)}
                          disabled={isSelf}
                          title={
                            isSelf
                              ? "No podés desactivar tu propia cuenta"
                              : u.is_active
                                ? "Desactivar (baja lógica)"
                                : "Reactivar"
                          }
                          className="px-2.5 py-1 text-xs font-semibold rounded border border-zinc-700 text-zinc-300 hover:bg-zinc-800 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
                        >
                          {u.is_active ? "Desactivar" : "Activar"}
                        </button>
                        <button
                          onClick={() => {
                            setResetTarget(u);
                            setResetPassword("");
                            setError(null);
                          }}
                          disabled={isSelf}
                          title={
                            isSelf
                              ? "Usá «Cambiar contraseña» desde tu perfil"
                              : "Resetear contraseña (vault regenerado)"
                          }
                          className="px-2.5 py-1 text-xs font-semibold rounded border border-amber-800/60 text-amber-400 hover:bg-amber-950/40 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
                        >
                          Reset contraseña
                        </button>
                      </div>
                    </td>
                  </tr>
                );
              })}
              {users.length === 0 && (
                <tr>
                  <td colSpan={5} className="px-4 py-6 text-center text-zinc-500">
                    No hay usuarios.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        )}
      </div>

      {/* Modal de reset de contraseña */}
      {resetTarget && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm p-4">
          <form
            onSubmit={handleReset}
            className="bg-zinc-900 border border-zinc-700 rounded-xl p-6 w-full max-w-md shadow-2xl"
          >
            <h3 className="text-base font-bold text-zinc-100 mb-1">
              Resetear contraseña
            </h3>
            <p className="text-xs text-zinc-500 mb-4">
              Usuario: <span className="text-zinc-300 font-semibold">{resetTarget.username}</span>
            </p>

            <div className="bg-amber-950/30 border border-amber-800/50 rounded-lg p-3 mb-4">
              <p className="text-xs text-amber-400">
                Se regenerará la llave de firma de este usuario: sus entries
                firmadas <strong>anteriores a este momento</strong> quedarán con la
                clave vieja (siguen verificándose), y las nuevas usarán la clave
                nueva. La contraseña anterior deja de servir.
              </p>
            </div>

            <Input
              type="password"
              label="Nueva contraseña (mínimo 8)"
              value={resetPassword}
              onChange={(e) => setResetPassword(e.target.value)}
              required
              minLength={8}
              placeholder="••••••••"
            />

            <div className="mt-5 flex justify-end gap-3">
              <Button
                variant="ghost"
                onClick={() => {
                  setResetTarget(null);
                  setResetPassword("");
                }}
                type="button"
              >
                Cancelar
              </Button>
              <Button
                variant="danger"
                type="submit"
                isLoading={resetting}
                disabled={resetPassword.length < 8}
              >
                Resetear
              </Button>
            </div>
          </form>
        </div>
      )}
    </div>
  );
}
