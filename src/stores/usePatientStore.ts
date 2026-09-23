// src/stores/usePatientStore.ts
import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { Entity } from "./useEntityStore";

interface PatientStoreState {
  selected: { patient: Entity } | null;
  isLoading: boolean;
  error: string | null;

  selectPatient: (entityId: number) => Promise<void>;
  clearSelection: () => void;
}

export const usePatientStore = create<PatientStoreState>((set) => ({
  selected: null,
  isLoading: false,
  error: null,

  selectPatient: async (entityId) => {
    set({ isLoading: true, error: null });
    try {
      const patient = await invoke<Entity>("get_entity", { id: entityId });
      set({
        selected: { patient },
        isLoading: false,
      });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  clearSelection: () => set({ selected: null, error: null }),
}));
