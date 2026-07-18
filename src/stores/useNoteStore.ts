// src/stores/useNoteStore.ts
import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export interface Note {
  id: number;
  external_id?: string;
  entity_id: number;
  template_id: string;
  fields: Record<string, string>;
  is_verified: boolean;
  created_by_name: string;
  created_at: string;
}

interface NoteState {
  notes: Note[];
  selectedNote: Note | null;
  isLoading: boolean;
  error: string | null;

  // Acciones
  fetchNotes: (entityId: number) => Promise<void>;
  createNote: (
    entityId: number,
    templateId: string,
    fields: Record<string, string>,
    userId: string,
  ) => Promise<Note | null>;
  selectNote: (note: Note | null) => void;
  clearSelection: () => void;
  clearError: () => void;
}

export const useNoteStore = create<NoteState>((set) => ({
  notes: [],
  selectedNote: null,
  isLoading: false,
  error: null,

  fetchNotes: async (entityId: number) => {
    set({ isLoading: true, error: null });
    try {
      const res = await invoke<Note[]>("get_notes_by_entity", { entityId });
      // Ordenamos por fecha descendente por defecto
      const sorted = res.sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime());
      set({ notes: sorted, isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  createNote: async (entityId, templateId, fields, userId) => {
    set({ isLoading: true, error: null });
    try {
      // 1. Crear la nota (Rust devuelve el ID)
      const noteId = await invoke<number>("create_note", {
        entityId,
        templateId,
        fields,
        userId,
      });

      // 2. Obtener la nota completa con su firma verificada
      const note = await invoke<Note>("get_note", { id: noteId });

      // 3. Optimistic UI: Agregar al principio de la lista
      set((state) => ({
        notes: [note, ...state.notes],
        isLoading: false,
      }));

      return note;
    } catch (err) {
      set({ error: String(err), isLoading: false });
      return null;
    }
  },

  selectNote: (note) => set({ selectedNote: note }),

  clearSelection: () => set({ selectedNote: null }),

  clearError: () => set({ error: null }),
}));
