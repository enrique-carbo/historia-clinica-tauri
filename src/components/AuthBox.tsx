import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "./ui/Button";
import { Alert } from "./ui/Alert";
import { Input } from "./ui/Input";
import { useAuthStore } from "../stores/useAuthStore";

export function AuthBox() {
  const [isLogin, setIsLogin] = useState(true);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [role, setRole] = useState("medico");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  // <-- Extraemos la función que actualiza el estado global
  const unlockVault = useAuthStore((state) => state.unlockVault);

  const handleSubmit = async (e: React.SubmitEvent) => {
    e.preventDefault();
    setError(null);
    setLoading(true);

    try {
      if (isLogin) {
        const user = await invoke<{
          user_id: string;
          username: string;
          role: string;
        }>("login_user", {
          form: { username, password_plain: password },
        });

        await invoke("unlock_vault", {
          userId: user.user_id,
          password: password,
        });

        // INYECTAMOS DIRECTAMENTE EN EL ESTADO GLOBAL:
        unlockVault(user);
      } else {
        await invoke<string>("register_user", {
          form: { username, password_plain: password, role },
        });

        setIsLogin(true);
        setPassword("");
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="w-full max-w-md mx-auto mt-24 bg-zinc-900 p-8 rounded-lg border border-zinc-800 shadow-2xl">
      <h2 className="text-xl font-bold text-zinc-100 mb-1">
        {isLogin ? "🔑 Iniciar Sesión Core" : "👤 Registrar Nuevo Usuario"}
      </h2>
      <p className="text-xs text-zinc-500 mb-6">
        {isLogin
          ? "Acceso local encriptado mediante Argon2id"
          : "El hash se calculará de forma aislada en la RAM"}
      </p>

      {error && <Alert variant="error">{error}</Alert>}

      <form onSubmit={handleSubmit} className="flex flex-col gap-4">
        {/* Input Usuario */}
        <div>
          <Input
            type="text"
            label="Usuario"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            required
            placeholder="ej: enrique_dev"
          />
        </div>

        {/* Input Contraseña */}
        <div>
          <Input
            type="password"
            label="Contraseña"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
            placeholder="••••••••"
            minLength={8}
          />
        </div>

        {/* Select Rol (Solo en Registro) */}
        {!isLogin && (
          <div className="relative">
            <label className="block text-xs text-zinc-500 mb-1 font-semibold">
              Rol Asignado
            </label>
            <select
              value={role}
              onChange={(e) => setRole(e.target.value)}
              className="w-full bg-zinc-950 text-zinc-200 border border-zinc-800 rounded px-3 py-2.5 pr-8 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 appearance-none"
            >
              <option value="medico">Médico (Consultorio / SOAP)</option>
              <option value="admin">Administrador (Recepción / Altas)</option>
              <option value="paciente">
                Paciente (Auto-registro / Métricas)
              </option>
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
        )}

        {/* Botón Principal */}
        <Button type="submit" isLoading={loading} className="w-full mt-2">
          {isLogin ? "Ingresar al Sistema" : "Crear Cuenta Local"}
        </Button>
      </form>

      {/* Toggle Login/Registro */}
      <Button
        variant="ghost"
        onClick={() => {
          setIsLogin(!isLogin);
          setError(null);
        }}
        className="w-full mt-5 underline text-center"
      >
        {isLogin
          ? "¿No tenés usuario local? Registrarse"
          : "¿Ya tenés usuario? Iniciar Sesión"}
      </Button>
    </div>
  );
}
