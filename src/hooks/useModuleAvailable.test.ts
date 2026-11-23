// src/hooks/useModuleAvailable.test.ts
import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import {
  notifyModulesChanged,
  useModuleAvailable,
} from "./useModuleAvailable";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const invokeMock = vi.mocked(invoke);

beforeEach(() => {
  vi.clearAllMocks();
  invokeMock.mockResolvedValue(true);
});

describe("useModuleAvailable", () => {
  it("consulta la disponibilidad al montar", async () => {
    const { result } = renderHook(() => useModuleAvailable("solicitudes"));

    await waitFor(() => expect(result.current).toBe(true));
    expect(invokeMock).toHaveBeenCalledWith("is_module_available", {
      moduleId: "solicitudes",
    });
  });

  it("revalida cuando el panel de admin activa o desactiva un módulo", async () => {
    invokeMock.mockResolvedValueOnce(false).mockResolvedValueOnce(true);
    const { result } = renderHook(() => useModuleAvailable("solicitudes"));
    await waitFor(() => expect(result.current).toBe(false));

    act(() => {
      notifyModulesChanged();
    });

    await waitFor(() => expect(result.current).toBe(true));
    expect(invokeMock).toHaveBeenCalledTimes(2);
  });

  it("revalida al enfocar la ventana", async () => {
    invokeMock.mockResolvedValueOnce(true).mockResolvedValueOnce(false);
    const { result } = renderHook(() => useModuleAvailable("solicitudes"));
    await waitFor(() => expect(result.current).toBe(true));

    act(() => {
      window.dispatchEvent(new Event("focus"));
    });

    await waitFor(() => expect(result.current).toBe(false));
  });

  it("trata un error de IPC como 'no disponible'", async () => {
    invokeMock.mockRejectedValueOnce(new Error("IPC caído"));
    const { result } = renderHook(() => useModuleAvailable("solicitudes"));

    await waitFor(() => expect(result.current).toBe(false));
  });
});
