import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { SolicitudesPanel } from "./SolicitudesPanel";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const invokeMock = vi.mocked(invoke);

describe("SolicitudesPanel", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("no renderiza nada cuando is_module_available devuelve false", async () => {
    invokeMock.mockResolvedValue(false);

    const { container } = render(<SolicitudesPanel entityId={1} />);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("is_module_available", {
        moduleId: "solicitudes",
      });
    });
    expect(container.firstChild).toBeNull();
  });

  it("lista solicitudes, emite una nueva desde template y cierra el form si cambia entityId", async () => {
    const user = userEvent.setup();

    invokeMock.mockImplementation((command: string, args?: unknown) => {
      if (command === "is_module_available") return Promise.resolve(true);
      if (command === "module_execute") {
        const req = (args as { req: { action: string } }).req;
        if (req.action === "solicitud.list") {
          return Promise.resolve({
            payload: [
              {
                id: "sol-1",
                entity_id: 1,
                study_name: "Laboratorio",
                template_id: "tpl-1",
                status: "issued",
                created_at: "2026-11-22T10:00:00Z",
              },
            ],
          });
        }
        if (req.action === "template.list") {
          return Promise.resolve({
            payload: [
              {
                id: "tpl-1",
                name: "Rx tórax",
                fields: [
                  {
                    name: "indicacion",
                    label: "Indicación",
                    type: "textarea",
                    required: true,
                  },
                  {
                    name: "prioridad",
                    label: "Prioridad",
                    type: "select",
                    required: true,
                    options: ["Rutina", "Urgente"],
                  },
                ],
                created_at: "2026-11-22T10:00:00Z",
                updated_at: "2026-11-22T10:00:00Z",
              },
            ],
          });
        }
        if (req.action === "solicitud.create") {
          return Promise.resolve({ payload: { id: "sol-2" } });
        }
      }
      return Promise.resolve(null);
    });

    const { rerender } = render(<SolicitudesPanel entityId={1} />);

    expect(await screen.findByText("Solicitudes de estudios")).toBeInTheDocument();
    expect(await screen.findByText("Laboratorio")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Nueva solicitud" }));
    await user.selectOptions(screen.getByLabelText("Template"), "tpl-1");
    await user.type(screen.getByLabelText("Indicación *"), "Control anual");
    await user.selectOptions(screen.getByLabelText("Prioridad *"), "Rutina");
    await user.click(screen.getByRole("button", { name: "Emitir solicitud" }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("module_execute", {
        moduleId: "solicitudes",
        req: {
          protocol_version: 1,
          action: "solicitud.create",
          entity_id: 1,
          payload: {
            template_id: "tpl-1",
            study_name: null,
            values: { indicacion: "Control anual", prioridad: "Rutina" },
          },
        },
      });
    });

    // Si abrimos de nuevo el form y cambia el paciente (entityId), se cierra solo.
    await user.click(screen.getByRole("button", { name: "Nueva solicitud" }));
    expect(screen.getByLabelText("Template")).toBeInTheDocument();

    rerender(<SolicitudesPanel entityId={2} />);
    await waitFor(() => {
      expect(screen.queryByLabelText("Template")).not.toBeInTheDocument();
    });
  });
});
