// src/stores/useEntryStore.ts
import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export type EntryCategory = "SOAP_NOTE" | "MEDICATION" | "ALLERGY" | "CONDITION";
export type EntryStatus = "ACTIVE" | "RESOLVED" | "COMPLETED";

export interface Entry {
  id: string;
  category: EntryCategory;
  subject_id: number;
  author_id: string;
  author_name: string;
  title: string;
  status: EntryStatus;
  timestamp: string;
  payload: Record<string, string>;
  signature: string;
  is_verified: boolean;
  created_at: string;
}

interface EntryState {
  entries: Entry[];
  selectedEntry: Entry | null;
  isLoading: boolean;
  error: string | null;

  // Paginación
  page: number;
  pageSize: number;
  hasMore: boolean;

  // Acciones
  fetchEntries: (subjectId: number, category?: EntryCategory) => Promise<void>;
  searchEntries: (subjectId: number, query: string, category?: EntryCategory) => Promise<void>;
  loadMore: (subjectId: number, category?: EntryCategory, query?: string) => Promise<void>;
  setPage: (page: number) => void;
  createEntry: (params: {
    category: EntryCategory;
    subjectId: number;
    title: string;
    status: EntryStatus;
    payload: Record<string, string>;
  }) => Promise<Entry | null>;
  selectEntry: (entry: Entry | null) => void;
  clearSelection: () => void;
  clearError: () => void;
}

export const useEntryStore = create<EntryState>((set, get) => ({
  entries: [],
  selectedEntry: null,
  isLoading: false,
  error: null,
  page: 0,
  pageSize: 20,
  hasMore: true,

  fetchEntries: async (subjectId: number, category?: EntryCategory) => {
    const { pageSize } = get();
    set({ isLoading: true, error: null, page: 0, hasMore: true, entries: [] });
    try {
      const res = category
        ? await invoke<Entry[]>("get_entries_by_category", {
            subjectId,
            category,
            limit: pageSize,
            offset: 0,
          })
        : await invoke<Entry[]>("get_entries_by_subject", {
            subjectId,
            limit: pageSize,
            offset: 0,
          });

      set({
        entries: res,
        isLoading: false,
        page: 1,
        hasMore: res.length === pageSize,
      });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  searchEntries: async (subjectId: number, query: string, category?: EntryCategory) => {
    const { pageSize } = get();
    set({ isLoading: true, error: null, page: 0, hasMore: true, entries: [] });
    try {
      const res = await invoke<Entry[]>("search_entries", {
        subjectId,
        query,
        category: category ?? null,
        limit: pageSize,
        offset: 0,
      });

      set({
        entries: res,
        isLoading: false,
        page: 1,
        hasMore: res.length === pageSize,
      });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  loadMore: async (subjectId: number, category?: EntryCategory, query?: string) => {
    const { page, pageSize, entries, hasMore, isLoading } = get();
    if (isLoading || !hasMore) return;

    set({ isLoading: true, error: null });
    try {
      const offset = page * pageSize;
      let res: Entry[];

      if (query && query.trim()) {
        res = await invoke<Entry[]>("search_entries", {
          subjectId,
          query,
          category: category ?? null,
          limit: pageSize,
          offset,
        });
      } else if (category) {
        res = await invoke<Entry[]>("get_entries_by_category", {
          subjectId,
          category,
          limit: pageSize,
          offset,
        });
      } else {
        res = await invoke<Entry[]>("get_entries_by_subject", {
          subjectId,
          limit: pageSize,
          offset,
        });
      }

      set({
        entries: [...entries, ...res],
        isLoading: false,
        page: page + 1,
        hasMore: res.length === pageSize,
      });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  setPage: (page: number) => set({ page }),

  createEntry: async ({ category, subjectId, title, status, payload }) => {
    set({ isLoading: true, error: null });
    try {
      // Obtener user_id del auth store
      const { useAuthStore } = await import("./useAuthStore");
      const { activeUser } = useAuthStore.getState();

      if (!activeUser) {
        set({ error: "Usuario no autenticado", isLoading: false });
        return null;
      }

      const entryId = await invoke<string>("create_entry", {
        category,
        subjectId,
        authorId: activeUser.user_id,
        title,
        status,
        payload,
      });

      const entry = await invoke<Entry>("get_entry", { id: entryId });

      set((state) => ({
        entries: [entry, ...state.entries],
        isLoading: false,
      }));

      return entry;
    } catch (err) {
      set({ error: String(err), isLoading: false });
      return null;
    }
  },

  selectEntry: (entry) => set({ selectedEntry: entry }),

  clearSelection: () => set({ selectedEntry: null }),

  clearError: () => set({ error: null }),
}));
