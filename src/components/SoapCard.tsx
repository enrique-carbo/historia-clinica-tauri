import { Entry } from "../stores/useEntryStore";

interface SoapCardProps {
  entry: Entry;
  onClick?: () => void;
}

const SOAP_SECTIONS: { key: string; label: string; letter: string; color: string }[] = [
  { key: "subjetivo", label: "Subjetivo", letter: "S", color: "text-blue-400 bg-blue-900/30 border-blue-800/50" },
  { key: "objetivo", label: "Objetivo", letter: "O", color: "text-teal-400 bg-teal-900/30 border-teal-800/50" },
  { key: "evaluacion", label: "Evaluación", letter: "E", color: "text-amber-400 bg-amber-900/30 border-amber-800/50" },
  { key: "plan", label: "Plan", letter: "P", color: "text-purple-400 bg-purple-900/30 border-purple-800/50" },
];

const STATUS_CONFIG: Record<string, { label: string; color: string }> = {
  ACTIVE: { label: "Activo", color: "bg-yellow-900/30 text-yellow-400" },
  RESOLVED: { label: "Resuelto", color: "bg-green-900/30 text-green-400" },
  COMPLETED: { label: "Completado", color: "bg-zinc-700 text-zinc-300" },
};

function formatDate(dateStr: string) {
  try {
    return new Date(dateStr).toLocaleDateString("es-AR", {
      day: "2-digit",
      month: "2-digit",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return dateStr;
  }
}

export function SoapCard({ entry, onClick }: SoapCardProps) {
  const statusConfig = STATUS_CONFIG[entry.status] || STATUS_CONFIG.ACTIVE;

  return (
    <div
      onClick={onClick}
      className="p-4 rounded-lg bg-zinc-900 border border-zinc-800 hover:border-zinc-700 transition-colors cursor-pointer"
    >
      {/* Header: badge SOAP + estado */}
      <div className="flex items-center justify-between gap-3 mb-2">
        <span className="inline-flex items-center gap-1.5 px-2 py-0.5 rounded text-xs font-medium border bg-blue-900/30 text-blue-400 border-blue-800/50">
          📋 Nota SOAP
        </span>
        <span className={`inline-flex px-1.5 py-0.5 rounded text-[10px] font-medium ${statusConfig.color}`}>
          {statusConfig.label}
        </span>
      </div>

      {/* Título */}
      <h3 className="text-sm font-semibold text-zinc-200 mb-3">{entry.title}</h3>

      {/* Secciones S-O-A-P */}
      <div className="space-y-2 mb-3">
        {SOAP_SECTIONS.map(({ key, label, letter, color }) => {
          const value = entry.payload[key];
          if (!value) return null;
          return (
            <div key={key} className="flex gap-2 text-xs">
              <span
                className={`shrink-0 w-5 h-5 inline-flex items-center justify-center rounded border font-bold text-[10px] ${color}`}
                title={label}
              >
                {letter}
              </span>
              <div className="min-w-0">
                <span className="text-zinc-500 font-medium">{label}: </span>
                <span className="text-zinc-400 break-words whitespace-pre-wrap">{value}</span>
              </div>
            </div>
          );
        })}
      </div>

      {/* Autor y firma */}
      <div className="flex items-center justify-between text-xs mb-2 px-2 py-1.5 bg-zinc-950/50 rounded">
        <div className="flex items-center gap-1.5 min-w-0">
          <span className="text-zinc-500 shrink-0">Firmado por:</span>
          <span className="text-zinc-300 font-medium truncate">{entry.author_name}</span>
        </div>
        <div className="flex items-center gap-1 shrink-0">
          {entry.is_verified ? (
            <span className="flex items-center gap-1 text-green-400" title={`Firma verificada: ${entry.signature.slice(0, 16)}...`}>
              <svg className="w-3.5 h-3.5" fill="currentColor" viewBox="0 0 20 20">
                <path fillRule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clipRule="evenodd" />
              </svg>
              Verificada
            </span>
          ) : (
            <span className="flex items-center gap-1 text-yellow-500" title="Firma no verificada">
              <svg className="w-3.5 h-3.5" fill="currentColor" viewBox="0 0 20 20">
                <path fillRule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clipRule="evenodd" />
              </svg>
              Sin verificar
            </span>
          )}
        </div>
      </div>

      {/* Footer */}
      <div className="flex items-center justify-between text-[10px] text-zinc-600">
        <span>{formatDate(entry.timestamp)}</span>
        <span>ID: {entry.id.slice(0, 8)}...</span>
      </div>
    </div>
  );
}
