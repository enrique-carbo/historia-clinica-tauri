// src/stores/useAuthStore.test.ts

import { beforeEach, describe, expect, it } from "vitest";
import { useAuthStore } from "./useAuthStore";

const USER = { user_id: "u1", username: "dra.ana", role: "medico" };

beforeEach(() => {
  useAuthStore.setState({ isVaultUnlocked: false, activeUser: null });
});

describe("useAuthStore", () => {
  it("arranca con la bóveda cerrada", () => {
    expect(useAuthStore.getState().isVaultUnlocked).toBe(false);
    expect(useAuthStore.getState().activeUser).toBeNull();
  });

  it("unlockVault setea el usuario activo", () => {
    useAuthStore.getState().unlockVault(USER);
    const s = useAuthStore.getState();
    expect(s.isVaultUnlocked).toBe(true);
    expect(s.activeUser).toEqual(USER);
  });

  it("lockVault limpia usuario y estado", () => {
    useAuthStore.getState().unlockVault(USER);
    useAuthStore.getState().lockVault();
    const s = useAuthStore.getState();
    expect(s.isVaultUnlocked).toBe(false);
    expect(s.activeUser).toBeNull();
  });
});
