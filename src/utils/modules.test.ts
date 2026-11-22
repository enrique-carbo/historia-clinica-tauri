import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  activateModule,
  deactivateModule,
  isModuleAvailable,
  listModules,
  moduleExecute,
} from "./modules";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const invokeMock = vi.mocked(invoke);

describe("cliente de módulos", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("moduleExecute manda el contrato por module_execute", async () => {
    invokeMock.mockResolvedValue({ payload: { ok: true } });

    const result = await moduleExecute<{ ok: boolean }>(
      "solicitudes",
      "solicitud.list",
      { filtro: 1 },
      7,
    );

    expect(result).toEqual({ ok: true });
    expect(invokeMock).toHaveBeenCalledWith("module_execute", {
      moduleId: "solicitudes",
      req: {
        protocol_version: 1,
        action: "solicitud.list",
        entity_id: 7,
        payload: { filtro: 1 },
      },
    });
  });

  it("usa entity_id null y payload vacío por defecto", async () => {
    invokeMock.mockResolvedValue({ payload: [] });

    await moduleExecute("solicitudes", "template.list");

    expect(invokeMock).toHaveBeenCalledWith("module_execute", {
      moduleId: "solicitudes",
      req: {
        protocol_version: 1,
        action: "template.list",
        entity_id: null,
        payload: {},
      },
    });
  });

  it("isModuleAvailable consulta el gate", async () => {
    invokeMock.mockResolvedValue(true);

    await expect(isModuleAvailable("solicitudes")).resolves.toBe(true);
    expect(invokeMock).toHaveBeenCalledWith("is_module_available", {
      moduleId: "solicitudes",
    });
  });

  it("listModules, activateModule y deactivateModule invocan los commands del host", async () => {
    invokeMock.mockResolvedValueOnce([
      {
        id: "solicitudes",
        name: "Solicitud de estudios complementarios",
        version: "0.1.0",
        scope: "Installation",
        allowed_roles: ["Profesional"],
        state: "inactive",
      },
    ]);
    const list = await listModules();
    expect(list).toHaveLength(1);
    expect(invokeMock).toHaveBeenCalledWith("list_modules");

    invokeMock.mockResolvedValueOnce(undefined);
    await activateModule("solicitudes");
    expect(invokeMock).toHaveBeenCalledWith("module_activate", {
      moduleId: "solicitudes",
    });

    invokeMock.mockResolvedValueOnce(undefined);
    await deactivateModule("solicitudes");
    expect(invokeMock).toHaveBeenCalledWith("module_deactivate", {
      moduleId: "solicitudes",
    });
  });
});
