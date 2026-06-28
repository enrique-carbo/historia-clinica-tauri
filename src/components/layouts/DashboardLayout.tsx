import { ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAuthStore } from "../../stores/useAuthStore";
import { usePatientStore } from "../../stores/usePatientStore";
import { usePatientRegistryStore } from "../../stores/usePatientRegistryStore";

interface DashboardLayoutProps {
  children: ReactNode;
}

export function DashboardLayout({ children }: DashboardLayoutProps) {
  const { activeUser: user, lockVault } = useAuthStore();

  // Cerrar sesión segura (Se mueve aquí porque es responsabilidad del Layout)
  const handleLogout = async () => {
    try {
      await invoke("lock_vault");
      lockVault();
      usePatientStore.getState().resetAllPatientData();
      usePatientRegistryStore.setState({ patients: [] });
    } catch (err) {
      console.error("Error al cerrar la bóveda:", err);
    }
  };

  if (!user) return null; // Fallback de seguridad

  return (
    <div className="min-h-screen bg-zinc-950 text-zinc-200 font-sans flex flex-col">
      {/* HEADER FIJO */}
      <header className="sticky top-0 z-10 bg-zinc-950/80 backdrop-blur-sm border-b border-zinc-800 px-10 py-4 flex justify-between items-center">
        <div>
          <h1 className="text-xl font-bold text-zinc-100 m-0">
            🩺 Simplex Health Core
          </h1>
          <p className="text-xs text-zinc-500 mt-0.5">
            Ecosistema Clínico Descentralizado — Rol:{" "}
            <strong className="text-blue-500 uppercase">{user.role}</strong>
          </p>
        </div>
        <div className="text-right flex items-center gap-4">
          <span className="text-sm text-zinc-500">
            Usuario: <strong className="text-zinc-300">{user.username}</strong>
          </span>
          <button
            onClick={handleLogout}
            className="px-3 py-1.5 bg-zinc-800 border border-zinc-700 text-zinc-300 rounded text-xs hover:bg-red-900/50 hover:border-red-800 hover:text-red-300 transition-colors cursor-pointer"
          >
            🔒 Cerrar Bóveda
          </button>
        </div>
      </header>

      {/* CONTENIDO DINÁMICO (Scrollable) */}
      <main className="flex-1 overflow-y-auto p-10">{children}</main>
    </div>
  );
}
