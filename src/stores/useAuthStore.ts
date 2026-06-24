// src/stores/useAuthStore.ts
import { create } from "zustand";

interface User {
  user_id: string;
  username: string;
  role: string;
}

interface AuthState {
  isVaultUnlocked: boolean;
  activeUser: User | null;
  unlockVault: (user: User) => void;
  lockVault: () => void;
}

export const useAuthStore = create<AuthState>((set) => ({
  isVaultUnlocked: false,
  activeUser: null,

  // Ahora recibe el objeto completo del usuario que nos da Rust
  unlockVault: (user: User) => set({ isVaultUnlocked: true, activeUser: user }),
  lockVault: () => set({ isVaultUnlocked: false, activeUser: null }),
}));
