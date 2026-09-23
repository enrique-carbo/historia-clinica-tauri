import { useEffect, useState } from "react";
import { useEntryStore, EntryCategory } from "../stores/useEntryStore";
import { EntryCard } from "./EntryCard";

interface EntryTimelineProps {
  subjectId: number;
}

const CATEGORIES: { value: EntryCategory | "ALL"; label: string; icon: string }[] = [
  { value: "ALL", label: "Todas", icon: "📊" },
  { value: "SOAP_NOTE", label: "SOAP", icon: "📋" },
  { value: "ALLERGY", label: "Alergias", icon: "⚠️" },
  { value: "MEDICATION", label: "Medicación", icon: "💊" },
  { value: "CONDITION", label: "Diagnósticos", icon: "🏥" },
];

export function EntryTimeline({ subjectId }: EntryTimelineProps) {
  const {
    entries,
    isLoading,
    error,
    hasMore,
    fetchEntries,
    searchEntries,
    loadMore,
    selectEntry,
    clearError,
  } = useEntryStore();
  const [filter, setFilter] = useState<EntryCategory | "ALL">("ALL");
  const [searchQuery, setSearchQuery] = useState("");
  const [debouncedQuery, setDebouncedQuery] = useState("");

  // Debounce para la búsqueda
  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedQuery(searchQuery.trim());
    }, 300);
    return () => clearTimeout(timer);
  }, [searchQuery]);

  // Cargar entries cuando cambia filtro o búsqueda
  useEffect(() => {
    clearError();
    const category = filter === "ALL" ? undefined : filter;
    if (debouncedQuery) {
      searchEntries(subjectId, debouncedQuery, category);
    } else {
      fetchEntries(subjectId, category);
    }
  }, [subjectId, filter, debouncedQuery, fetchEntries, searchEntries, clearError]);

  const handleFilterChange = (newFilter: EntryCategory | "ALL") => {
    setFilter(newFilter);
  };

  const handleLoadMore = () => {
    loadMore(subjectId, filter === "ALL" ? undefined : filter, debouncedQuery);
  };

  const handleClearSearch = () => {
    setSearchQuery("");
    setDebouncedQuery("");
  };

  return (
    <div className="space-y-3">
      {/* Barra de herramientas: filtros + búsqueda */}
      <div className="flex flex-col sm:flex-row gap-3 mb-4">
        {/* Filtros por categoría */}
        <div className="flex flex-wrap gap-2 flex-1">
          {CATEGORIES.map((cat) => (
            <button
              key={cat.value}
              onClick={() => handleFilterChange(cat.value)}
              className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-colors flex items-center gap-1.5 ${
                filter === cat.value
                  ? "bg-blue-900/40 border border-blue-700 text-blue-300"
                  : "bg-zinc-900 border border-zinc-800 text-zinc-400 hover:border-zinc-700 hover:text-zinc-300"
              }`}
            >
              <span>{cat.icon}</span>
              {cat.label}
            </button>
          ))}
        </div>

        {/* Búsqueda por título */}
        <div className="relative w-full sm:w-64">
          <svg
            className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-zinc-500"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
            />
          </svg>
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Buscar por título..."
            className="w-full pl-9 pr-8 py-1.5 bg-zinc-900 border border-zinc-800 rounded-lg text-sm text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-blue-600"
          />
          {searchQuery && (
            <button
              onClick={handleClearSearch}
              className="absolute right-2 top-1/2 -translate-y-1/2 text-zinc-500 hover:text-zinc-300"
              aria-label="Limpiar búsqueda"
            >
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          )}
        </div>
      </div>

      {/* Contenido */}
      {error ? (
        <div className="p-4 rounded-lg bg-red-950/30 border border-red-800/50 text-red-400 text-sm">
          {error}
        </div>
      ) : entries.length === 0 && !isLoading ? (
        <div className="flex flex-col items-center justify-center py-12 text-zinc-500">
          <svg
            className="w-12 h-12 mb-3 text-zinc-700"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={1.5}
              d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
            />
          </svg>
          <p className="text-sm">
            {debouncedQuery
              ? `No se encontraron entries para "${debouncedQuery}"`
              : filter === "ALL"
                ? "No hay entries registradas"
                : `No hay entries de tipo ${CATEGORIES.find((c) => c.value === filter)?.label}`}
          </p>
          <p className="text-xs text-zinc-600 mt-1">
            {debouncedQuery
              ? "Probá con otro término de búsqueda"
              : "Las entries son inmutables y se crean al registrar datos clínicos"}
          </p>
        </div>
      ) : (
        <>
          {/* Contador */}
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-sm font-semibold text-zinc-300">
              {debouncedQuery
                ? `Resultados para "${debouncedQuery}" (${entries.length})`
                : filter === "ALL"
                  ? `Todas las entries (${entries.length})`
                  : `${CATEGORIES.find((c) => c.value === filter)?.label} (${entries.length})`}
            </h3>
          </div>

          {/* Timeline */}
          <div className="relative">
            {/* Línea vertical conectora */}
            <div className="absolute left-4 top-0 bottom-0 w-px bg-zinc-800" />

            {/* Entries */}
            <div className="space-y-4">
              {entries.map((entry) => (
                <div key={entry.id} className="relative pl-10">
                  {/* Punto en la línea de tiempo */}
                  <div className="absolute left-2.5 top-4 w-3 h-3 rounded-full bg-zinc-700 border-2 border-zinc-600" />

                  <EntryCard entry={entry} onClick={() => selectEntry(entry)} />
                </div>
              ))}
            </div>
          </div>

          {/* Load more */}
          <div className="flex justify-center mt-6">
            {isLoading ? (
              <div className="flex items-center gap-2 text-zinc-500 text-sm">
                <svg
                  className="animate-spin h-4 w-4"
                  xmlns="http://www.w3.org/2000/svg"
                  fill="none"
                  viewBox="0 0 24 24"
                >
                  <circle
                    className="opacity-25"
                    cx="12"
                    cy="12"
                    r="10"
                    stroke="currentColor"
                    strokeWidth="4"
                  />
                  <path
                    className="opacity-75"
                    fill="currentColor"
                    d="M4 12a8 8 0 018-8v4a4 4 0 00-4 4H4z"
                  />
                </svg>
                Cargando...
              </div>
            ) : hasMore ? (
              <button
                onClick={handleLoadMore}
                className="px-4 py-2 bg-zinc-900 border border-zinc-800 text-zinc-400 rounded-lg text-sm hover:bg-zinc-800 hover:text-zinc-300 transition-colors"
              >
                Cargar más ({entries.length} cargadas)
              </button>
            ) : (
              <span className="text-xs text-zinc-600">
                Fin de la lista
              </span>
            )}
          </div>
        </>
      )}
    </div>
  );
}
