// src/stores/usePatientStore.test.ts

import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { usePatientStore } from "./usePatientStore";
import type { Entity } from "./useEntityStore";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const invokeMock = vi.mocked(invoke);

const PATIENT: Entity = {
  id: 42,
  entity_type: "patient",
  data: { nombre: "Carla" },
  created_at: "2026-01-01T00:00:00Z",
};

beforeEach(() => {
  vi.clearAllMocks();
  usePatientStore.setState({ selected: null, isLoading: false, error: null });
});

describe("selectPatient", () => {
  it("carga el paciente y lo selecciona", async () => {
    invokeMock.mockResolvedValueOnce(PATIENT);
    await usePatientStore.getState().selectPatient(42);

    expect(invokeMock).toHaveBeenCalledWith("get_entity", { id: 42 });
    const s = usePatientStore.getState();
    expect(s.selected).toEqual({ patient: PATIENT });
    expect(s.isLoading).toBe(false);
    expect(s.error).toBeNull();
  });

  it("error del backend deja selected en null y expone el mensaje", async () => {
    invokeMock.mockRejectedValueOnce("paciente inexistente");
    await usePatientStore.getState().selectPatient(1);

    const s = usePatientStore.getState();
    expect(s.selected).toBeNull();
    expect(s.isLoading).toBe(false);
    expect(s.error).toBe("paciente inexistente");
  });
});

describe("clearSelection", () => {
  it("limpia selección y error", async () => {
    invokeMock.mockResolvedValueOnce(PATIENT);
    await usePatientStore.getState().selectPatient(42);
    usePatientStore.getState().clearSelection();

    const s = usePatientStore.getState();
    expect(s.selected).toBeNull();
    expect(s.error).toBeNull();
  });
});
