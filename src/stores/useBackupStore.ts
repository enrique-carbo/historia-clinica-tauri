// src/stores/useBackupStore.ts
//
// Estado del último respaldo para el recordatorio. Solo los administradores
// pueden consultarlo (`backup_status` exige rol); para el resto el store
// queda en `null` y la UI no muestra nada.

import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export interface BackupStatus {
  last_backup_at: string | null;
  backup_count: number;
}

interface BackupState {
  status: BackupStatus | null;
  isLoading: boolean;
  load: () => Promise<void>;
}

export const useBackupStore = create<BackupState>((set) => ({
  status: null,
  isLoading: false,

  load: async () => {
    set({ isLoading: true });
    try {
      const status = await invoke<BackupStatus>("backup_status");
      set({ status, isLoading: false });
    } catch {
      // Rol sin permiso, sesión cerrada o DB no inicializada: sin datos no
      // hay recordatorio. No es un error que el usuario deba ver.
      set({ status: null, isLoading: false });
    }
  },
}));
