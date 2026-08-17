// src/components/PatientEhrView.tsx
import { useState, useCallback, useEffect } from "react";
import { useEntityStore } from "../stores/useEntityStore";
import { useNoteStore } from "../stores/useNoteStore";
import { useAuthStore } from "../stores/useAuthStore";
import { calculateAge } from "../utils/dateUtils";

export function PatientEhrView() {
  const { activeUser } = useAuthStore();
  const {
    entities,
    selectedEntity,
    isLoading: entityLoading,
    searchEntities,
    selectEntity,
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
  const [showNewNote, setShowNewNote] = useState(false);
  const [isPatientDetailsOpen, setIsPatientDetailsOpen] = useState(false);

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
      // No necesitamos fetchNotes aquí gracias al Optimistic UI del store
    }
  };

  return (
    <div className="grid grid-cols-12 gap-6 h-[calc(100vh-180px)]">
      {/* PANEL IZQUIERDO: Búsqueda y Listado */}
      <div className="col-span-4 flex flex-col border border-zinc-800 rounded-xl bg-zinc-900/50 backdrop-blur-sm overflow-hidden shadow-lg">
        <div className="p-4 border-b border-zinc-800 space-y-3 bg-zinc-900">
          <h3 className="text-xs font-bold text-zinc-500 uppercase tracking-widest">
            Padrón de Pacientes
          </h3>
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Buscar por nombre, DNI o teléfono..."
            className="w-full rounded-lg bg-zinc-950 border border-zinc-700 px-4 py-2.5 text-sm text-zinc-200 focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-all outline-none"
          />
        </div>

        <div className="flex-1 overflow-y-auto p-2 space-y-1 custom-scrollbar">
          {entityLoading ? (
            <div className="flex justify-center py-8">
              <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-500"></div>
            </div>
          ) : entities.length === 0 ? (
            <div className="text-center py-12 text-zinc-600 text-sm">
              {searchQuery
                ? "No se encontraron coincidencias"
                : "Comienza escribiendo para buscar"}
            </div>
          ) : (
            entities.map((entity) => (
              <button
                key={entity.id}
                onClick={() => handleSelectPatient(entity)}
                className={`w-full text-left p-3 rounded-lg transition-all duration-200 group ${
                  selectedEntity?.id === entity.id
                    ? "bg-blue-900/20 border border-blue-500/50 shadow-md"
                    : "bg-zinc-950/50 border border-transparent hover:bg-zinc-800 hover:border-zinc-700"
                }`}
              >
                <div className="flex justify-between items-start">
                  <div className="font-semibold text-zinc-200 group-hover:text-white">
                    {entity.data.nombre} {entity.data.apellido}
                  </div>
                  {selectedEntity?.id === entity.id && (
                    <span className="h-2 w-2 rounded-full bg-blue-500 mt-1.5"></span>
                  )}
                </div>
                <div className="text-xs text-zinc-500 mt-1.5 flex items-center gap-2">
                  <span className="bg-zinc-800 px-1.5 py-0.5 rounded text-zinc-400">
                    DNI: {entity.data.dni}
                  </span>
                  <span>{entity.data.telefono}</span>
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
            {/* FICHA DEL PACIENTE COLAPSABLE */}
            <div className="border border-zinc-800 rounded-xl bg-zinc-900/50 shadow-lg overflow-hidden transition-all duration-300">
              {/* CABECERA COMPACTA (Siempre visible) */}
              <div
                className="p-5 cursor-pointer hover:bg-zinc-800/30 transition-colors flex justify-between items-center"
                onClick={() => setIsPatientDetailsOpen(!isPatientDetailsOpen)}
              >
                <div className="flex items-center gap-4">
                  {/* Icono del Paciente */}
                  <div className="p-3 rounded-full bg-blue-900/20 text-blue-400">
                    <svg xmlns="http://www.w3.org/2000/svg" className="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
                    </svg>
                  </div>

                  <div>
                    <div className="flex items-center gap-3">
                      <h2 className="text-xl font-bold text-zinc-100 tracking-tight">
                        {selectedEntity.data.nombre} {selectedEntity.data.apellido}
                      </h2>

                      {/* ALERTA DE ALERGIA PULSANTE */}
                      {selectedEntity.data.alergias && selectedEntity.data.alergias.trim() !== "" && (
                        <div className="group relative flex items-center justify-center">
                          <span className="absolute inline-flex h-3 w-3 rounded-full bg-red-500 opacity-75 animate-ping"></span>
                          <span className="relative inline-flex h-3 w-3 rounded-full bg-red-600"></span>
                          {/* Tooltip simple al pasar el mouse */}
                          <div className="absolute top-full mt-2 hidden group-hover:block w-48 p-2 bg-zinc-900 border border-red-900/50 rounded text-xs text-red-200 z-50 shadow-xl">
                            ⚠️ {selectedEntity.data.alergias}
                          </div>
                        </div>
                      )}
                    </div>

                    <div className="flex items-center gap-3 mt-0.5 text-sm text-zinc-400">
                      <span className="font-mono bg-zinc-800 px-1.5 py-0.5 rounded text-xs">DNI: {selectedEntity.data.dni}</span>
                      <span>{calculateAge(selectedEntity.data.fecha_nacimiento)} años</span>
                    </div>
                  </div>
                </div>

                {/* Flecha y ID... */}
                <div className="flex items-center gap-4">
                   <div className="text-right hidden sm:block">
                    <div className="text-xs font-mono text-zinc-600">ID: {selectedEntity.id}</div>
                  </div>
                  <div className={`transform transition-transform duration-200 ${isPatientDetailsOpen ? 'rotate-180' : ''}`}>
                    <svg xmlns="http://www.w3.org/2000/svg" className="h-5 w-5 text-zinc-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                    </svg>
                  </div>
                </div>
              </div>

              {/* CUERPO EXPANDIDO (Detalles completos) */}
              {isPatientDetailsOpen && (
                <div className="p-5 pt-4 border-t border-zinc-800 bg-zinc-950/30 animate-in slide-in-from-top-2 duration-200">
                  {/* Datos de Contacto */}
                  <div className="grid grid-cols-3 gap-3 mb-4">
                    {[
                      {
                        label: "Teléfono",
                        value: selectedEntity.data.telefono,
                      },
                      { label: "Email", value: selectedEntity.data.email },
                      {
                        label: "Dirección",
                        value: selectedEntity.data.direccion,
                      },
                    ].map((item, idx) => (
                      <div
                        key={idx}
                        className="bg-zinc-900/50 rounded-lg p-3 border border-zinc-800/50"
                      >
                        <div className="text-[10px] text-zinc-500 uppercase font-bold tracking-wider">
                          {item.label}
                        </div>
                        <div className="text-sm text-zinc-300 mt-0.5 truncate">
                          {item.value || "—"}
                        </div>
                      </div>
                    ))}
                  </div>

                  {/* Antecedentes Clínicos */}
                  <div className="grid grid-cols-2 gap-3">
                    {[
                      {
                        label: "Antecedentes Personales",
                        value: selectedEntity.data.antecedentes_personales,
                        color: "text-blue-500",
                      },
                      {
                        label: "Antecedentes Familiares",
                        value: selectedEntity.data.antecedentes_familiares,
                        color: "text-blue-500",
                      },
                      {
                        label: "Medicación habitual",
                        value: selectedEntity.data.medicacion,
                        color: "text-orange-500",
                      },
                      {
                        label: "Alergias",
                        value: selectedEntity.data.alergias,
                        color: "text-red-500",
                      },
                    ].map((item, idx) => (
                      <div
                        key={idx}
                        className={`bg-zinc-900/50 rounded-lg p-4 border border-zinc-800/50 min-h-25 ${item.label === "Alergias" && item.value ? "border-red-900/50 bg-red-900/5" : ""}`}
                      >
                        <div
                          className={`text-[10px] uppercase font-bold tracking-wider mb-2 flex items-center gap-2 ${item.color}`}
                        >
                          {item.label === "Alergias" && item.value && (
                            <span className="w-2 h-2 rounded-full bg-red-500 animate-pulse"></span>
                          )}
                          {item.label}
                        </div>
                        <p className="text-sm text-zinc-300 whitespace-pre-wrap leading-relaxed">
                          {item.value || "Sin registros"}
                        </p>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>

            {/* SECCIÓN DE EVOLUCIONES */}
            <div className="flex-1 border border-zinc-800 rounded-xl bg-zinc-900/50 overflow-hidden flex flex-col shadow-lg">
              <div className="p-4 border-b border-zinc-800 flex justify-between items-center bg-zinc-900">
                <h3 className="text-xs font-bold text-zinc-500 uppercase tracking-widest">
                  Historial Clínico ({notes.length})
                </h3>
                <button
                  onClick={() => setShowNewNote(!showNewNote)}
                  className={`rounded-lg px-4 py-2 text-sm font-medium transition-colors ${
                    showNewNote
                      ? "bg-zinc-800 text-zinc-300 hover:bg-zinc-700"
                      : "bg-blue-600 text-white hover:bg-blue-700 shadow-md shadow-blue-900/20"
                  }`}
                >
                  {showNewNote ? "Cancelar Nota" : "+ Nueva Nota"}
                </button>
              </div>

              {/* Formulario Nueva Nota (SOAP) */}
              {showNewNote && (
                <div className="p-4 border-b border-zinc-800 bg-zinc-950/80 space-y-4 animate-in slide-in-from-top-2 duration-200">
                  <div className="grid grid-cols-2 gap-4">
                    <div className="space-y-1">
                      <label className="text-xs font-bold text-blue-400 uppercase">
                        Subjetivo (S)
                      </label>
                      <textarea
                        value={noteFields.subjetivo}
                        onChange={(e) =>
                          setNoteFields((p) => ({
                            ...p,
                            subjetivo: e.target.value,
                          }))
                        }
                        rows={4}
                        className="w-full rounded-lg bg-zinc-900 border border-zinc-700 px-3 py-2 text-sm text-zinc-200 focus:border-blue-500 outline-none resize-none"
                        placeholder="Motivo de consulta y síntomas..."
                      />
                    </div>
                    <div className="space-y-1">
                      <label className="text-xs font-bold text-emerald-400 uppercase">
                        Objetivo (O)
                      </label>
                      <textarea
                        value={noteFields.objetivo}
                        onChange={(e) =>
                          setNoteFields((p) => ({
                            ...p,
                            objetivo: e.target.value,
                          }))
                        }
                        rows={4}
                        className="w-full rounded-lg bg-zinc-900 border border-zinc-700 px-3 py-2 text-sm text-zinc-200 focus:border-emerald-500 outline-none resize-none"
                        placeholder="Signos vitales y examen físico..."
                      />
                    </div>
                  </div>
                  <div className="grid grid-cols-2 gap-4">
                    <div className="space-y-1">
                      <label className="text-xs font-bold text-amber-400 uppercase">
                        Evaluación (A)
                      </label>
                      <textarea
                        value={noteFields.evaluacion}
                        onChange={(e) =>
                          setNoteFields((p) => ({
                            ...p,
                            evaluacion: e.target.value,
                          }))
                        }
                        rows={3}
                        className="w-full rounded-lg bg-zinc-900 border border-zinc-700 px-3 py-2 text-sm text-zinc-200 focus:border-amber-500 outline-none resize-none"
                        placeholder="Diagnóstico presuntivo..."
                      />
                    </div>
                    <div className="space-y-1">
                      <label className="text-xs font-bold text-purple-400 uppercase">
                        Plan (P)
                      </label>
                      <textarea
                        value={noteFields.plan}
                        onChange={(e) =>
                          setNoteFields((p) => ({ ...p, plan: e.target.value }))
                        }
                        rows={3}
                        className="w-full rounded-lg bg-zinc-900 border border-zinc-700 px-3 py-2 text-sm text-zinc-200 focus:border-purple-500 outline-none resize-none"
                        placeholder="Tratamiento y estudios..."
                      />
                    </div>
                  </div>

                  {/* BOTÓN DE GUARDAR Y FIRMAR - AHORA SÍ VISIBLE */}
                  <div className="flex justify-end pt-2">
                    <button
                      onClick={handleCreateNote}
                      disabled={noteLoading || !activeUser}
                      className="rounded-lg bg-emerald-600 px-6 py-2.5 text-sm font-bold text-white hover:bg-emerald-700 disabled:opacity-50 disabled:cursor-not-allowed shadow-lg shadow-emerald-900/20 transition-all flex items-center gap-2"
                    >
                      {noteLoading ? (
                        <>
                          <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
                          Firmando...
                        </>
                      ) : (
                        <>Guardar + Firmar</>
                      )}
                    </button>
                  </div>
                </div>
              )}

              {/* Listado de Notas */}
              <div className="flex-1 overflow-y-auto p-3 space-y-3 custom-scrollbar">
                {noteLoading && !showNewNote ? (
                  <div className="flex justify-center py-12">
                    <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
                  </div>
                ) : notes.length === 0 ? (
                  <div className="flex flex-col items-center justify-center h-full text-zinc-600">
                    <span className="text-4xl mb-2 opacity-20">📝</span>
                    <p>No hay evoluciones registradas</p>
                  </div>
                ) : (
                  notes.map((note) => (
                    <div
                      key={note.id}
                      onClick={() =>
                        selectNote(note.id === selectedNote?.id ? null : note)
                      }
                      className={`p-4 rounded-xl border cursor-pointer transition-all duration-200 ${
                        selectedNote?.id === note.id
                          ? "bg-blue-900/10 border-blue-500/30 shadow-md"
                          : "bg-zinc-950/50 border-zinc-800 hover:bg-zinc-800/50 hover:border-zinc-700"
                      }`}
                    >
                      {/* Cabecera de la Nota */}
                      <div className="flex justify-between items-start mb-2">
                        <div className="flex items-center gap-2">
                          <span className="text-sm font-bold text-zinc-200">
                            Nota #{note.id}
                          </span>
                          <span
                            className={`text-[10px] px-1.5 py-0.5 rounded font-bold uppercase ${
                              note.is_verified
                                ? "bg-emerald-500/10 text-emerald-500 border border-emerald-500/20"
                                : "bg-red-500/10 text-red-500 border border-red-500/20"
                            }`}
                          >
                            {note.is_verified ? "Firmada" : "Sin firma"}
                          </span>
                        </div>
                        <span className="text-xs text-zinc-500 font-mono bg-zinc-900 px-2 py-1 rounded">
                          {new Date(note.created_at).toLocaleString([], {
                            dateStyle: "short",
                            timeStyle: "short",
                            hour12: false,
                          })}
                        </span>
                      </div>

                      <div className="text-xs text-zinc-400 mb-2 flex items-center gap-2">
                        <span className="w-1.5 h-1.5 rounded-full bg-zinc-600"></span>
                        Por:{" "}
                        <span className="text-zinc-300 font-medium">
                          {note.created_by_name}
                        </span>
                      </div>

                      {/* Contenido Expandido (Ordenado S-O-A-P) */}
                      {selectedNote?.id === note.id && note.fields && (
                        <div className="mt-4 pt-4 border-t border-zinc-800 grid grid-cols gap-x-6 gap-y-4 animate-in fade-in slide-in-from-top-1 duration-200">
                          {/* 1. Subjetivo */}
                          <div className="col-span-2 sm:col-span-1 space-y-1">
                            <div className="flex items-center gap-2">
                              <span className="text-[10px] font-bold uppercase tracking-wider text-blue-400 bg-blue-900/20 px-1.5 py-0.5 rounded">
                                S
                              </span>
                              <span className="text-xs font-semibold text-zinc-400">
                                Subjetivo
                              </span>
                            </div>
                            <p className="text-sm text-zinc-300 whitespace-pre-wrap leading-relaxed pl-1 border-l-2 border-blue-900/50 min-h-5">
                              {note.fields.subjetivo || "—"}
                            </p>
                          </div>

                          {/* 2. Objetivo */}
                          <div className="col-span-2 sm:col-span-1 space-y-1">
                            <div className="flex items-center gap-2">
                              <span className="text-[10px] font-bold uppercase tracking-wider text-emerald-400 bg-emerald-900/20 px-1.5 py-0.5 rounded">
                                O
                              </span>
                              <span className="text-xs font-semibold text-zinc-400">
                                Objetivo
                              </span>
                            </div>
                            <p className="text-sm text-zinc-300 whitespace-pre-wrap leading-relaxed pl-1 border-l-2 border-emerald-900/50 min-h-5">
                              {note.fields.objetivo || "—"}
                            </p>
                          </div>

                          {/* 3. Evaluación */}
                          <div className="col-span-2 sm:col-span-1 space-y-1">
                            <div className="flex items-center gap-2">
                              <span className="text-[10px] font-bold uppercase tracking-wider text-amber-400 bg-amber-900/20 px-1.5 py-0.5 rounded">
                                A
                              </span>
                              <span className="text-xs font-semibold text-zinc-400">
                                Evaluación
                              </span>
                            </div>
                            <p className="text-sm text-zinc-300 whitespace-pre-wrap leading-relaxed pl-1 border-l-2 border-amber-900/50 min-h-5">
                              {note.fields.evaluacion || "—"}
                            </p>
                          </div>

                          {/* 4. Plan */}
                          <div className="col-span-2 sm:col-span-1 space-y-1">
                            <div className="flex items-center gap-2">
                              <span className="text-[10px] font-bold uppercase tracking-wider text-purple-400 bg-purple-900/20 px-1.5 py-0.5 rounded">
                                P
                              </span>
                              <span className="text-xs font-semibold text-zinc-400">
                                Plan
                              </span>
                            </div>
                            <p className="text-sm text-zinc-300 whitespace-pre-wrap leading-relaxed pl-1 border-l-2 border-purple-900/50 min-h-5">
                              {note.fields.plan || "—"}
                            </p>
                          </div>
                        </div>
                      )}
                    </div>
                  ))
                )}
              </div>
            </div>
          </>
        ) : (
          <div className="flex-1 flex flex-col items-center justify-center border border-zinc-800 rounded-xl bg-zinc-900/30 border-dashed">
            <div className="text-zinc-700 text-6xl mb-4 opacity-50">🩺</div>
            <p className="text-zinc-500 font-medium">
              Selecciona un paciente para comenzar la atención
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
