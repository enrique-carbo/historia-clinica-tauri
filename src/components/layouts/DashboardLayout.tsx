import { ReactNode, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAuthStore } from "../../stores/useAuthStore";
import { usePatientStore } from "../../stores/usePatientStore";
import { useEntityStore } from "../../stores/useEntityStore";
import { useNoteStore } from "../../stores/useNoteStore";
import { useNavigationStore } from "../../stores/useNavigationStore";
import { useSeedStore } from "../../stores/useSeedStore";
import { NavigationDrawer } from "../ui/Drawer";
import { SeedPhraseSetup } from "../SeedPhraseSetup";

interface DashboardLayoutProps {
  children: ReactNode;
}

export function DashboardLayout({ children }: DashboardLayoutProps) {
  const { activeUser: user, lockVault } = useAuthStore();
  const { openDrawer } = useNavigationStore();
  const { isSeedConfigured, setSeedConfigured } = useSeedStore();
  const [showSeedSetup, setShowSeedSetup] = useState(false);

  const handleLogout = async () => {
    try {
      await invoke("lock_vault");
      lockVault();

      // Limpiar stores
      useEntityStore.setState({
        entities: [],
        selectedEntity: null,
        isLoading: false,
        error: null,
      });
      useNoteStore.setState({
        notes: [],
        selectedNote: null,
        isLoading: false,
        error: null,
      });
      usePatientStore.setState({
        selected: null,
        isLoading: false,
        error: null,
      });
    } catch (err) {
      console.error("Error al cerrar la bóveda:", err);
    }
  };

  if (!user) return null;

  // Mostrar SeedPhraseSetup como modal
  if (showSeedSetup) {
    return (
      <SeedPhraseSetup
        onComplete={() => {
          setSeedConfigured(true);
          setShowSeedSetup(false);
        }}
      />
    );
  }

  return (
    <div className="min-h-screen bg-zinc-950 text-zinc-200 font-sans flex flex-col">
      {/* HEADER */}
      <header className="sticky top-0 z-10 bg-zinc-950/80 backdrop-blur-sm border-b border-zinc-800 px-4 sm:px-10 py-4 flex justify-between items-center">
        <div className="flex items-center gap-3">
          {/* Botón Hamburguesa */}
          <button
            onClick={openDrawer}
            className="p-2 rounded-lg bg-zinc-900 border border-zinc-800 text-zinc-300 hover:text-white hover:bg-zinc-800 transition-colors"
            aria-label="Abrir menú"
          >
            <svg
              className="w-5 h-5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M4 6h16M4 12h16M4 18h16"
              />
            </svg>
          </button>

          <div>
            <h1 className="text-lg sm:text-xl font-bold text-zinc-100 m-0">
              🩺 Simplex Health
            </h1>
            <p className="text-xs text-zinc-500 mt-0.5 hidden sm:block">
              Historia Clínica — Rol:{" "}
              <strong className="text-blue-500 uppercase">{user.role}</strong>
            </p>
          </div>
        </div>

        <div className="text-right flex items-center gap-2 sm:gap-4">
          {/* Indicador de seed no configurada */}
          {!isSeedConfigured && (
            <button
              onClick={() => setShowSeedSetup(true)}
              className="px-3 py-1.5 bg-yellow-900/30 border border-yellow-700/50 text-yellow-400 rounded text-xs hover:bg-yellow-900/50 transition-colors cursor-pointer hidden sm:block"
              title="Configurar frase semilla"
            >
              🔐 Configurar Seed
            </button>
          )}
          <span className="text-xs sm:text-sm text-zinc-500 hidden sm:inline">
            Usuario: <strong className="text-zinc-300">{user.username}</strong>
          </span>
          <button
            onClick={handleLogout}
            className="px-3 py-1.5 bg-zinc-800 border border-zinc-700 text-zinc-300 rounded text-xs hover:bg-red-900/50 hover:border-red-800 hover:text-red-300 transition-colors cursor-pointer"
          >
            🔒 Cerrar
          </button>
        </div>
      </header>

      {/* NAVIGATION DRAWER */}
      <NavigationDrawer onLogout={handleLogout} />

      {/* CONTENIDO PRINCIPAL */}
      <main className="flex-1 overflow-y-auto p-4 sm:p-10">{children}</main>
    </div>
  );
}
