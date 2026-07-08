// src/components/PatientEhrView.tsx
import { useState, useCallback, useEffect } from "react";
import { useEntityStore } from "../stores/useEntityStore";
import { useNoteStore } from "../stores/useNoteStore";
import { useAuthStore } from "../stores/useAuthStore";

export function PatientEhrView() {
  const { activeUser } = useAuthStore();
  const {
    entities,
    selectedEntity,
    isLoading: entityLoading,
    searchEntities,
    selectEntity,
    findByBlindIndex,
  } = useEntityStore();

  const {
    notes,
    selectedNote,
    isLoading: noteLoading,
    fetchNotes,
    createNote,
    selectNote,
  } = useNoteStore();

  const [searchQuery, setSearchQuery] = useState("");
  const [dniQuery, setDniQuery] = useState("");
  const [showNewNote, setShowNewNote] = useState(false);

  // Campos para nueva nota
  const [noteFields, setNoteFields] = useState({
    subjetivo: "",
    objetivo: "",
    evaluacion: "",
    plan: "",
  });

  // Debounce para búsqueda por texto
  useEffect(() => {
    const timer = setTimeout(() => {
      if (searchQuery.trim()) {
        searchEntities("patient", searchQuery);
      }
    }, 300);
    return () => clearTimeout(timer);
  }, [searchQuery, searchEntities]);

  const handleSelectPatient = useCallback(
    (entity: (typeof entities)[0]) => {
      selectEntity(entity);
      fetchNotes(entity.id);
      setShowNewNote(false);
      selectNote(null);
    },
    [selectEntity, fetchNotes, selectNote],
  );

  const handleFindByDni = async () => {
    if (!dniQuery.trim()) return;
    const id = await findByBlindIndex("patient", dniQuery);
    if (id) {
      const entity = entities.find((e) => e.id === id);
      if (entity) {
        handleSelectPatient(entity);
      }
    }
  };

  const handleCreateNote = async () => {
    if (!selectedEntity || !activeUser) return;
    const res = await createNote(
      selectedEntity.id,
      "soap_v1",
      noteFields,
      activeUser.user_id,
    );
    if (res) {
      setNoteFields({ subjetivo: "", objetivo: "", evaluacion: "", plan: "" });
      setShowNewNote(false);
      fetchNotes(selectedEntity.id);
    }
  };

  return (
    <div className="grid grid-cols-12 gap-6 h-[calc(100vh-200px)]">
      {/* PANEL IZQUIERDO: Búsqueda y listado */}
      <div className="col-span-4 flex flex-col border border-zinc-800 rounded-lg bg-zinc-900 overflow-hidden">
        {/* Búsqueda */}
        <div className="p-4 border-b border-zinc-800 space-y-3">
          <h3 className="text-sm font-semibold text-zinc-400 uppercase tracking-wider">
            Buscar Paciente
          </h3>

          {/* Búsqueda por DNI (exacto) */}
          <div className="flex gap-2">
            <input
              type="text"
              value={dniQuery}
              onChange={(e) => setDniQuery(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleFindByDni()}
              placeholder="DNI exacto..."
              className="flex-1 rounded bg-zinc-950 border border-zinc-700 px-3 py-2 text-sm text-zinc-200"
            />
            <button
              onClick={handleFindByDni}
              disabled={entityLoading}
              className="rounded bg-amber-600 px-3 py-2 text-sm text-white hover:bg-amber-700 disabled:opacity-50"
            >
              🔍
            </button>
          </div>

          {/* Búsqueda por nombre (parcial) */}
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Nombre, apellido, teléfono..."
            className="w-full rounded bg-zinc-950 border border-zinc-700 px-3 py-2 text-sm text-zinc-200"
          />
        </div>

        {/* Listado de resultados */}
        <div className="flex-1 overflow-y-auto p-2 space-y-1">
          {entityLoading ? (
            <div className="text-center py-8 text-zinc-500 text-sm">
              Buscando...
            </div>
          ) : entities.length === 0 ? (
            <div className="text-center py-8 text-zinc-600 text-sm">
              {searchQuery ? "Sin resultados" : "Escribí para buscar"}
            </div>
          ) : (
            entities.map((entity) => (
              <button
                key={entity.id}
                onClick={() => handleSelectPatient(entity)}
                className={`w-full text-left p-3 rounded transition-colors ${
                  selectedEntity?.id === entity.id
                    ? "bg-blue-950 border border-blue-800"
                    : "bg-zinc-950 border border-transparent hover:bg-zinc-800"
                }`}
              >
                <div className="font-medium text-zinc-200">
                  {entity.data.nombre} {entity.data.apellido}
                </div>
                <div className="text-xs text-zinc-500 mt-1">
                  DNI: {entity.data.dni} • {entity.data.telefono}
                </div>
                <div className="text-xs text-zinc-600 mt-0.5">
                  ID: {entity.id} • {entity.created_at.slice(0, 10)}
                </div>
              </button>
            ))
          )}
        </div>
      </div>

      {/* PANEL DERECHO: Ficha + Evoluciones */}
      <div className="col-span-8 flex flex-col gap-4 overflow-hidden">
        {selectedEntity ? (
          <>
            {/* FICHA DEL PACIENTE */}
            <div className="border border-zinc-800 rounded-lg bg-zinc-900 p-4">
              <div className="flex justify-between items-start">
                <div>
                  <h2 className="text-xl font-bold text-zinc-100">
                    {selectedEntity.data.nombre} {selectedEntity.data.apellido}
                  </h2>
                  <p className="text-sm text-zinc-500 mt-1">
                    DNI: {selectedEntity.data.dni} •{" "}
                    {selectedEntity.data.fecha_nacimiento}
                  </p>
                </div>
                <div className="text-right">
                  <div className="text-xs text-zinc-600 font-mono">
                    ID: {selectedEntity.id}
                  </div>
                  <div className="text-xs text-zinc-600 font-mono">
                    {selectedEntity.external_id?.slice(0, 8)}
                  </div>
                </div>
              </div>

              <div className="grid grid-cols-3 gap-4 mt-4">
                <div className="bg-zinc-950 rounded p-3 border border-zinc-800">
                  <div className="text-xs text-zinc-500 uppercase">
                    Teléfono
                  </div>
                  <div className="text-sm text-zinc-300 mt-1">
                    {selectedEntity.data.telefono || "—"}
                  </div>
                </div>
                <div className="bg-zinc-950 rounded p-3 border border-zinc-800">
                  <div className="text-xs text-zinc-500 uppercase">Email</div>
                  <div className="text-sm text-zinc-300 mt-1">
                    {selectedEntity.data.email || "—"}
                  </div>
                </div>
                <div className="bg-zinc-950 rounded p-3 border border-zinc-800">
                  <div className="text-xs text-zinc-500 uppercase">
                    Dirección
                  </div>
                  <div className="text-sm text-zinc-300 mt-1">
                    {selectedEntity.data.direccion || "—"}
                  </div>
                </div>
              </div>
            </div>

            {/* EVOLUCIONES */}
            <div className="flex-1 border border-zinc-800 rounded-lg bg-zinc-900 overflow-hidden flex flex-col">
              <div className="p-4 border-b border-zinc-800 flex justify-between items-center">
                <h3 className="text-sm font-semibold text-zinc-400 uppercase tracking-wider">
                  Evoluciones ({notes.length})
                </h3>
                <button
                  onClick={() => setShowNewNote(!showNewNote)}
                  className="rounded bg-blue-600 px-3 py-1.5 text-sm text-white hover:bg-blue-700"
                >
                  {showNewNote ? "Cancelar" : "+ Nueva Evolución"}
                </button>
              </div>

              {/* Formulario nueva nota */}
              {showNewNote && (
                <div className="p-4 border-b border-zinc-800 bg-zinc-950 space-y-3">
                  <div className="grid grid-cols-2 gap-3">
                    <div>
                      <label className="text-xs text-zinc-500">Subjetivo</label>
                      <textarea
                        value={noteFields.subjetivo}
                        onChange={(e) =>
                          setNoteFields((p) => ({
                            ...p,
                            subjetivo: e.target.value,
                          }))
                        }
                        rows={3}
                        className="w-full rounded bg-zinc-900 border border-zinc-700 px-3 py-2 text-sm text-zinc-200 resize-y"
                      />
                    </div>
                    <div>
                      <label className="text-xs text-zinc-500">Objetivo</label>
                      <textarea
                        value={noteFields.objetivo}
                        onChange={(e) =>
                          setNoteFields((p) => ({
                            ...p,
                            objetivo: e.target.value,
                          }))
                        }
                        rows={3}
                        className="w-full rounded bg-zinc-900 border border-zinc-700 px-3 py-2 text-sm text-zinc-200 resize-y"
                      />
                    </div>
                  </div>
                  <div className="grid grid-cols-2 gap-3">
                    <div>
                      <label className="text-xs text-zinc-500">
                        Evaluación
                      </label>
                      <textarea
                        value={noteFields.evaluacion}
                        onChange={(e) =>
                          setNoteFields((p) => ({
                            ...p,
                            evaluacion: e.target.value,
                          }))
                        }
                        rows={2}
                        className="w-full rounded bg-zinc-900 border border-zinc-700 px-3 py-2 text-sm text-zinc-200 resize-y"
                      />
                    </div>
                    <div>
                      <label className="text-xs text-zinc-500">Plan</label>
                      <textarea
                        value={noteFields.plan}
                        onChange={(e) =>
                          setNoteFields((p) => ({ ...p, plan: e.target.value }))
                        }
                        rows={2}
                        className="w-full rounded bg-zinc-900 border border-zinc-700 px-3 py-2 text-sm text-zinc-200 resize-y"
                      />
                    </div>
                  </div>
                  <button
                    onClick={handleCreateNote}
                    disabled={noteLoading}
                    className="rounded bg-emerald-600 px-4 py-2 text-sm text-white hover:bg-emerald-700 disabled:opacity-50"
                  >
                    {noteLoading ? "Guardando..." : "Guardar + Firmar"}
                  </button>
                </div>
              )}

              {/* Listado de notas */}
              <div className="flex-1 overflow-y-auto p-2 space-y-2">
                {noteLoading ? (
                  <div className="text-center py-8 text-zinc-500">
                    Cargando...
                  </div>
                ) : notes.length === 0 ? (
                  <div className="text-center py-8 text-zinc-600 text-sm">
                    Sin evoluciones registradas
                  </div>
                ) : (
                  notes.map((note) => (
                    <div
                      key={note.id}
                      onClick={() => selectNote(note)}
                      className={`p-3 rounded border cursor-pointer transition-colors ${
                        selectedNote?.id === note.id
                          ? "bg-blue-950 border-blue-800"
                          : "bg-zinc-950 border-zinc-800 hover:bg-zinc-800"
                      }`}
                    >
                      <div className="flex justify-between items-center">
                        <span className="text-sm font-medium text-zinc-300">
                          Nota #{note.id}
                        </span>
                        <span
                          className={`text-xs ${
                            note.is_verified
                              ? "text-emerald-500"
                              : "text-red-500"
                          }`}
                        >
                          {note.is_verified ? "✅ Firmada" : "❌ Sin firma"}
                        </span>
                      </div>
                      <div className="text-xs text-zinc-600 mt-1">
                        {note.created_at}
                      </div>
                      {selectedNote?.id === note.id && note.fields && (
                        <div className="mt-3 pt-3 border-t border-zinc-800 space-y-2">
                          {Object.entries(note.fields).map(([k, v]) => (
                            <div key={k}>
                              <span className="text-xs text-zinc-500 uppercase">
                                {k}
                              </span>
                              <p className="text-sm text-zinc-300 mt-0.5 whitespace-pre-wrap">
                                {v}
                              </p>
                            </div>
                          ))}
                        </div>
                      )}
                    </div>
                  ))
                )}
              </div>
            </div>
          </>
        ) : (
          <div className="flex-1 flex items-center justify-center border border-zinc-800 rounded-lg bg-zinc-900">
            <div className="text-center text-zinc-600">
              <div className="text-4xl mb-4">👤</div>
              <p>Seleccioná un paciente para ver su ficha</p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
