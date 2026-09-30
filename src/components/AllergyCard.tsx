import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Entry } from "../stores/useEntryStore";

interface AllergyCardProps {
  subjectId: number;
}

const SEVERITY_STYLES: Record<string, { label: string; chip: string; dot: string }> = {
  Leve: { label: "Leve", chip: "bg-yellow-900/30 text-yellow-400 border-yellow-800/50", dot: "bg-yellow-500" },
  Moderada: { label: "Moderada", chip: "bg-amber-900/30 text-amber-400 border-amber-800/50", dot: "bg-amber-500" },
  Severa: { label: "Severa", chip: "bg-red-900/40 text-red-400 border-red-800/50", dot: "bg-red-500" },
  Anafilaxia: { label: "Anafilaxia", chip: "bg-red-950/60 text-red-300 border-red-700/60", dot: "bg-red-600 animate-pulse" },
};

const DEFAULT_SEVERITY = { label: "Sin datos", chip: "bg-zinc-800 text-zinc-400 border-zinc-700", dot: "bg-zinc-500" };

function severityOf(entry: Entry) {
  return SEVERITY_STYLES[entry.payload.tipo_reaccion] ?? DEFAULT_SEVERITY;
}

export function AllergyCard({ subjectId }: AllergyCardProps) {
  const [allergies, setAllergies] = useState<Entry[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [isExpanded, setIsExpanded] = useState(false);

  useEffect(() => {
    let cancelled = false;

    const load = async () => {
      setIsLoading(true);
      setError(null);
      setIsExpanded(false);
      try {
        const res = await invoke<Entry[]>("get_entries_by_category", {
          subjectId,
          category: "ALLERGY",
          limit: 50,
          offset: 0,
        });
        if (!cancelled) setAllergies(res);
      } catch (err) {
        if (!cancelled) setError(String(err));
      } finally {
        if (!cancelled) setIsLoading(false);
      }
    };

    load();
    return () => {
      cancelled = true;
    };
  }, [subjectId]);

  const hasAllergies = allergies.length > 0;

  return (
    <div className="border-t border-zinc-800 bg-zinc-950/40">
      {/* Barra siempre visible */}
      <div
        className={`px-4 sm:px-5 py-3 flex items-center justify-between gap-3 ${
          hasAllergies ? "cursor-pointer hover:bg-red-950/20" : ""
        } transition-colors`}
        onClick={hasAllergies ? () => setIsExpanded((v) => !v) : undefined}
      >
        <div className="flex items-center gap-2.5 min-w-0 flex-wrap">
          <span className="text-sm shrink-0">⚠️</span>
          <span className="text-[10px] text-zinc-500 uppercase font-bold tracking-wider shrink-0">
            Alergias
          </span>

          {isLoading ? (
            <span className="text-xs text-zinc-500">Cargando…</span>
          ) : error ? (
            <span className="text-xs text-red-400 truncate">{error}</span>
          ) : !hasAllergies ? (
            <span className="inline-flex items-center gap-1.5 text-xs text-green-400">
              <span className="h-1.5 w-1.5 rounded-full bg-green-500"></span>
              Ninguna registrada
            </span>
          ) : (
            <>
              <span className="text-[10px] font-mono bg-red-900/40 text-red-300 border border-red-800/60 px-1.5 py-0.5 rounded">
                {allergies.length}
              </span>
              {/* Chips rápidos: sustancia + severidad */}
              {allergies.map((a) => {
                const sev = severityOf(a);
                return (
                  <span
                    key={a.id}
                    className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded border text-[11px] font-medium ${sev.chip}`}
                  >
                    <span className={`h-1.5 w-1.5 rounded-full ${sev.dot}`}></span>
                    {a.payload.sustancia || a.title}
                  </span>
                );
              })}
            </>
          )}
        </div>

        {hasAllergies && !isLoading && !error && (
          <svg
            className={`w-4 h-4 text-zinc-500 shrink-0 transition-transform duration-200 ${isExpanded ? "rotate-180" : ""}`}
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
          </svg>
        )}
      </div>

      {/* Detalle expandible */}
      {isExpanded && hasAllergies && (
        <div className="px-4 sm:px-5 pb-4 space-y-2 border-t border-zinc-800/60 pt-3">
          {allergies.map((a) => {
            const sev = severityOf(a);
            return (
              <div
                key={a.id}
                className="bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2.5"
              >
                <div className="flex items-center justify-between gap-2 mb-1">
                  <span className="text-sm font-semibold text-zinc-200">
                    {a.payload.sustancia || a.title}
                  </span>
                  <span className={`inline-flex items-center gap-1.5 px-1.5 py-0.5 rounded border text-[10px] font-medium shrink-0 ${sev.chip}`}>
                    <span className={`h-1.5 w-1.5 rounded-full ${sev.dot}`}></span>
                    {sev.label}
                  </span>
                </div>
                {a.payload.descripcion && (
                  <p className="text-xs text-zinc-400 whitespace-pre-wrap wrap-break-words leading-relaxed">
                    {a.payload.descripcion}
                  </p>
                )}
                <div className="flex items-center justify-between text-[10px] text-zinc-600 mt-1.5">
                  <span>
                    Firmado por <span className="text-zinc-500">{a.author_name}</span>
                    {a.is_verified ? " ✓" : " (sin verificar)"}
                  </span>
                  <span>ID: {a.id.slice(0, 8)}…</span>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
