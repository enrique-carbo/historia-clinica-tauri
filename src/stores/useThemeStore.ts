// src/stores/useThemeStore.ts
// Tema de la UI (Fase C, DESIGN.md §12): "light" | "dark". La clase
// `.dark` vive en <html>; la preferencia persiste en localStorage y el
// arranque respeta prefers-color-scheme. index.html aplica el tema antes
// del primer paint (mismo código) para evitar flash.
import { create } from "zustand";

export type Theme = "light" | "dark";

export const THEME_STORAGE_KEY = "fichio-theme";

function applyTheme(theme: Theme) {
  document.documentElement.classList.toggle("dark", theme === "dark");
  document.documentElement.style.colorScheme = theme;
}

function systemTheme(): Theme {
  if (typeof window.matchMedia !== "function") return "dark";
  return window.matchMedia("(prefers-color-scheme: light)").matches
    ? "light"
    : "dark";
}

export function readStoredTheme(): Theme {
  try {
    const stored = localStorage.getItem(THEME_STORAGE_KEY);
    if (stored === "light" || stored === "dark") return stored;
  } catch {
    // localStorage no disponible → fallback al sistema
  }
  return systemTheme();
}

interface ThemeStoreState {
  theme: Theme;
  setTheme: (theme: Theme) => void;
  toggleTheme: () => void;
}

const initialTheme = readStoredTheme();
applyTheme(initialTheme);

export const useThemeStore = create<ThemeStoreState>((set, get) => ({
  theme: initialTheme,

  setTheme: (theme) => {
    try {
      localStorage.setItem(THEME_STORAGE_KEY, theme);
    } catch {
      // sin persistencia si el storage no está disponible
    }
    applyTheme(theme);
    set({ theme });
  },

  toggleTheme: () => {
    get().setTheme(get().theme === "dark" ? "light" : "dark");
  },
}));
