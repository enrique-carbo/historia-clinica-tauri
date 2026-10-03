// src/stores/useNavigationStore.ts
import { create } from "zustand";

export type ViewKey = "administrador" | "asistente" | "medico" | "enfermeria" | "paciente";

export type MedicoTab =
  | "perfil"
  | "paciente"
  | "ehr"
  | "entries";

export type AdminTab = "usuarios" | "paciente" | "auditoria" | "semilla";

interface NavigationState {
  isDrawerOpen: boolean;
  activeView: ViewKey;
  activeMedicoTab: MedicoTab;
  activeAdminTab: AdminTab;
  toggleDrawer: () => void;
  openDrawer: () => void;
  closeDrawer: () => void;
  setActiveView: (view: ViewKey) => void;
  setActiveMedicoTab: (tab: MedicoTab) => void;
  setActiveAdminTab: (tab: AdminTab) => void;
}

export const useNavigationStore = create<NavigationState>((set) => ({
  isDrawerOpen: false,
  activeView: "medico",
  activeMedicoTab: "paciente",
  activeAdminTab: "usuarios",

  toggleDrawer: () => set((state) => ({ isDrawerOpen: !state.isDrawerOpen })),
  openDrawer: () => set({ isDrawerOpen: true }),
  closeDrawer: () => set({ isDrawerOpen: false }),
  setActiveView: (view) => set({ activeView: view }),
  setActiveMedicoTab: (tab) => set({ activeMedicoTab: tab }),
  setActiveAdminTab: (tab) => set({ activeAdminTab: tab }),
}));
