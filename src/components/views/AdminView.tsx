import { useState } from "react";
import { Navbar, Tab } from "../../components/ui/Navbar";
import { Entity } from "../Entity";

const ADMIN_TABS: Tab[] = [{ id: "paciente", label: "Paciente", icon: "👤" }];

export function AdminView() {
  const [activeTab, setActiveTab] = useState("paciente"); // ← Default a la nueva vista

  return (
    <Navbar tabs={ADMIN_TABS} activeTab={activeTab} onTabChange={setActiveTab}>
      {activeTab === "paciente" && (
        <div className="flex flex-col gap-8">
          <div>
            <h2 className="text-lg font-semibold mb-4 text-zinc-300">
              Admitir Nuevo Paciente
            </h2>
            <Entity/>
          </div>
        </div>
      )}
    </Navbar>
  );
}
