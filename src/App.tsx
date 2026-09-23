import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AuthBox } from "./components/AuthBox";
import { DashboardLayout } from "./components/layouts/DashboardLayout";
import { AdminView } from "./components/views/AdminView";
import { MedicoView } from "./components/views/MedicoView";
import { PacienteView } from "./components/views/PacienteView";
import { SeedPhraseSetup } from "./components/SeedPhraseSetup";

import { useAuthStore } from "./stores/useAuthStore";
import { useSeedStore } from "./stores/useSeedStore";

export default function App() {
  const { activeUser, isVaultUnlocked, lockVault } = useAuthStore();
  const { isSeedConfigured, setSeedConfigured } = useSeedStore();

  // Sincronizar vault con Rust al iniciar
  useEffect(() => {
    invoke<boolean>("is_vault_unlocked")
      .then((unlocked) => {
        if (!unlocked && isVaultUnlocked) {
          lockVault();
        }
      })
      .catch(() => {
        // is_vault_unlocked no disponible, ignorar
      });
  }, [isVaultUnlocked, lockVault]);

  // 1. Si no hay usuario o vault bloqueado → AuthBox
  if (!activeUser || !isVaultUnlocked) {
    return <AuthBox />;
  }

  // 2. Si vault desbloqueado pero seed no configurada → SeedPhraseSetup
  if (!isSeedConfigured) {
    return <SeedPhraseSetup onComplete={() => setSeedConfigured(true)} />;
  }

  // 3. App principal
  return (
    <DashboardLayout>
      {activeUser.role === "admin" && <AdminView />}

      {activeUser.role === "medico" && <MedicoView />}

      {activeUser.role === "paciente" && <PacienteView />}
    </DashboardLayout>
  );
}
