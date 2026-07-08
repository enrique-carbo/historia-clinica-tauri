import { useState } from "react";
import { Navbar, Tab } from "../../components/ui/Navbar";
import { PatientEhrView } from "../../components/PatientEhrView"; // ← NUEVO
import { EntityTest } from "../../components/EntityTest";

const MEDICO_TABS: Tab[] = [
  { id: "paciente", label: "Paciente", icon: "👤" },
  { id: "ehr", label: "Historia Clínica", icon: "📋" },
];

export function MedicoView() {
  const [activeTab, setActiveTab] = useState("ehr"); // ← Default a la nueva vista

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
    </Navbar>
  );
}
