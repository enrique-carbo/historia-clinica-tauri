import { useState } from "react";
import { Navbar, Tab } from "../../components/ui/Navbar";
import { PatientEhrView } from "../../components/PatientEhrView";
import { EntityTest } from "../../components/EntityTest";
import { ProfileView } from "../../components/ProfileView";

const MEDICO_TABS: Tab[] = [
  { id: "perfil", label: "Perfil", icon: "👨🏻‍⚕️" },
  { id: "paciente", label: "Paciente", icon: "👤" },
  { id: "ehr", label: "Historia Clínica", icon: "📋" },
];

export function MedicoView() {
  const [activeTab, setActiveTab] = useState("perfil");

  return (
    <Navbar tabs={MEDICO_TABS} activeTab={activeTab} onTabChange={setActiveTab}>
      {activeTab === "ehr" && <PatientEhrView />}

      {activeTab === "paciente" && (
        <div className="flex flex-col gap-8">
          <div>
            <h2 className="text-lg font-semibold mb-4 text-zinc-300">
              Admitir Nuevo Paciente
            </h2>
            <EntityTest />
          </div>
        </div>
      )}

      {activeTab === "perfil" && (
        <div className="flex flex-col gap-8">
          <div>
            <h2 className="text-lg font-semibold mb-4 text-zinc-300">
              Perfil Profesional
            </h2>
            <ProfileView />
          </div>
        </div>
      )}


    </Navbar>
  );
}
