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
  created_at: string;
}

interface NoteState {
  notes: Note[];
  selectedNote: Note | null;
  isLoading: boolean;
  error: string | null;

  fetchNotes: (entityId: number) => Promise<void>;
  createNote: (
    entityId: number,
    templateId: string,
    fields: Record<string, string>,
    userId: string,
  ) => Promise<Note | null>;
  selectNote: (note: Note | null) => void;
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
      const res = await invoke<Note[]>("get_notes_by_entity", {
        entityId,
      });
      set({ notes: res, isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  createNote: async (entityId, templateId, fields, userId) => {
    set({ isLoading: true, error: null });
    try {
      // 1. Crear la nota, devuelve el ID
      const noteId = await invoke<number>("create_note", {
        entityId,
        templateId,
        fields,
        userId,
      });

      // 2. Obtener la nota completa recién creada
      const note = await invoke<Note>("get_note", {
        id: noteId,
      });

      // 3. Agregar a la lista local (Optimistic UI)
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

  selectNote: (note) => {
    set({ selectedNote: note });
  },

  clearError: () => set({ error: null }),
}));
