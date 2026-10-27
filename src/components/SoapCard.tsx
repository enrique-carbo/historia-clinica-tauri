import { Entry } from "../stores/useEntryStore";
import {
  CategoryBadge,
  EntryFooter,
  SignatureStatus,
  SignedBy,
  StatusBadge,
} from "./EntryMeta";

interface SoapCardProps {
  entry: Entry;
  onClick?: () => void;
}

const SOAP_SECTIONS: { key: string; label: string; letter: string; color: string }[] = [
  { key: "subjetivo", label: "Subjetivo", letter: "S", color: "text-blue-200 bg-blue-900/80 border-blue-800/50" },
  { key: "objetivo", label: "Objetivo", letter: "O", color: "text-teal-200 bg-teal-900/80 border-teal-800/50" },
  { key: "evaluacion", label: "Análisis / Evaluación", letter: "A", color: "text-amber-200 bg-amber-900/80 border-amber-800/50" },
  { key: "plan", label: "Plan", letter: "P", color: "text-purple-200 bg-purple-900/80 border-purple-800/50" },
];

export function SoapCard({ entry, onClick }: SoapCardProps) {
  return (
    <div
      role="button"
      tabIndex={0}
      onClick={onClick}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onClick?.();
        }
      }}
      className="p-4 rounded-lg bg-surface border border-border hover:border-border-strong transition-colors cursor-pointer"
    >
      {/* Header: badge SOAP + estado */}
      <div className="flex items-center justify-between gap-3 mb-2">
        <CategoryBadge category={entry.category} />
        <StatusBadge status={entry.status} />
      </div>

      {/* Título */}
      <h3 className="text-sm font-semibold text-text-primary mb-3">{entry.title}</h3>

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
                <span className="text-text-muted font-medium">{label}: </span>
                <span className="text-text-secondary wrap-break-words whitespace-pre-wrap">{value}</span>
              </div>
            </div>
          );
        })}
      </div>

      {/* Autor y firma */}
      <div className="flex flex-wrap gap-2 items-center justify-between text-xs mb-2 px-2 py-1.5 bg-canvas/50 rounded">
        <SignedBy name={entry.author_name} profession={entry.author_profession}/>
        <div className="flex items-center gap-1 shrink-0">
          <SignatureStatus isVerified={entry.is_verified} signature={entry.signature} />
        </div>
      </div>

      {/* Footer */}
      <EntryFooter timestamp={entry.timestamp} id={entry.id} />
    </div>
  );
}
