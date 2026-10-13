import { ChangePasswordForm } from "../ChangePasswordForm";
import { useAuthStore } from "../../stores/useAuthStore";

export function PacienteView() {
  const { activeUser } = useAuthStore();

  return (
    <div className="max-w-3xl mx-auto flex flex-col gap-8">
      <div>
        <h2 className="text-lg font-semibold mb-2 text-zinc-300">
          👤 Mi Cuenta
        </h2>
        <p className="text-sm text-zinc-500">
          Tu usuario de acceso y la seguridad de tu cuenta.
        </p>
      </div>

      <div className="p-6 bg-zinc-900 rounded-xl border border-zinc-800 shadow-lg">
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-5">
          <div>
            <label className="block text-xs font-semibold text-zinc-500 mb-1.5 uppercase">
              Usuario
            </label>
            <p className="text-sm text-zinc-200 font-mono font-medium py-2.5 border-b border-zinc-800/50">
              {activeUser?.username || "—"}
            </p>
          </div>
          <div>
            <label className="block text-xs font-semibold text-zinc-500 mb-1.5 uppercase">
              Rol
            </label>
            <p className="text-sm text-zinc-200 font-medium py-2.5 border-b border-zinc-800/50 uppercase">
              {activeUser?.role || "—"}
            </p>
          </div>
        </div>

        <div className="mt-8 pt-6 border-t border-zinc-800">
          <ChangePasswordForm />
        </div>
      </div>

      <div className="p-4 bg-blue-950/30 border border-blue-900 rounded-lg text-xs text-blue-400 text-center">
        🔒 <strong>Privacidad Garantizada:</strong> Tus datos clínicos se
        guardan cifrados de extremo a extremo. Ni siquiera los servidores de
        Fichio Care pueden leerlos.
      </div>
    </div>
  );
}
