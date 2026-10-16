import { ReactNode, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAuthStore } from "../../stores/useAuthStore";
import { usePatientStore } from "../../stores/usePatientStore";
import { useEntityStore } from "../../stores/useEntityStore";
import { useNavigationStore } from "../../stores/useNavigationStore";
import { useSeedStore } from "../../stores/useSeedStore";
import { NavigationDrawer } from "../ui/Drawer";
import { SeedPhraseSetup } from "../SeedPhraseSetup";
import { Alert } from "../ui/Alert";
import { Button } from "../ui/Button";
import { useEntryStore } from "../../stores/useEntryStore";

interface DashboardLayoutProps {
  children: ReactNode;
}

export function DashboardLayout({ children }: DashboardLayoutProps) {
  const { activeUser: user, lockVault } = useAuthStore();
  const { openDrawer } = useNavigationStore();
  const { isSeedConfigured, setSeedConfigured } = useSeedStore();
  const [showSeedSetup, setShowSeedSetup] = useState(false);
  const [logoutError, setLogoutError] = useState<string | null>(null);

  const handleLogout = async () => {
    setLogoutError(null);
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

      usePatientStore.setState({
        selected: null,
        isLoading: false,
        error: null,
      });

      useEntryStore.setState({
        entries: [],
        selectedEntry: null,
        isLoading: false,
        isLoadingMore: false,
        error: null,
      });
    } catch (err) {
      setLogoutError(String(err));
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
    <div className="min-h-screen bg-canvas text-text-primary font-sans flex flex-col">
      {/* HEADER */}
      <header className="sticky top-0 z-10 bg-canvas/80 backdrop-blur-sm border-b border-border px-4 sm:px-10 py-4 flex justify-between items-center">
        <div className="flex items-center gap-3">
          {/* Botón Hamburguesa */}
          <button
            onClick={openDrawer}
            className="p-2 rounded-lg bg-surface border border-border text-text-primary hover:text-text-primary hover:bg-surface-raised transition-colors"
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
            <h1 className="text-lg sm:text-xl font-bold text-text-primary m-0">
              <span aria-hidden="true">🩺</span> Fichio Care
            </h1>
            <p className="text-xs text-text-muted mt-0.5 hidden sm:block">
              Historia Clínica — Rol:{" "}
              <strong className="text-info-text uppercase">{user.role}</strong>
            </p>
          </div>
        </div>

        <div className="text-right flex items-center gap-2 sm:gap-4">
          {/* Indicador de seed no configurada */}
          {!isSeedConfigured && (
            <button
              onClick={() => setShowSeedSetup(true)}
              className="px-3 py-1.5 bg-yellow-900/30 border border-yellow-700/50 text-yellow-400 rounded text-xs hover:bg-yellow-900/50 transition-colors cursor-pointer"
              title="Configurar frase semilla"
            >
              <span aria-hidden="true">🔐</span>{" "}
              <span className="hidden sm:inline">Configurar Seed</span>
              <span className="sm:hidden">Seed</span>
            </button>
          )}
          <span className="text-xs sm:text-sm text-text-muted hidden sm:inline">
            Usuario: <strong className="text-text-primary">{user.username}</strong>
          </span>
          <button
            onClick={handleLogout}
            className="px-3 py-1.5 bg-surface-raised border border-border-strong text-text-primary rounded text-xs hover:bg-red-900/50 hover:border-red-800 hover:text-red-300 transition-colors cursor-pointer"
          >
            <span aria-hidden="true">🔒</span> Cerrar
          </button>
        </div>
      </header>

      {logoutError && (
        <Alert variant="error" className="mx-4 mt-3 sm:mx-10">
          <div className="flex justify-between gap-4">
            <span>No se pudo cerrar la bóveda: {logoutError}</span>
            <Button
              size="icon"
              variant="ghost"
              onClick={() => setLogoutError(null)}
              aria-label="Cerrar"
            >
              ✕
            </Button>
          </div>
        </Alert>
      )}

      {/* NAVIGATION DRAWER */}
      <NavigationDrawer onLogout={handleLogout} />

      {/* CONTENIDO PRINCIPAL */}
      <main className="flex-1 overflow-y-auto p-4 sm:p-10">{children}</main>
    </div>
  );
}
