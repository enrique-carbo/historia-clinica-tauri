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

  it("lista solicitudes, emite una nueva con checklist y cierra el form si cambia entityId", async () => {
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
                study_name: "Laboratorio general",
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
                name: "Laboratorio general",
                items: ["Hemograma completo", "Glucemia"],
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
    expect(await screen.findByText("Laboratorio general")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Nueva" }));
    await user.selectOptions(screen.getByLabelText("Template"), "tpl-1");
    await user.click(
      screen.getByRole("checkbox", { name: "Hemograma completo" }),
    );
    await user.type(
      screen.getByLabelText("Diagnóstico / indicación"),
      "Control anual",
    );
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
            items: ["Hemograma completo"],
            diagnostico: "Control anual",
            notas: "",
          },
        },
      });
    });

    // Si abrimos de nuevo el form y cambia el paciente (entityId), se cierra solo.
    await user.click(screen.getByRole("button", { name: "Nueva" }));
    expect(screen.getByLabelText("Template")).toBeInTheDocument();

    rerender(<SolicitudesPanel entityId={2} />);
    await waitFor(() => {
      expect(screen.queryByLabelText("Template")).not.toBeInTheDocument();
    });
  });

  it("colapsa y expande el contenido desde el encabezado", async () => {
    const user = userEvent.setup();

    invokeMock.mockImplementation((command: string, args?: unknown) => {
      if (command === "is_module_available") return Promise.resolve(true);
      if (command === "module_execute") {
        const req = (args as { req: { action: string } }).req;
        if (req.action === "solicitud.list") return Promise.resolve({ payload: [] });
        if (req.action === "template.list") return Promise.resolve({ payload: [] });
      }
      return Promise.resolve(null);
    });

    render(<SolicitudesPanel entityId={1} />);
    await screen.findByText("Solicitudes de estudios");

    const toggle = screen.getByRole("button", {
      name: /Solicitudes de estudios/i,
    });
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(
      await screen.findByText("Sin solicitudes registradas"),
    ).toBeInTheDocument();

    await user.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(
      screen.queryByText("Sin solicitudes registradas"),
    ).not.toBeInTheDocument();

    await user.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(
      screen.getByText("Sin solicitudes registradas"),
    ).toBeInTheDocument();
  });

  it("tilda el único estudio del template y respeta el orden canónico", async () => {
    const user = userEvent.setup();

    invokeMock.mockImplementation((command: string, args?: unknown) => {
      if (command === "is_module_available") return Promise.resolve(true);
      if (command === "module_execute") {
        const req = (args as { req: { action: string } }).req;
        if (req.action === "solicitud.list") return Promise.resolve({ payload: [] });
        if (req.action === "template.list") {
          return Promise.resolve({
            payload: [
              {
                id: "tpl-single",
                name: "Radiografía tórax",
                items: ["Radiografía de tórax frente y perfil"],
                created_at: "2026-11-22T10:00:00Z",
                updated_at: "2026-11-22T10:00:00Z",
              },
              {
                id: "tpl-lab",
                name: "Laboratorio general",
                items: ["Hemograma completo", "Glucemia"],
                created_at: "2026-11-22T10:00:00Z",
                updated_at: "2026-11-22T10:00:00Z",
              },
            ],
          });
        }
        if (req.action === "solicitud.create") {
          return Promise.resolve({ payload: { id: "sol-3" } });
        }
      }
      return Promise.resolve(null);
    });

    render(<SolicitudesPanel entityId={1} />);
    await screen.findByText("Solicitudes de estudios");
    await user.click(screen.getByRole("button", { name: "Nueva" }));

    // Template de un solo estudio: viene tildado.
    await user.selectOptions(
      screen.getByLabelText("Template"),
      "tpl-single",
    );
    expect(
      screen.getByRole("checkbox", {
        name: "Radiografía de tórax frente y perfil",
      }),
    ).toBeChecked();

    // Tildar en orden inverso al del template no cambia el orden emitido.
    await user.selectOptions(screen.getByLabelText("Template"), "tpl-lab");
    await user.click(screen.getByRole("checkbox", { name: "Glucemia" }));
    await user.click(
      screen.getByRole("checkbox", { name: "Hemograma completo" }),
    );
    await user.click(screen.getByRole("button", { name: "Emitir solicitud" }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("module_execute", {
        moduleId: "solicitudes",
        req: {
          protocol_version: 1,
          action: "solicitud.create",
          entity_id: 1,
          payload: {
            template_id: "tpl-lab",
            study_name: null,
            items: ["Hemograma completo", "Glucemia"],
            diagnostico: "",
            notas: "",
          },
        },
      });
    });
  });
});
