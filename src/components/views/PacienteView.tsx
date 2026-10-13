import { ProfileView } from "../ProfileView";

export function PacienteView() {
  return (
    <div className="max-w-3xl mx-auto flex flex-col gap-8">
      <div>
        <h2 className="text-lg font-semibold mb-2 text-zinc-300">
          👤 Mi Cuenta
        </h2>
        <p className="text-sm text-zinc-500">
          Tu perfil de paciente y la seguridad de tu acceso.
        </p>
      </div>

      <ProfileView />

      <div className="p-4 bg-blue-950/30 border border-blue-900 rounded-lg text-xs text-blue-400 text-center">
        🔒 <strong>Privacidad Garantizada:</strong> Tus datos clínicos se
        guardan cifrados de extremo a extremo. Ni siquiera los servidores de
        Fichio Care pueden leerlos.
      </div>
    </div>
  );
}
