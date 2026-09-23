// src/components/views/MedicoView.tsx
import { useState } from "react";
import { Navbar, Tab } from "../../components/ui/Navbar";
import { PatientEhrView } from "../../components/PatientEhrView";
import { Entity } from "../Entity";
import { ProfileView } from "../../components/ProfileView";
import { EntryTimeline } from "../../components/EntryTimeline";
import { EntryForm } from "../../components/EntryForm";
import { useNavigationStore, MedicoTab } from "../../stores/useNavigationStore";
import { usePatientStore } from "../../stores/usePatientStore";

const MEDICO_TABS: Tab[] = [
  { id: "perfil", label: "Perfil", icon: "👨🏻‍⚕️" },
  { id: "paciente", label: "Paciente", icon: "👤" },
  { id: "ehr", label: "Historia Clínica", icon: "📋" },
  { id: "entries", label: "Entries", icon: "⏱️" },
];

export function MedicoView() {
  const { activeMedicoTab, setActiveMedicoTab } = useNavigationStore();
  const selectedPatient = usePatientStore((s) => s.selected?.patient ?? null);
  const [showEntryForm, setShowEntryForm] = useState(false);

  return (
    <Navbar tabs={MEDICO_TABS} activeTab={activeMedicoTab} onTabChange={(tab) => setActiveMedicoTab(tab as MedicoTab)}>
      {activeMedicoTab === "ehr" && <PatientEhrView />}

      {activeMedicoTab === "paciente" && (
        <div className="flex flex-col gap-8 mx-auto w-full p-6">
          <div>
            <h2 className="text-lg font-semibold mb-4 text-zinc-300 flex items-center gap-2">
              <span className="p-1.5 rounded bg-blue-900/20 text-blue-400">👤</span>
              Admitir / Editar Paciente
            </h2>
            <Entity/>
          </div>
        </div>
      )}

      {activeMedicoTab === "perfil" && (
        <div className="flex flex-col gap-8 max-w-3xl mx-auto w-full p-6">
          <div>
            <h2 className="text-lg font-semibold mb-4 text-zinc-300">Perfil Profesional</h2>
            <ProfileView />
          </div>
        </div>
      )}

      {activeMedicoTab === "entries" && (
        <div className="flex flex-col gap-6 mx-auto w-full p-6">
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-semibold text-zinc-300 flex items-center gap-2">
              <span className="p-1.5 rounded bg-purple-900/20 text-purple-400">⏱️</span>
              Entries (Append-Only)
            </h2>
            {selectedPatient && (
              <button
                onClick={() => setShowEntryForm(!showEntryForm)}
                className="px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded-lg text-sm font-medium transition-colors"
              >
                {showEntryForm ? "Ver Timeline" : "+ Nueva Entry"}
              </button>
            )}
          </div>

          {showEntryForm ? (
            <EntryForm onSuccess={() => setShowEntryForm(false)} />
          ) : (
            <>
              {selectedPatient ? (
                <EntryTimeline subjectId={selectedPatient.id} />
              ) : (
                <div className="p-4 rounded-lg bg-zinc-900 border border-zinc-800 text-zinc-500 text-sm">
                  Seleccioná un paciente primero para ver sus entries.
                </div>
              )}
            </>
          )}
        </div>
      )}
    </Navbar>
  );
}
