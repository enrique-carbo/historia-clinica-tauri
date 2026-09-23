// src/stores/useSeedStore.ts
import { create } from "zustand";
import { persist } from "zustand/middleware";

interface SeedState {
  isSeedConfigured: boolean;
  setSeedConfigured: (value: boolean) => void;
}

export const useSeedStore = create<SeedState>()(
  persist(
    (set) => ({
      isSeedConfigured: false,
      setSeedConfigured: (value) => set({ isSeedConfigured: value }),
    }),
    {
      name: "seed-storage",
    }
  )
);
