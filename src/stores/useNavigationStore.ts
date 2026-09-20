// src/stores/useNavigationStore.ts
import { create } from "zustand";

export type ViewKey = "admin" | "medico" | "paciente";

export type MedicoTab =
  | "perfil"
  | "paciente"
  | "antecedentes"
  | "ehr";

interface NavigationState {
  isDrawerOpen: boolean;
  activeView: ViewKey;
  activeMedicoTab: MedicoTab;
  toggleDrawer: () => void;
  openDrawer: () => void;
  closeDrawer: () => void;
  setActiveView: (view: ViewKey) => void;
  setActiveMedicoTab: (tab: MedicoTab) => void;
}

export const useNavigationStore = create<NavigationState>((set) => ({
  isDrawerOpen: false,
  activeView: "medico",
  activeMedicoTab: "paciente",

  toggleDrawer: () => set((state) => ({ isDrawerOpen: !state.isDrawerOpen })),
  openDrawer: () => set({ isDrawerOpen: true }),
  closeDrawer: () => set({ isDrawerOpen: false }),
  setActiveView: (view) => set({ activeView: view }),
  setActiveMedicoTab: (tab) => set({ activeMedicoTab: tab }),
}));
