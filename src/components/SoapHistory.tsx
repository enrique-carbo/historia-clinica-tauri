interface SoapHistoryProps {
  records: any[];
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
          className="bg-zinc-900 p-5 rounded-lg border border-zinc-800"
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
          <div className="flex flex-col gap-2 text-sm">
            <div>
              <strong className="text-zinc-400">S:</strong>{" "}
              <span className="text-zinc-300">{record.subjetivo}</span>
            </div>
            <div>
              <strong className="text-zinc-400">O:</strong>{" "}
              <span className="text-zinc-300">{record.objetivo}</span>
            </div>
            <div>
              <strong className="text-zinc-400">A:</strong>{" "}
              <span className="text-zinc-300">{record.analisis}</span>
            </div>
            <div>
              <strong className="text-zinc-400">P:</strong>{" "}
              <span className="text-zinc-300">{record.plan}</span>
            </div>
          </div>

          {/* ID Oculto en el pie */}
          <div className="text-[11px] text-zinc-700 font-mono mt-3">
            ID: {record.id}
          </div>
        </div>
      ))}
    </div>
  );
}
