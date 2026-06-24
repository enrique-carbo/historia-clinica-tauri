import { SoapRecord } from "../stores/usePatientStore";

interface SoapHistoryProps {
  records: SoapRecord[];
}

export function SoapHistory({ records }: SoapHistoryProps) {
  if (records.length === 0) {
    return (
      <div className="p-8 text-center text-zinc-600 border border-dashed border-zinc-800 rounded-lg">
        No hay consultas previas registradas para este paciente.
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-5 mt-2.5">
      {records.map((record) => (
        <div
          key={record.id}
          className="bg-zinc-900 p-5 rounded-lg border border-zinc-800 hover:border-zinc-700 transition-colors"
        >
          {/* Cabecera con Fecha y Badge de Sync */}
          <div className="flex justify-between items-center border-b border-zinc-800 pb-2.5 mb-3">
            <span className="text-xs font-bold text-blue-500">
              📅 {new Date(record.created_at).toLocaleString()}
            </span>
            <span
              className={`text-[11px] px-2 py-0.5 rounded-full font-semibold ${
                record.is_synced
                  ? "bg-green-950 text-green-400"
                  : "bg-purple-950 text-purple-400"
              }`}
            >
              {record.is_synced ? "Sincronizado" : "Solo Local (Cifrado)"}
            </span>
          </div>

          {/* Cuerpo SOAP */}
          <div className="flex flex-col gap-3 text-sm">
            <div>
              <strong className="text-zinc-400 font-mono">S:</strong>{" "}
              {/* 🚀 whitespace-pre-wrap para que los "Enter" del médico se vean */}
              <span className="text-zinc-300 whitespace-pre-wrap">
                {record.subjetivo}
              </span>
            </div>
            <div>
              <strong className="text-zinc-400 font-mono">O:</strong>{" "}
              <span className="text-zinc-300 whitespace-pre-wrap">
                {record.objetivo}
              </span>
            </div>
            <div>
              <strong className="text-zinc-400 font-mono">A:</strong>{" "}
              <span className="text-zinc-300 whitespace-pre-wrap">
                {record.analisis}
              </span>
            </div>
            <div>
              <strong className="text-zinc-400 font-mono">P:</strong>{" "}
              <span className="text-zinc-300 whitespace-pre-wrap">
                {record.plan}
              </span>
            </div>
          </div>

          {/* ID Oculto en el pie */}
          <div
            className="text-[11px] text-zinc-700 font-mono mt-3 truncate"
            title={record.id}
          >
            ID: {record.id}
          </div>
        </div>
      ))}
    </div>
  );
}
