// src/stores/usePatientStore.ts
import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { useAuthStore } from "./useAuthStore";
import type { Entity } from "./useEntityStore";

export interface ClinicalData {
  grupo_sanguineo?: string;
  antecedentes_personales?: string;
  antecedentes_familiares?: string;
  alergias?: string;
  medicacion?: string;
}

interface PatientWithClinical {
  patient: Entity;
  clinical: ClinicalData | null;
}

interface PatientStoreState {
  selected: PatientWithClinical | null;
  isLoading: boolean;
  error: string | null;

  selectPatient: (entityId: number) => Promise<void>;
  saveClinical: (entityId: number, data: ClinicalData) => Promise<void>;
  clearSelection: () => void;
}

export const usePatientStore = create<PatientStoreState>((set) => ({
  selected: null,
  isLoading: false,
  error: null,

  selectPatient: async (entityId) => {
    set({ isLoading: true, error: null });
    try {
      const [patient, clinicalRecord] = await Promise.all([
        invoke<Entity>("get_entity", { id: entityId }),
        invoke<{ data: ClinicalData } | null>("get_medical_history", { entityId }),
      ]);
      set({
        selected: {
          patient,
          clinical: clinicalRecord?.data ?? null,
        },
        isLoading: false,
      });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  saveClinical: async (entityId, data) => {
    const { activeUser } = useAuthStore.getState();
    if (!activeUser) return;

    // Optimistic update inmediato en todos los consumidores
    set((state) => ({
      selected: state.selected
        ? { ...state.selected, clinical: data }
        : state.selected,
    }));

    try {
      await invoke("upsert_medical_history", {
        entityId,
        data,
        userId: activeUser.user_id,
      });
    } catch (err) {
      set({ error: String(err) });
    }
  },

  clearSelection: () => set({ selected: null, error: null }),
}));
