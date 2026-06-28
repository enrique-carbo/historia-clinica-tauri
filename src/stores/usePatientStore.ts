// src/stores/usePatientStore.ts
import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

interface Patient {
  id: string;
  full_name: string;
}

interface Metric {
  id: string;
  metric_type: string;
  sub_metric: string;
  value_num: number;
  measured_at: string;
}

export interface SoapRecord {
  id: string;
  paciente_id: string;
  medico_id: string;
  subjetivo: string;
  objetivo: string;
  analisis: string;
  plan: string;
  is_synced: boolean;
  created_at: string;
}

interface PatientState {
  activePatient: Patient | null;
  history: SoapRecord[];
  isLoadingHistory: boolean;
  metrics: Metric[];
  resetAllPatientData: () => void;

  // Acciones
  selectPatient: (patient: Patient | null) => void;
  clearPatient: () => void;
  fetchHistory: () => Promise<void>;
  fetchMetrics: () => Promise<void>;
}

export const usePatientStore = create<PatientState>((set, get) => ({
  activePatient: null,
  history: [],
  isLoadingHistory: false,
  metrics: [],

  selectPatient: (patient) => {
    set({ activePatient: patient, history: [], metrics: [] }); // Limpiamos el historial al cambiar de paciente
  },

  clearPatient: () => {
    set({ activePatient: null, history: [] });
  },

  fetchHistory: async () => {
    const { activePatient } = get();

    // Si no hay paciente seleccionado, no hacemos nada
    if (!activePatient) {
      set({ history: [] });
      return;
    }

    set({ isLoadingHistory: true });

    try {
      const res = await invoke<SoapRecord[]>("get_patient_history", {
        pacienteId: activePatient.id,
      });
      set({ history: res, isLoadingHistory: false });
    } catch (err) {
      console.error("Error al leer historial descifrado:", err);
      set({ isLoadingHistory: false });
    }
  },
  fetchMetrics: async () => {
    const { activePatient } = get();
    if (!activePatient) return;

    try {
      const res = await invoke<Metric[]>("get_patient_metrics", {
        pacienteId: activePatient.id,
      });
      set({ metrics: res });
    } catch (err) {
      console.error("Error al leer métricas:", err);
    }
  },
  resetAllPatientData: () => {
    set({
      activePatient: null,
      history: [],
      metrics: [],
    });
  },
}));
