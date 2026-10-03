import { useState } from "react";
import { Entry, EntryCategory } from "../stores/useEntryStore";

interface EntryCardProps {
  entry: Entry;
  onClick?: () => void;
}

const CATEGORY_CONFIG: Record<EntryCategory, { label: string; icon: string; color: string }> = {
  SOAP_NOTE: {
    label: "Nota SOAP",
    icon: "📋",
    color: "bg-blue-900/30 text-blue-400 border-blue-800/50",
  },
  MEDICATION: {
    label: "Medicación",
    icon: "💊",
    color: "bg-green-900/30 text-green-400 border-green-800/50",
  },
  ALLERGY: {
    label: "Alergia",
    icon: "⚠️",
    color: "bg-red-900/30 text-red-400 border-red-800/50",
  },
  CONDITION: {
    label: "Condición",
    icon: "🏥",
    color: "bg-purple-900/30 text-purple-400 border-purple-800/50",
  },
};

const STATUS_CONFIG: Record<string, { label: string; color: string }> = {
  ACTIVE: { label: "Activo", color: "bg-yellow-900/30 text-yellow-400" },
  RESOLVED: { label: "Resuelto", color: "bg-green-900/30 text-green-400" },
  COMPLETED: { label: "Completado", color: "bg-zinc-700 text-zinc-300" },
};

// Orden canónico de campos según template (el payload llega como HashMap → orden aleatorio)
const FIELD_ORDER: Record<string, string[]> = {
  SOAP_NOTE: ["subjetivo", "objetivo", "evaluacion", "plan"],
  ALLERGY: ["alergeno", "reaccion", "severidad"],
  MEDICATION: ["nombre", "dosis", "frecuencia", "duracion"],
  CONDITION: ["diagnostico", "estado", "notas"],
};

const FIELD_LABELS: Record<string, string> = {
  subjetivo: "S — Subjetivo",
  objetivo: "O — Objetivo",
  evaluacion: "E — Evaluación",
  plan: "P — Plan",
};

function formatKey(key: string): string {
  if (FIELD_LABELS[key]) return FIELD_LABELS[key];
  return key
    .split(/[_-]/)
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(" ");
}

function sortPayload(category: string, payload: Record<string, string>): [string, string][] {
  const order = FIELD_ORDER[category] ?? [];
  return Object.entries(payload).sort(([a], [b]) => {
    const ia = order.indexOf(a);
    const ib = order.indexOf(b);
    if (ia !== -1 && ib !== -1) return ia - ib;
    if (ia !== -1) return -1;
    if (ib !== -1) return 1;
    return a.localeCompare(b);
  });
}

export function EntryCard({ entry, onClick }: EntryCardProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const categoryConfig = CATEGORY_CONFIG[entry.category] || CATEGORY_CONFIG.SOAP_NOTE;
  const statusConfig = STATUS_CONFIG[entry.status] || STATUS_CONFIG.ACTIVE;

  const formatDate = (dateStr: string) => {
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
  };

  const handleToggle = () => {
    setIsExpanded((prev) => !prev);
    onClick?.();
  };

  return (
    <div
      onClick={handleToggle}
      className={`p-4 rounded-lg bg-zinc-900 border transition-colors cursor-pointer ${
        isExpanded ? "border-zinc-700" : "border-zinc-800 hover:border-zinc-700"
      }`}
    >
      {/* Header: categoría + estado */}
      <div className="flex items-start justify-between gap-3 mb-2">
        <div className="flex items-center gap-2">
          <span className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded text-xs font-medium border ${categoryConfig.color}`}>
            {categoryConfig.icon} {categoryConfig.label}
          </span>
          <span className={`inline-flex px-1.5 py-0.5 rounded text-[10px] font-medium ${statusConfig.color}`}>
            {statusConfig.label}
          </span>
        </div>
        {/* Chevron indicador de expansión */}
        <svg
          className={`w-4 h-4 text-zinc-500 transition-transform duration-200 ${isExpanded ? "rotate-180" : ""}`}
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
        </svg>
      </div>

      {/* Título */}
      <h3 className="text-sm font-semibold text-zinc-200 mb-2">{entry.title}</h3>

      {/* Autor y firma (siempre visibles) */}
      <div className="flex items-center justify-between text-xs px-2 py-1.5 bg-zinc-950/50 rounded">
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

      {/* Payload (solo expandido) */}
      {isExpanded && (
        <div className="mt-3 pt-3 border-t border-zinc-800">
          <div className="text-[10px] text-zinc-500 uppercase font-bold tracking-wider mb-2">
            Contenido
          </div>
          <div className="space-y-2">
            {sortPayload(entry.category, entry.payload).map(([key, value]) => (
              <div
                key={key}
                className="bg-zinc-950/50 border border-zinc-800/50 rounded-lg px-3 py-2"
              >
                <div className="text-[10px] text-zinc-500 uppercase font-bold tracking-wider mb-1">
                  {formatKey(key)}
                </div>
                <div className="text-xs text-zinc-300 whitespace-pre-wrap wrap-break-words leading-relaxed">
                  {value || "—"}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Footer */}
      <div className="flex items-center justify-between text-[10px] text-zinc-500 mt-2">
        <span>{formatDate(entry.timestamp)}</span>
        <span>ID: {entry.id.slice(0, 8)}...</span>
      </div>
    </div>
  );
}
