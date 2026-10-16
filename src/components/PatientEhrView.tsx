// src/components/PatientEhrView.tsx
import { useState, useCallback, useEffect } from "react";
import { useEntityStore } from "../stores/useEntityStore";
import { usePatientStore } from "../stores/usePatientStore";
import { useEntryStore } from "../stores/useEntryStore";
import { SoapCard } from "./SoapCard";
import { AllergyCard } from "./AllergyCard";
import { Alert } from "./ui/Alert";
import { EmptyState } from "./ui/EmptyState";
import { calculateAge } from "../utils/dateUtils";
import { FLAT_PATIENT_FIELDS } from "../utils/schema";

// Campos ya mostrados en el encabezado de la ficha (nombre/apellido como
// título, dni como badge, fecha_nacimiento como edad). El resto de los
// campos del schema (incluidos los aplanados de groups como
// responsable_*) se listan en "Datos de contacto" — un campo nuevo en
// entity_schema.json aparece acá sin tocar este archivo.
const HEADER_FIELDS = ["nombre", "apellido", "dni", "fecha_nacimiento"];
const CONTACT_FIELDS = FLAT_PATIENT_FIELDS.filter(
  (f) => !HEADER_FIELDS.includes(f.name),
);

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
    <div className="col-span-1 md:col-span-4 flex flex-col border border-border rounded-xl bg-surface/50 backdrop-blur-sm overflow-hidden shadow-lg min-h-87.5 md:min-h-0">
      <div className="p-4 border-b border-border space-y-3 bg-surface">
        <h3 className="text-xs font-bold text-text-muted uppercase tracking-widest">
          Padrón de Pacientes
        </h3>
        <input
          type="text"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          placeholder="Buscar por nombre, DNI o teléfono..."
          aria-label="Buscar pacientes"
          className="w-full rounded-lg bg-field border border-border-strong px-4 py-2.5 text-sm text-text-primary focus:border-focus focus:ring-1 focus:ring-focus transition-colors outline-none"
        />
      </div>

      <div className="flex-1 overflow-y-auto p-2 space-y-1 custom-scrollbar max-h-[40vh] md:max-h-none">
        {isLoading ? (
          <div className="flex justify-center py-8">
            <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-500"></div>
          </div>
        ) : entities.length === 0 ? (
          <EmptyState
            icon={null}
            title={searchQuery ? "No se encontraron coincidencias" : "Comienza escribiendo para buscar"}
          />
        ) : (
          entities.map((entity) => (
            <button
              key={entity.id}
              onClick={() => handleSelectPatient(entity.id)}
              className={`w-full text-left p-3 rounded-lg transition-colors duration-200 group ${
                patient?.id === entity.id
                  ? "bg-blue-900/20 border border-blue-500/50"
                  : "bg-canvas/50 border border-transparent hover:bg-surface-raised hover:border-border-strong"
              }`}
            >
              <div className="flex justify-between items-start gap-2">
                <div className="font-semibold text-text-primary group-hover:text-text-inverse truncate min-w-0 flex-1">
                  {entity.data.nombre} {entity.data.apellido}
                </div>
                {patient?.id === entity.id && (
                  <span className="h-2 w-2 shrink-0 rounded-full bg-blue-500 mt-1.5"></span>
                )}
              </div>
              <div className="text-xs text-text-muted mt-1.5 flex flex-wrap items-center gap-2">
                <span className="bg-surface-raised px-1.5 py-0.5 rounded text-text-secondary font-mono">
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
          <div className="border border-border rounded-xl bg-surface/50 shadow-lg overflow-hidden transition-colors duration-300">
            <div
              role="button"
              tabIndex={0}
              className="p-4 sm:p-5 cursor-pointer hover:bg-surface-raised/30 transition-colors flex justify-between items-center gap-2"
              onClick={() => setIsPatientDetailsOpen(!isPatientDetailsOpen)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  setIsPatientDetailsOpen(!isPatientDetailsOpen);
                }
              }}
              aria-expanded={isPatientDetailsOpen}
            >
              <div className="flex items-center gap-3 sm:gap-4 min-w-0">
                <div className="p-2.5 sm:p-3 rounded-full bg-blue-900/20 text-blue-400 shrink-0">
                  <svg aria-hidden="true" xmlns="http://www.w3.org/2000/svg" className="h-5 w-5 sm:h-6 sm:w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
                  </svg>
                </div>
                <div className="min-w-0">
                  <div className="flex items-center gap-2 sm:gap-3 flex-wrap">
                    <h2 className="text-lg sm:text-xl font-bold text-text-primary tracking-tight truncate">
                      {patient.data.nombre} {patient.data.apellido}
                    </h2>
                  </div>
                  <div className="flex items-center gap-2 sm:gap-3 mt-0.5 text-xs sm:text-sm text-text-secondary flex-wrap">
                    <span className="font-mono bg-surface-raised px-1.5 py-0.5 rounded text-xs">
                      DNI: {patient.data.dni}
                    </span>
                    <span>{calculateAge(patient.data.fecha_nacimiento)} años</span>
                  </div>
                </div>
              </div>

              <div className="flex items-center gap-2 sm:gap-4 shrink-0">
                <div className="text-right hidden sm:block">
                  <div className="text-xs font-mono text-text-muted">ID: {patient.id}</div>
                </div>
                <div className={`transform transition-transform duration-200 ${isPatientDetailsOpen ? "rotate-180" : ""}`}>
                  <svg aria-hidden="true" xmlns="http://www.w3.org/2000/svg" className="h-5 w-5 text-text-muted" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
                  </svg>
                </div>
              </div>
              </div>

              {/* Alergias: siempre visible, sin importar el colapso de la ficha */}
              <AllergyCard subjectId={patient.id} />

              {isPatientDetailsOpen && (
                <div className="p-4 sm:p-5 pt-4 border-t border-border bg-canvas/30 animate-in slide-in-from-top-2 duration-200">
                  {/* Datos de contacto */}
                  <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
                    {CONTACT_FIELDS.map((field) => ({
                      label: field.label,
                      value: patient.data[field.name],
                    })).map((item, idx) => (
                      <div key={idx} className="bg-surface/50 rounded-lg p-3 border border-border/50">
                        <div className="text-[10px] text-text-muted uppercase font-bold tracking-wider">{item.label}</div>
                        <div className="text-sm text-text-primary mt-0.5 truncate">{item.value || "—"}</div>
                      </div>
                    ))}
                  </div>
                </div>
              )}
          </div>

          {/* HISTORIAL: ENTRADAS SOAP */}
          <div className="border border-border rounded-xl bg-surface/50 backdrop-blur-sm shadow-lg flex-1 flex flex-col overflow-hidden min-h-0">
            <div className="p-4 border-b border-border bg-surface flex items-center justify-between gap-3">
              <h3 className="text-xs font-bold text-text-muted uppercase tracking-widest">
                Historial — Notas SOAP
              </h3>
              {!entriesLoading && !entriesError && soapEntries.length > 0 && (
                <span className="text-[10px] font-mono bg-surface-raised text-text-secondary px-2 py-0.5 rounded">
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
                <Alert variant="error" className="text-sm">
                  {entriesError}
                </Alert>
              ) : soapEntries.length === 0 ? (
                <EmptyState
                  title="Sin notas SOAP registradas"
                  hint="Se crean desde la pestaña Entries"
                />
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
