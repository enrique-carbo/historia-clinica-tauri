import { Navbar, Tab } from "../../components/ui/Navbar";
import { Entity } from "../Entity";
import { useAuthStore } from "../../stores/useAuthStore";
import { useNavigationStore, AdminTab } from "../../stores/useNavigationStore";
import { AdminUsersPanel } from "./AdminUsersPanel";
import { AdminAuditPanel } from "./AdminAuditPanel";
import { AdminSeedPanel } from "./AdminSeedPanel";
import { AdminBackupPanel } from "./AdminBackupPanel";
import { AdminModulesPanel } from "./AdminModulesPanel";
import {
  ClipboardList,
  DatabaseBackup,
  Puzzle,
  Sprout,
  UserPlus,
  Users,
} from "lucide-react";

const ADMIN_TABS: Tab[] = [
  { id: "usuarios", label: "Usuarios", icon: <Users size={18} /> },
  { id: "paciente", label: "Admisión", icon: <UserPlus size={18} /> },
  { id: "auditoria", label: "Auditoría", icon: <ClipboardList size={18} /> },
  { id: "semilla", label: "Frase Semilla", icon: <Sprout size={18} /> },
  { id: "respaldos", label: "Respaldos", icon: <DatabaseBackup size={18} /> },
  { id: "modulos", label: "Módulos", icon: <Puzzle size={18} /> },
];

const ASISTENTE_TABS: Tab[] = [
  { id: "paciente", label: "Admisión de Pacientes", icon: <UserPlus size={18} /> },
];

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
            <h2 className="text-lg font-semibold mb-4 text-text-primary">
              Admitir Nuevo Paciente
            </h2>
            <Entity />
          </div>
        </div>
      )}

      {activeTab === "auditoria" && <AdminAuditPanel />}

      {activeTab === "semilla" && <AdminSeedPanel />}

      {activeTab === "respaldos" && <AdminBackupPanel />}

      {activeTab === "modulos" && <AdminModulesPanel />}
    </Navbar>
  );
}
