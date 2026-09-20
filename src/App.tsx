import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AuthBox } from "./components/AuthBox";
import { DashboardLayout } from "./components/layouts/DashboardLayout";
import { AdminView } from "./components/views/AdminView";
import { MedicoView } from "./components/views/MedicoView";
import { PacienteView } from "./components/views/PacienteView";

import { useAuthStore } from "./stores/useAuthStore";

export default function App() {
  const { activeUser, isVaultUnlocked, lockVault } = useAuthStore();

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

  if (!activeUser || !isVaultUnlocked) {
    return <AuthBox />;
  }

  return (
    <DashboardLayout>
      {activeUser.role === "admin" && <AdminView/>}

      {activeUser.role === "medico" && <MedicoView/>}

      {activeUser.role === "paciente" && <PacienteView/>}
    </DashboardLayout>
  );
}
