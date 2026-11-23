import { ReactNode, useEffect } from "react";
import { useNavigationStore, MedicoTab, AdminTab } from "../../stores/useNavigationStore";
import { useAuthStore } from "../../stores/useAuthStore";
import { useModuleAvailable } from "../../hooks/useModuleAvailable";
import {
  ClipboardList,
  Clock,
  DatabaseBackup,
  FlaskConical,
  LogOut,
  Puzzle,
  Sprout,
  UserPlus,
  UserRound,
  UsersRound,
  X,
} from "lucide-react";
import { Button } from "./Button";
import { ThemeToggle } from "./ThemeToggle";
import { AppVersion } from "./AppVersion";

interface NavigationDrawerProps {
  onLogout: () => void;
}

interface MenuItem {
  key: string;
  label: string;
  icon: ReactNode;
  onClick?: () => void;
}

const MEDICO_MENU: MenuItem[] = [
  { key: "perfil", label: "Perfil", icon: <UserRound size={20} /> },
  { key: "paciente", label: "Paciente", icon: <UsersRound size={20} /> },
  { key: "ehr", label: "Historia Clínica", icon: <ClipboardList size={20} /> },
  { key: "entries", label: "Entries", icon: <Clock size={20} /> },
];

const SOLICITUDES_MENU_ITEM: MenuItem = {
  key: "solicitudes",
  label: "Solicitudes",
  icon: <FlaskConical size={20} />,
};

const MENU_ITEMS: Record<string, MenuItem[]> = {
  administrador: [
    { key: "usuarios", label: "Usuarios", icon: <UsersRound size={20} /> },
    { key: "paciente", label: "Admisión de Pacientes", icon: <UserPlus size={20} /> },
    { key: "auditoria", label: "Auditoría", icon: <ClipboardList size={20} /> },
    { key: "semilla", label: "Frase Semilla", icon: <Sprout size={20} /> },
    { key: "respaldos", label: "Respaldos", icon: <DatabaseBackup size={20} /> },
    { key: "modulos", label: "Módulos", icon: <Puzzle size={20} /> },
  ],
  asistente: [
    { key: "paciente", label: "Admisión de Pacientes", icon: <UserPlus size={20} /> },
  ],
  profesional: MEDICO_MENU,
  paciente: [
    { key: "perfil", label: "Mi Perfil", icon: <UserRound size={20} /> },
  ],
};

export function NavigationDrawer({ onLogout }: NavigationDrawerProps) {
  const {
    isDrawerOpen,
    closeDrawer,
    activeMedicoTab,
    setActiveMedicoTab,
    activeAdminTab,
    setActiveAdminTab,
  } = useNavigationStore();
  const { activeUser: user } = useAuthStore();

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") closeDrawer();
    };
    if (isDrawerOpen) {
      window.addEventListener("keydown", handleKeyDown);
    }
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isDrawerOpen, closeDrawer]);

  const usesMedicoTabs = user?.role === "profesional";
  const usesAdminTabs = user?.role === "administrador" || user?.role === "asistente";
  const solicitudesAvailable = useModuleAvailable("solicitudes");

  const handleMenuClick = (key: string) => {
    if (usesMedicoTabs) {
      setActiveMedicoTab(key as MedicoTab);
    } else if (usesAdminTabs) {
      setActiveAdminTab(key as AdminTab);
    }
    closeDrawer();
  };

  const baseItems = user ? MENU_ITEMS[user.role] || [] : [];
  const menuItems =
    usesMedicoTabs && solicitudesAvailable
      ? [
          ...baseItems.slice(0, baseItems.findIndex((i) => i.key === "entries")),
          SOLICITUDES_MENU_ITEM,
          ...baseItems.slice(baseItems.findIndex((i) => i.key === "entries")),
        ]
      : baseItems;

  return (
    <div
      className={`fixed inset-0 z-50 transition-opacity duration-300 ${
        isDrawerOpen ? "opacity-100 pointer-events-auto" : "opacity-0 pointer-events-none"
      }`}
    >
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/60 backdrop-blur-xs"
        onClick={closeDrawer}
      />

      {/* Panel del Drawer */}
      <aside
        className={`absolute top-0 left-0 h-full w-80 max-w-[85vw] bg-surface border-r border-border text-text-primary shadow-2xl flex flex-col transform transition-transform duration-300 ease-in-out ${
          isDrawerOpen ? "translate-x-0" : "-translate-x-full"
        }`}
      >
        {/* Cabecera */}
        <div className="p-4 border-b border-border flex justify-between items-center bg-canvas/50">
          <h2 className="text-sm font-bold text-text-primary uppercase tracking-wider">
            Navegación
          </h2>
          <button
            onClick={closeDrawer}
            className="p-1.5 rounded-lg text-text-secondary hover:text-text-primary hover:bg-surface-raised transition-colors"
            aria-label="Cerrar menú"
          >
            <X size={20} aria-hidden="true" />
          </button>
        </div>

        {/* Info del usuario */}
        <div className="p-4 border-b border-border flex items-start justify-between gap-3">
          <div>
            <p className="text-xs text-text-secondary">Sesión activa</p>
            <p className="text-sm font-semibold text-text-primary">{user?.username}</p>
            <span className="inline-block mt-1 text-[10px] font-mono font-bold bg-info-tint/30 text-info-text border border-info-border/50 px-1.5 py-0.5 rounded uppercase">
              {user?.role}
            </span>
          </div>
          <ThemeToggle />
        </div>

        {/* Menú de navegación */}
        <nav className="flex-1 overflow-y-auto p-4 custom-scrollbar">
          <ul className="space-y-1">
            {menuItems.map((item) => {
              const isActive =
                (usesMedicoTabs && activeMedicoTab === item.key) ||
                (usesAdminTabs && activeAdminTab === item.key);
              return (
                <li key={item.key}>
                  <button
                    onClick={() => handleMenuClick(item.key)}
                    className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm transition-colors text-left ${
                      isActive
                        ? "bg-surface-raised text-text-primary"
                        : "text-text-primary hover:bg-surface-raised hover:text-text-primary"
                    }`}
                  >
                    <span className="inline-flex shrink-0" aria-hidden="true">{item.icon}</span>
                    {item.label}
                  </button>
                </li>
              );
            })}
          </ul>
        </nav>

        {/* Logout */}
        <div className="p-4">
          <Button
            variant="ghost-danger"
            onClick={() => {
              closeDrawer();
              onLogout();
            }}
            className="w-full flex items-center gap-3 font-normal"
          >
            <LogOut size={20} aria-hidden="true" />
            Cerrar Bóveda
          </Button>
        </div>
        <div className="p-4 border-t border-border">
          <AppVersion className="block w-full text-center" />
        </div>
      </aside>
    </div>
  );
}
