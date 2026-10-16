import { useEffect, useState } from "react";
import { useEntryStore, EntryCategory } from "../stores/useEntryStore";
import { EntryCard } from "./EntryCard";
import { Button } from "./ui/Button";
import { Alert } from "./ui/Alert";
import { EmptyState } from "./ui/EmptyState";

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
    isLoadingMore,
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
      <div className="flex flex-col md:flex-row gap-3 mb-4">
        {/* Filtros por categoría */}
        <div className="flex flex-wrap gap-2 flex-1">
          {CATEGORIES.map((cat) => (
            <button
              key={cat.value}
              onClick={() => handleFilterChange(cat.value)}
              className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-colors flex items-center gap-1.5 ${
                filter === cat.value
                  ? "bg-selected-tint/40 border border-selected-border/60 text-selected-text"
                  : "bg-surface border border-border text-text-secondary hover:border-border-strong hover:text-text-primary"
              }`}
            >
              <span>{cat.icon}</span>
              {cat.label}
            </button>
          ))}
        </div>

        {/* Búsqueda por título */}
        <div className="relative w-full sm:w-64 self-start shrink-0">
          <svg
            className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 z-10 w-4 h-4 text-text-muted"
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
            className="relative w-full pl-9 pr-8 py-1.5 bg-surface border border-border rounded-lg text-sm text-text-primary placeholder:text-text-placeholder focus:outline-none focus:border-focus"
          />
          {searchQuery && (
            <button
              onClick={handleClearSearch}
              className="absolute right-2 top-1/2 -translate-y-1/2 z-10 text-text-muted hover:text-text-primary"
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
        <Alert variant="error" className="text-sm">
          {error}
        </Alert>
      ) : entries.length === 0 && !isLoading ? (
        <EmptyState
          title={
            debouncedQuery
              ? `No se encontraron entries para "${debouncedQuery}"`
              : filter === "ALL"
                ? "No hay entries registradas"
                : `No hay entries de tipo ${CATEGORIES.find((c) => c.value === filter)?.label}`
          }
          hint={
            debouncedQuery
              ? "Probá con otro término de búsqueda"
              : "Las entries son inmutables y se crean al registrar datos clínicos"
          }
        />
      ) : (
        <>
          {/* Contador */}
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-sm font-semibold text-text-primary">
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
            <div className="absolute left-4 top-0 bottom-0 w-px bg-surface-raised" />

            {/* Entries */}
            <div className="space-y-4">
              {entries.map((entry) => (
                <div key={entry.id} className="relative pl-10">
                  {/* Punto en la línea de tiempo */}
                  <div className="absolute left-2.5 top-4 w-3 h-3 rounded-full bg-border-strong border-2 border-text-muted" />

                  <EntryCard entry={entry} onClick={() => selectEntry(entry)} />
                </div>
              ))}
            </div>
          </div>

          {/* Load more */}
          <div className="flex justify-center mt-6">
            {isLoading || isLoadingMore ? (
              <div className="flex items-center gap-2 text-text-muted text-sm">
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
              <Button variant="secondary" onClick={handleLoadMore}>
                Cargar más ({entries.length} cargadas)
              </Button>
            ) : (
              <span className="text-xs text-text-muted">
                Fin de la lista
              </span>
            )}
          </div>
        </>
      )}
    </div>
  );
}
