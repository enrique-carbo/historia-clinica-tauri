import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface PatientSelectorProps {
  onSelectPatient: (patient: { id: string; full_name: string }) => void;
  selectedPatientId: string | null;
}

export function PatientSelector({
  onSelectPatient,
  selectedPatientId,
}: PatientSelectorProps) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<any[]>([]);

  useEffect(() => {
    const delayDebounce = setTimeout(async () => {
      try {
        const list = await invoke<any[]>("search_patients", {
          queryName: query,
        });
        setResults(list);
      } catch (err) {
        console.error("Error al buscar pacientes:", err);
      }
    }, 150);

    return () => clearTimeout(delayDebounce);
  }, [query]);

  return (
    <div className="bg-zinc-900 p-5 rounded-lg border border-zinc-800 mb-5">
      <h3 className="text-sm font-semibold text-zinc-200 mb-2.5">
        🔍 Selector Clínico de Pacientes
      </h3>

      <input
        type="text"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Escribí el nombre del paciente a evolucionar..."
        className="w-full bg-zinc-950 text-zinc-200 border border-zinc-800 rounded px-3 py-2.5 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 placeholder:text-zinc-700 mb-3"
      />

      <div className="max-h-44 overflow-y-auto flex flex-col gap-2">
        {results.length === 0 ? (
          <div className="text-xs text-zinc-600 p-1">
            No se encontraron pacientes descifrados.
          </div>
        ) : (
          results.map((p) => {
            const isSelected = p.id === selectedPatientId;
            return (
              <div
                key={p.id}
                onClick={() => onSelectPatient(p)}
                className={`p-2.5 rounded border cursor-pointer transition-all duration-200 ease-in-out
                  ${
                    isSelected
                      ? "bg-blue-900 border-blue-500"
                      : "bg-zinc-950 border-zinc-800 hover:bg-zinc-900 hover:border-zinc-700"
                  }`}
              >
                <div
                  className={`text-sm font-semibold ${isSelected ? "text-white" : "text-zinc-200"}`}
                >
                  {p.full_name}
                </div>
                <div
                  className={`text-[11px] font-mono mt-0.5 ${isSelected ? "text-blue-300" : "text-zinc-600"}`}
                >
                  ID: {p.id}
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
