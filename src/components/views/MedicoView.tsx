// src/components/views/MedicoView.tsx
import { Navbar, Tab } from "../../components/ui/Navbar";
import { PatientEhrView } from "../../components/PatientEhrView";
import { Entity } from "../Entity";
import { ProfileView } from "../../components/ProfileView";
import { MedicalHistoryForm } from "../../components/MedicalHistoryForm";
import { useNavigationStore, MedicoTab } from "../../stores/useNavigationStore";

const MEDICO_TABS: Tab[] = [
  { id: "perfil", label: "Perfil", icon: "👨🏻‍⚕️" },
  { id: "paciente", label: "Paciente", icon: "👤" },
  { id: "antecedentes", label: "Antecedentes", icon: "📝" },
  { id: "ehr", label: "Historia Clínica", icon: "📋" },
];

export function MedicoView() {
  const { activeMedicoTab, setActiveMedicoTab } = useNavigationStore();

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

      {activeMedicoTab === "antecedentes" && (
        <div className="flex flex-col gap-8 mx-auto w-full p-6">
          <div>
            <MedicalHistoryForm />
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
    </Navbar>
  );
}
