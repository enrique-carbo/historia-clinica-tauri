// src/stores/useEntityStore.ts
// src/stores/useEntityStore.ts
import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { useAuthStore } from "./useAuthStore";

export interface Entity {
  id: number;
  external_id?: string;
  entity_type: string;
  data: Record<string, string>;
  created_at: string;
}

interface EntityState {
  entities: Entity[];
  selectedEntity: Entity | null;
  isLoading: boolean;
  error: string | null;

  fetchEntities: (entityType: string, limit?: number) => Promise<void>;
  searchEntities: (entityType: string, query: string) => Promise<void>;
  selectEntity: (entity: Entity | null) => void;
  createEntity: (
    entityType: string,
    data: Record<string, string>,
  ) => Promise<Entity | null>;
  findByBlindIndex: (entityType: string, dni: string) => Promise<number | null>;
  clearError: () => void;
}

export const useEntityStore = create<EntityState>((set, get) => ({
  entities: [],
  selectedEntity: null,
  isLoading: false,
  error: null,

  fetchEntities: async (entityType: string, limit = 50) => {
    set({ isLoading: true, error: null });
    try {
      const res = await invoke<Entity[]>("list_entities", {
        entityType,
        limit,
        offset: 0,
      });
      set({ entities: res, isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  searchEntities: async (entityType: string, query: string) => {
    if (!query.trim()) {
      get().fetchEntities(entityType);
      return;
    }
    set({ isLoading: true, error: null });
    try {
      const res = await invoke<Entity[]>("search_entities", {
        entityType,
        query,
      });
      set({ entities: res, isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  selectEntity: (entity) => {
    set({ selectedEntity: entity });
  },

  createEntity: async (entityType: string, data: Record<string, string>) => {
    set({ isLoading: true, error: null });
    try {
      // ← NUEVO: Obtener userId del auth store
      const { activeUser } = useAuthStore.getState();
      if (!activeUser) {
        set({ error: "Usuario no autenticado", isLoading: false });
        return null;
      }

      const res = await invoke<Entity>("create_entity", {
        entityType,
        data,
        userId: activeUser.user_id, // ← PASAR userId
      });

      set((state) => ({
        entities: [res, ...state.entities],
        selectedEntity: res,
        isLoading: false,
      }));
      return res;
    } catch (err) {
      set({ error: String(err), isLoading: false });
      return null;
    }
  },

  findByBlindIndex: async (entityType: string, dni: string) => {
    try {
      const id = await invoke<number | null>("find_entity_by_blind_index", {
        entityType,
        dni,
      });
      return id;
    } catch (err) {
      set({ error: String(err) });
      return null;
    }
  },

  clearError: () => set({ error: null }),
}));
