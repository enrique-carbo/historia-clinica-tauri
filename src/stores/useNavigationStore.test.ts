// src/stores/useNavigationStore.test.ts

import { beforeEach, describe, expect, it } from "vitest";
import { useNavigationStore } from "./useNavigationStore";

beforeEach(() => {
  useNavigationStore.setState({
    isDrawerOpen: false,
    activeView: "profesional",
    activeMedicoTab: "paciente",
    activeAdminTab: "usuarios",
  });
});

describe("useNavigationStore", () => {
  it("el drawer arranca cerrado y alterna con toggle", () => {
    expect(useNavigationStore.getState().isDrawerOpen).toBe(false);
    useNavigationStore.getState().toggleDrawer();
    expect(useNavigationStore.getState().isDrawerOpen).toBe(true);
    useNavigationStore.getState().toggleDrawer();
    expect(useNavigationStore.getState().isDrawerOpen).toBe(false);
  });

  it("open/close son idempotentes", () => {
    useNavigationStore.getState().openDrawer();
    useNavigationStore.getState().openDrawer();
    expect(useNavigationStore.getState().isDrawerOpen).toBe(true);
    useNavigationStore.getState().closeDrawer();
    useNavigationStore.getState().closeDrawer();
    expect(useNavigationStore.getState().isDrawerOpen).toBe(false);
  });

  it("setters de vista y tabs", () => {
    useNavigationStore.getState().setActiveView("administrador");
    useNavigationStore.getState().setActiveAdminTab("auditoria");
    useNavigationStore.getState().setActiveMedicoTab("ehr");

    const s = useNavigationStore.getState();
    expect(s.activeView).toBe("administrador");
    expect(s.activeAdminTab).toBe("auditoria");
    expect(s.activeMedicoTab).toBe("ehr");
  });
});
