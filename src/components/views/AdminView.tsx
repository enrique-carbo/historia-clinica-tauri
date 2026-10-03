import { Navbar, Tab } from "../../components/ui/Navbar";
import { Entity } from "../Entity";
import { useAuthStore } from "../../stores/useAuthStore";
import { useNavigationStore, AdminTab } from "../../stores/useNavigationStore";
import { AdminUsersPanel } from "./AdminUsersPanel";
import { AdminAuditPanel } from "./AdminAuditPanel";
import { AdminSeedPanel } from "./AdminSeedPanel";

const ADMIN_TABS: Tab[] = [
  { id: "usuarios", label: "Usuarios", icon: "👥" },
  { id: "paciente", label: "Admisión", icon: "👤" },
  { id: "auditoria", label: "Auditoría", icon: "📋" },
  { id: "semilla", label: "Frase Semilla", icon: "🌱" },
];

const ASISTENTE_TABS: Tab[] = [{ id: "paciente", label: "Admisión de Pacientes", icon: "👤" }];

export function AdminView() {
  const { activeUser } = useAuthStore();
  const { activeAdminTab, setActiveAdminTab } = useNavigationStore();

  const isAdmin = activeUser?.role === "administrador";
  const tabs = isAdmin ? ADMIN_TABS : ASISTENTE_TABS;
  const activeTab = isAdmin ? activeAdminTab : "paciente";

  return (
    <Navbar
      tabs={tabs}
      activeTab={activeTab}
      onTabChange={(t) => setActiveAdminTab(t as AdminTab)}
    >
      {activeTab === "usuarios" && <AdminUsersPanel />}

      {activeTab === "paciente" && (
        <div className="flex flex-col gap-8">
          <div>
            <h2 className="text-lg font-semibold mb-4 text-zinc-300">
              Admitir Nuevo Paciente
            </h2>
            <Entity />
          </div>
        </div>
      )}

      {activeTab === "auditoria" && <AdminAuditPanel />}

      {activeTab === "semilla" && <AdminSeedPanel />}
    </Navbar>
  );
}
