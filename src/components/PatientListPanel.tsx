// Panel izquierdo del padrón: búsqueda por nombre/teléfono, búsqueda exacta
// por el documento (blind index) y listado de pacientes. Presentacional:
// recibe estado y callbacks.
import { EmptyState } from "./ui/EmptyState";
import { BLIND_INDEX_LABEL } from "../utils/schema";

export interface EntityRecord {
  id: number;
  external_id?: string;
  entity_type: string;
  data: Record<string, string>;
  created_at: string;
}

interface PatientListPanelProps {
  entities: EntityRecord[];
  loading: boolean;
  searchQuery: string;
  searchDni: string;
  selectedId: number | null;
  onSearchQueryChange: (value: string) => void;
  onSearchDniChange: (value: string) => void;
  onSearch: () => void;
  onFind: () => void;
  onSelect: (id: number) => void;
  onEdit: (id: number) => void;
}

const INPUT_CLASSES =
  "w-full min-w-0 rounded-lg bg-field border border-border-strong px-3.5 py-2 text-sm text-text-primary focus:border-focus focus:ring-1 focus:ring-focus transition-colors outline-none";
const SEARCH_BUTTON_CLASSES =
  "shrink-0 rounded-lg bg-action-primary px-3.5 py-2 text-sm font-medium text-text-inverse hover:bg-action-primary-hover disabled:opacity-50 transition-colors";

export function PatientListPanel({
  entities,
  loading,
  searchQuery,
  searchDni,
  selectedId,
  onSearchQueryChange,
  onSearchDniChange,
  onSearch,
  onFind,
  onSelect,
  onEdit,
}: PatientListPanelProps) {
  return (
    <div className="col-span-1 md:col-span-5 lg:col-span-4 flex flex-col border border-border rounded-xl bg-surface/50 backdrop-blur-sm overflow-hidden shadow-lg min-h-100 md:min-h-0">
      <div className="p-4 border-b border-border space-y-3 bg-surface">
        <h3 className="text-xs font-bold text-text-muted uppercase tracking-widest">
          Padrón de Pacientes
        </h3>

        {/* Búsqueda por Nombre / Teléfono */}
        <div className="flex gap-2">
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => onSearchQueryChange(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && onSearch()}
            placeholder="Buscar por nombre, teléfono..."
            className={INPUT_CLASSES}
          />
          <button
            onClick={onSearch}
            disabled={loading}
            aria-label="Buscar por nombre o teléfono"
            className={SEARCH_BUTTON_CLASSES}
          >
            <span aria-hidden="true">🔍</span>
          </button>
        </div>

        {/* Búsqueda exacta por el documento (blind index) */}
        <div className="flex gap-2">
          <input
            type="text"
            value={searchDni}
            onChange={(e) => onSearchDniChange(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && onFind()}
            placeholder={`${BLIND_INDEX_LABEL} exacto...`}
            className={`${INPUT_CLASSES} flex-1`}
          />
          <button
            onClick={onFind}
            disabled={loading}
            aria-label={`Buscar por ${BLIND_INDEX_LABEL}`}
            className={SEARCH_BUTTON_CLASSES}
          >
            <span aria-hidden="true">🔍</span>
          </button>
        </div>
      </div>

      {/* Lista de Pacientes */}
      <div className="flex-1 overflow-y-auto p-2 space-y-1 custom-scrollbar max-h-[50vh] md:max-h-none">
        {loading ? (
          <div className="flex justify-center py-8">
            <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-focus"></div>
          </div>
        ) : entities.length === 0 ? (
          <EmptyState
            icon={null}
            className="px-4"
            title={
              searchQuery || searchDni
                ? "No se encontraron coincidencias"
                : "Busca un paciente o crea uno nuevo"
            }
          />
        ) : (
          entities.map((e) => (
            <div
              key={e.id}
              className={`w-full text-left p-3 rounded-lg transition-colors duration-200 group flex items-center justify-between gap-2 ${
                selectedId === e.id
                  ? "bg-selected-tint/20 border border-selected-border/50"
                  : "bg-canvas/50 border border-transparent hover:bg-surface-raised hover:border-border-strong"
              }`}
            >
              <button
                type="button"
                className="flex-1 min-w-0 text-left cursor-pointer"
                onClick={() => onSelect(e.id)}
              >
                <div className="flex justify-between items-start gap-2">
                  <div
                    className={`font-semibold transition-colors truncate ${
                      selectedId === e.id
                        ? "text-selected-text"
                        : "text-text-primary"
                    }`}
                  >
                    {e.data.nombre} {e.data.apellido}
                  </div>
                  {selectedId === e.id && (
                    <span className="h-2 w-2 shrink-0 rounded-full bg-selected-border mt-1.5"></span>
                  )}
                </div>
                <div className="text-xs text-text-muted mt-1.5 flex flex-wrap items-center gap-2">
                  <span className="bg-surface-raised px-1.5 py-0.5 rounded text-text-secondary font-mono">
                    {BLIND_INDEX_LABEL}: {e.data.dni}
                  </span>
                </div>
              </button>

              {/* En móviles siempre visible, en desktop sólo en hover */}
              <button
                onClick={() => onEdit(e.id)}
                aria-label="Editar paciente"
                className="shrink-0 px-2.5 py-1.5 rounded-lg bg-surface-raised text-text-secondary hover:text-text-inverse hover:bg-action-primary transition-[color,background-color,border-color,opacity] text-xs font-bold border border-border-strong hover:border-action-primary opacity-100 md:opacity-0 md:group-hover:opacity-100"
                title="Editar paciente"
              >
                <span aria-hidden="true">✏️</span>
              </button>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
