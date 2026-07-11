export function PacienteView() {


  return (
    <div className="max-w-3xl mx-auto flex flex-col gap-8">
      <div>
        <h2 className="text-lg font-semibold mb-2 text-zinc-300">
          📬 Mi Buzón Clínico
        </h2>
        <p className="text-sm text-zinc-500">
          Espacio seguro para la teleconsulta asíncrona y seguimiento de
          variables.
        </p>
      </div>

      <hr className="border-zinc-800" />

      <div className="p-8 text-center bg-zinc-900/50 rounded-lg border border-dashed border-zinc-800 text-zinc-600">
        <p className="text-3xl mb-3">🛠️</p>
        <p className="text-sm font-semibold text-zinc-500">
          Módulo de Teleconsulta Asíncrona
        </p>
        <p className="text-xs mt-1">
          (Disponible en la Fase 5. Aquí se desplegarán los hilos de mensajes
          cifrados de extremo a extremo con su médico).
        </p>
      </div>

      <hr className="border-zinc-800" />

      <div className="p-6 bg-zinc-900 rounded-lg border border-zinc-800">
        <h3 className="text-base font-semibold text-zinc-200 mb-4">
          📊 Registrar Mi Métrica Diaria
        </h3>
      </div>

      <div>
        <h3 className="text-base font-semibold text-zinc-200 mb-4">
          Historial de Mis Registros
        </h3>

      </div>

      <div className="p-4 bg-blue-950/30 border border-blue-900 rounded-lg text-xs text-blue-400 text-center">
        🔒 <strong>Privacidad Garantizada:</strong> Toda comunicación aquí será
        cifrada de extremo a extremo. Ni siquiera los servidores de Simplex
        podrán leer sus síntomas o mensajes.
      </div>
    </div>
  );
}
