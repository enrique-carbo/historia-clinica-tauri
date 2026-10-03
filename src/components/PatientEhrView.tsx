// src/components/PatientEhrView.tsx
import { useState, useCallback, useEffect } from "react";
import { useEntityStore } from "../stores/useEntityStore";
import { usePatientStore } from "../stores/usePatientStore";
import { useEntryStore } from "../stores/useEntryStore";
import { SoapCard } from "./SoapCard";
import { AllergyCard } from "./AllergyCard";
import { calculateAge } from "../utils/dateUtils";

export function PatientEhrView() {
  const { entities, isLoading: entityLoading, searchEntities } = useEntityStore();
  const { selected, isLoading: patientLoading, selectPatient } = usePatientStore();
  const {
    entries: soapEntries,
    isLoading: entriesLoading,
    error: entriesError,
    fetchEntries,
    selectEntry,
    clearError,
  } = useEntryStore();

  const [searchQuery, setSearchQuery] = useState("");
  const [isPatientDetailsOpen, setIsPatientDetailsOpen] = useState(false);


  useEffect(() => {
    const timer = setTimeout(() => {
      if (searchQuery.trim()) searchEntities("patient", searchQuery);
    }, 300);
    return () => clearTimeout(timer);
  }, [searchQuery, searchEntities]);

  // Selección unificada: carga paciente + notas
  const handleSelectPatient = useCallback(
    async (entityId: number) => {
      await selectPatient(entityId);

    },
    [selectPatient],
  );

  const patient = selected?.patient;
  const isLoading = patientLoading || entityLoading;

  useEffect(() => {
    if (!patient) return;
    clearError();
    fetchEntries(patient.id, "SOAP_NOTE");
  }, [patient, fetchEntries, clearError]);

  return (
  <div className="grid grid-cols-1 md:grid-cols-12 gap-6 h-auto md:h-[calc(100vh-180px)]">
    {/* PANEL IZQUIERDO: PADRÓN DE PACIENTES */}
    <div className="col-span-1 md:col-span-4 flex flex-col border border-zinc-800 rounded-xl bg-zinc-900/50 backdrop-blur-sm overflow-hidden shadow-lg min-h-87.5 md:min-h-0">
      <div className="p-4 border-b border-zinc-800 space-y-3 bg-zinc-900">
        <h3 className="text-xs font-bold text-zinc-500 uppercase tracking-widest">
          Padrón de Pacientes
        </h3>
        <input
          type="text"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          placeholder="Buscar por nombre, DNI o teléfono..."
          className="w-full rounded-lg bg-zinc-950 border border-zinc-700 px-4 py-2.5 text-sm text-zinc-200 focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-colors outline-none"
        />
      </div>

      <div className="flex-1 overflow-y-auto p-2 space-y-1 custom-scrollbar max-h-[40vh] md:max-h-none">
        {isLoading ? (
          <div className="flex justify-center py-8">
            <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-500"></div>
          </div>
        ) : entities.length === 0 ? (
          <div className="text-center py-12 text-zinc-500 text-sm">
            {searchQuery ? "No se encontraron coincidencias" : "Comienza escribiendo para buscar"}
          </div>
        ) : (
          entities.map((entity) => (
            <button
              key={entity.id}
              onClick={() => handleSelectPatient(entity.id)}
              className={`w-full text-left p-3 rounded-lg transition-colors duration-200 group ${
                patient?.id === entity.id
                  ? "bg-blue-900/20 border border-blue-500/50 shadow-md"
                  : "bg-zinc-950/50 border border-transparent hover:bg-zinc-800 hover:border-zinc-700"
              }`}
            >
              <div className="flex justify-between items-start gap-2">
                <div className="font-semibold text-zinc-200 group-hover:text-white truncate min-w-0 flex-1">
                  {entity.data.nombre} {entity.data.apellido}
                </div>
                {patient?.id === entity.id && (
                  <span className="h-2 w-2 shrink-0 rounded-full bg-blue-500 mt-1.5"></span>
                )}
              </div>
              <div className="text-xs text-zinc-500 mt-1.5 flex flex-wrap items-center gap-2">
                <span className="bg-zinc-800 px-1.5 py-0.5 rounded text-zinc-400 font-mono">
                  DNI: {entity.data.dni}
                </span>
                <span className="truncate">{entity.data.telefono}</span>
              </div>
            </button>
          ))
        )}
      </div>
    </div>

    {/* PANEL DERECHO: FICHA + HISTORIAL */}
    <div className="col-span-1 md:col-span-8 flex flex-col gap-4">
      {patient && (
        <>
          {/* FICHA COLAPSABLE */}
          <div className="border border-zinc-800 rounded-xl bg-zinc-900/50 shadow-lg overflow-hidden transition-colors duration-300">
            <div
              className="p-4 sm:p-5 cursor-pointer hover:bg-zinc-800/30 transition-colors flex justify-between items-center gap-2"
              onClick={() => setIsPatientDetailsOpen(!isPatientDetailsOpen)}
            >
              <div className="flex items-center gap-3 sm:gap-4 min-w-0">
                <div className="p-2.5 sm:p-3 rounded-full bg-blue-900/20 text-blue-400 shrink-0">
                  <svg xmlns="http://www.w3.org/2000/svg" className="h-5 w-5 sm:h-6 sm:w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
                  </svg>
                </div>
                <div className="min-w-0">
                  <div className="flex items-center gap-2 sm:gap-3 flex-wrap">
                    <h2 className="text-lg sm:text-xl font-bold text-zinc-100 tracking-tight truncate">
                      {patient.data.nombre} {patient.data.apellido}
                    </h2>
                  </div>
                  <div className="flex items-center gap-2 sm:gap-3 mt-0.5 text-xs sm:text-sm text-zinc-400 flex-wrap">
                    <span className="font-mono bg-zinc-800 px-1.5 py-0.5 rounded text-xs">
                      DNI: {patient.data.dni}
                    </span>
                    <span>{calculateAge(patient.data.fecha_nacimiento)} años</span>
                  </div>
                </div>
              </div>

              <div className="flex items-center gap-2 sm:gap-4 shrink-0">
                <div className="text-right hidden sm:block">
                  <div className="text-xs font-mono text-zinc-500">ID: {patient.id}</div>
                </div>
                <div className={`transform transition-transform duration-200 ${isPatientDetailsOpen ? "rotate-180" : ""}`}>
                  <svg xmlns="http://www.w3.org/2000/svg" className="h-5 w-5 text-zinc-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                  </svg>
                </div>
              </div>
              </div>

              {/* Alergias: siempre visible, sin importar el colapso de la ficha */}
              <AllergyCard subjectId={patient.id} />

              {isPatientDetailsOpen && (
                <div className="p-4 sm:p-5 pt-4 border-t border-zinc-800 bg-zinc-950/30 animate-in slide-in-from-top-2 duration-200">
                  {/* Datos de contacto */}
                  <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
                    {[
                      { label: "Teléfono", value: patient.data.telefono },
                      { label: "Email", value: patient.data.email },
                      { label: "Dirección", value: patient.data.direccion },
                    ].map((item, idx) => (
                      <div key={idx} className="bg-zinc-900/50 rounded-lg p-3 border border-zinc-800/50">
                        <div className="text-[10px] text-zinc-500 uppercase font-bold tracking-wider">{item.label}</div>
                        <div className="text-sm text-zinc-300 mt-0.5 truncate">{item.value || "—"}</div>
                      </div>
                    ))}
                  </div>
                </div>
              )}
          </div>

          {/* HISTORIAL: ENTRADAS SOAP */}
          <div className="border border-zinc-800 rounded-xl bg-zinc-900/50 backdrop-blur-sm shadow-lg flex-1 flex flex-col overflow-hidden min-h-0">
            <div className="p-4 border-b border-zinc-800 bg-zinc-900 flex items-center justify-between gap-3">
              <h3 className="text-xs font-bold text-zinc-500 uppercase tracking-widest">
                Historial — Notas SOAP
              </h3>
              {!entriesLoading && !entriesError && soapEntries.length > 0 && (
                <span className="text-[10px] font-mono bg-zinc-800 text-zinc-400 px-2 py-0.5 rounded">
                  {soapEntries.length}
                </span>
              )}
            </div>

            <div className="flex-1 overflow-y-auto p-4 custom-scrollbar">
              {entriesLoading && soapEntries.length === 0 ? (
                <div className="flex justify-center py-10">
                  <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-500"></div>
                </div>
              ) : entriesError ? (
                <div className="p-4 rounded-lg bg-red-950/30 border border-red-800/50 text-red-400 text-sm">
                  {entriesError}
                </div>
              ) : soapEntries.length === 0 ? (
                <div className="flex flex-col items-center justify-center py-10 text-zinc-500">
                  <svg
                    className="w-10 h-10 mb-3 text-zinc-500"
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
                  <p className="text-sm">Sin notas SOAP registradas</p>
                  <p className="text-xs text-zinc-500 mt-1">
                    Se crean desde la pestaña Entries
                  </p>
                </div>
              ) : (
                <div className="space-y-4">
                  {soapEntries.map((entry) => (
                    <SoapCard key={entry.id} entry={entry} onClick={() => selectEntry(entry)} />
                  ))}
                </div>
              )}
            </div>
          </div>
        </>
      )}
    </div>
  </div>
)}
