// src/stores/useThemeStore.test.ts

import { beforeEach, describe, expect, it } from "vitest";
import {
  useThemeStore,
  readStoredTheme,
  THEME_STORAGE_KEY,
} from "./useThemeStore";

beforeEach(() => {
  localStorage.removeItem(THEME_STORAGE_KEY);
  document.documentElement.classList.remove("dark");
  useThemeStore.getState().setTheme("dark");
  localStorage.removeItem(THEME_STORAGE_KEY);
});

describe("useThemeStore", () => {
  it("setTheme aplica la clase .dark y color-scheme en <html>", () => {
    useThemeStore.getState().setTheme("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    expect(document.documentElement.style.colorScheme).toBe("dark");

    useThemeStore.getState().setTheme("light");
    expect(document.documentElement.classList.contains("dark")).toBe(false);
    expect(document.documentElement.style.colorScheme).toBe("light");
  });

  it("setTheme persiste la preferencia en localStorage", () => {
    useThemeStore.getState().setTheme("light");
    expect(localStorage.getItem(THEME_STORAGE_KEY)).toBe("light");

    useThemeStore.getState().setTheme("dark");
    expect(localStorage.getItem(THEME_STORAGE_KEY)).toBe("dark");
  });

  it("toggleTheme alterna entre light y dark", () => {
    useThemeStore.getState().setTheme("dark");
    useThemeStore.getState().toggleTheme();
    expect(useThemeStore.getState().theme).toBe("light");

    useThemeStore.getState().toggleTheme();
    expect(useThemeStore.getState().theme).toBe("dark");
  });

  it("readStoredTheme valida valores guardados inválidos", () => {
    localStorage.setItem(THEME_STORAGE_KEY, "banana");
    // Inválido → fallback al sistema (jsdom sin matchMedia → "dark")
    expect(readStoredTheme()).toBe("dark");

    localStorage.setItem(THEME_STORAGE_KEY, "light");
    expect(readStoredTheme()).toBe("light");
  });
});
