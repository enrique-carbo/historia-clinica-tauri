import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "./ui/Button";
import { Alert } from "./ui/Alert";
import { Input } from "./ui/Input";
import { useAuthStore } from "../stores/useAuthStore";

export function AuthBox() {
  const [bootstrap, setBootstrap] = useState<boolean | null>(null);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [needsSeed, setNeedsSeed] = useState(false);
  const [seedPhrase, setSeedPhrase] = useState("");

  // <-- Extraemos la función que actualiza el estado global
  const unlockVault = useAuthStore((state) => state.unlockVault);

  // ¿La instalación no tiene usuarios? → modo bootstrap (crear el primer
  // administrador). Si ya hay usuarios, el registro está cerrado.
  useEffect(() => {
    invoke<number>("count_users")
      .then((count) => setBootstrap(count === 0))
      .catch(() => setBootstrap(false));
  }, []);

  const doLogin = async (seed?: string) => {
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
      seedPhrase: seed ?? null,
    });

    // INYECTAMOS DIRECTAMENTE EN EL ESTADO GLOBAL:
    unlockVault(user);
  };

  const handleSubmit = async (e: React.SubmitEvent) => {
    e.preventDefault();
    setError(null);
    setLoading(true);

    try {
      if (bootstrap) {
        // Bootstrap: crea el primer usuario como administrador (el backend
        // fuerza el rol y rechaza si ya existe algún usuario).
        await invoke<string>("register_user", {
          form: { username, password_plain: password },
        });
        // Pasamos al login con las credenciales recién creadas
        await doLogin();
      } else {
        try {
          await doLogin();
        } catch (err) {
          if (String(err).includes("SEED_REQUIRED")) {
            setNeedsSeed(true);
            setError(null);
            return;
          }
          throw err;
        }
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleSeedSubmit = async (e: React.SubmitEvent) => {
    e.preventDefault();
    setError(null);
    setLoading(true);

    try {
      await doLogin(seedPhrase.trim());
      setNeedsSeed(false);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  if (bootstrap === null) {
    return (
      <div className="w-full max-w-md mx-auto mt-24 bg-zinc-900 p-8 rounded-lg border border-zinc-800 text-center text-zinc-500 text-sm">
        Verificando instalación…
      </div>
    );
  }

  return (
    <div className="w-full max-w-md mx-auto mt-24 bg-zinc-900 p-8 rounded-lg border border-zinc-800 shadow-2xl">
      <h2 className="text-xl font-bold text-zinc-100 mb-1">
        {bootstrap
          ? "🌱 Crear Primer Administrador"
          : "🔑 Iniciar Sesión Core"}
      </h2>
      <p className="text-xs text-zinc-500 mb-6">
        {bootstrap
          ? "Esta instalación no tiene usuarios. Creá la cuenta administradora (será la dueña de la gestión técnica)."
          : "Acceso local encriptado mediante Argon2id"}
      </p>

      {error && <Alert variant="error" className="my-3">{error}</Alert>}

      {/* Formulario de seed (bootstrap / recovery) */}
      {!bootstrap && needsSeed ? (
        <form onSubmit={handleSeedSubmit} className="flex flex-col gap-4">
          <div className="p-3 bg-yellow-950/30 border border-yellow-800/50 rounded-lg">
            <p className="text-xs text-yellow-400">
              🌱 Este usuario necesita la <strong>frase semilla</strong> (6 palabras del papel) para
              acceder a los datos cifrados de esta instalación.
            </p>
          </div>
          <div>
            <Input
              type="text"
              label="Frase Semilla"
              value={seedPhrase}
              onChange={(e) => setSeedPhrase(e.target.value)}
              required
              placeholder="palabra1 palabra2 ... palabra6"
            />
          </div>
          <Button type="submit" isLoading={loading} className="w-full">
            Desbloquear con Seed
          </Button>
          <Button
            variant="ghost"
            onClick={() => {
              setNeedsSeed(false);
              setSeedPhrase("");
              setError(null);
            }}
            className="w-full underline text-center"
          >
            Volver
          </Button>
        </form>
      ) : (
        <form onSubmit={handleSubmit} className="flex flex-col gap-4">
        {/* Input Usuario */}
        <div>
          <Input
            type="text"
            label="Usuario"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            required
            placeholder={bootstrap ? "ej: admin_principal" : "ej: enrique_dev"}
          />
        </div>

        {/* Input Contraseña */}
        <div>
          <Input
            type="password"
            label={bootstrap ? "Contraseña (mínimo 8)" : "Contraseña"}
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
            placeholder="••••••••"
            minLength={8}
          />
        </div>

        {bootstrap && (
          <div className="p-3 bg-blue-950/30 border border-blue-800/50 rounded-lg">
            <p className="text-xs text-blue-400">
              Esta cuenta quedará con rol <strong>Administrador</strong>: podrá dar de
              alta al resto de los usuarios, resetear contraseñas y rotar la frase
              semilla. Después de crearla no habrá más registro libre.
            </p>
          </div>
        )}

        {/* Botón Principal */}
        <Button type="submit" isLoading={loading} className="w-full mt-2">
          {bootstrap ? "Crear Administrador" : "Ingresar al Sistema"}
        </Button>
      </form>
      )}
    </div>
  );
}
