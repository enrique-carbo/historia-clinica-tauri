// src/stores/usePatientRegistryStore.ts
import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

// 🚀 Agregamos created_at para hacer match con tu struct de Rust
export interface RegisteredPatient {
  id: string;
  full_name: string;
  created_at: string;
}

interface RegistryState {
  patients: RegisteredPatient[];
  isLoading: boolean;
  fetchPatients: () => Promise<void>;
  addPatientLocally: (newPatient: RegisteredPatient) => void;
}

export const usePatientRegistryStore = create<RegistryState>((set) => ({
  patients: [],
  isLoading: false,

  fetchPatients: async () => {
    set({ isLoading: true });
    try {
      const res = await invoke<RegisteredPatient[]>("get_patients_list");
      set({ patients: res, isLoading: false });
    } catch (err) {
      console.error("Error al obtener padrón:", err);
      set({ isLoading: false });
    }
  },

  addPatientLocally: (newPatient) => {
    set((state) => ({
      patients: [newPatient, ...state.patients],
    }));
  },
}));
