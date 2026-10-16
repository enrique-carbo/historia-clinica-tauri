import { useState } from "react";
import { Entry } from "../stores/useEntryStore";
import {
  CategoryBadge,
  EntryFooter,
  SignatureStatus,
  SignedBy,
  StatusBadge,
} from "./EntryMeta";

interface EntryCardProps {
  entry: Entry;
  onClick?: () => void;
}

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
  evaluacion: "A — Análisis / Evaluación",
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

  const handleToggle = () => {
    setIsExpanded((prev) => !prev);
    onClick?.();
  };

  return (
    <div
      role="button"
      tabIndex={0}
      onClick={handleToggle}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          handleToggle();
        }
      }}
      aria-expanded={isExpanded}
      className={`p-4 rounded-lg bg-surface border transition-colors cursor-pointer ${
        isExpanded ? "border-border-strong" : "border-border hover:border-border-strong"
      }`}
    >
      {/* Header: categoría + estado */}
      <div className="flex items-start justify-between gap-3 mb-2">
        <div className="flex items-center gap-2">
          <CategoryBadge category={entry.category} />
          <StatusBadge status={entry.status} />
        </div>
        {/* Chevron indicador de expansión */}
        <svg
          aria-hidden="true"
          className={`w-4 h-4 text-text-muted transition-transform duration-200 ${isExpanded ? "rotate-180" : ""}`}
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
        </svg>
      </div>

      {/* Título */}
      <h3 className="text-sm font-semibold text-text-primary mb-2">{entry.title}</h3>

      {/* Autor y firma (siempre visibles) */}
      <div className="flex flex-wrap gap-2 items-center justify-between text-xs px-2 py-1.5 bg-canvas/50 rounded">
        <SignedBy name={entry.author_name} profession={entry.author_profession} />
        <div className="flex items-center gap-1 shrink-0">
          <SignatureStatus isVerified={entry.is_verified} signature={entry.signature} />
        </div>
      </div>

      {/* Payload (solo expandido) */}
      {isExpanded && (
        <div className="mt-3 pt-3 border-t border-border">
          <div className="text-[10px] text-text-muted uppercase font-bold tracking-wider mb-2">
            Contenido
          </div>
          <div className="space-y-2">
            {sortPayload(entry.category, entry.payload).map(([key, value]) => (
              <div
                key={key}
                className="bg-canvas/50 border border-border/50 rounded-lg px-3 py-2"
              >
                <div className="text-[10px] text-text-muted uppercase font-bold tracking-wider mb-1">
                  {formatKey(key)}
                </div>
                <div className="text-xs text-text-primary whitespace-pre-wrap wrap-break-words leading-relaxed">
                  {value || "—"}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Footer */}
      <EntryFooter timestamp={entry.timestamp} id={entry.id} className="mt-2" />
    </div>
  );
}
