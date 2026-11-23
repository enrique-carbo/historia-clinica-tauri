// src/components/views/MedicoView.test.tsx

import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { MedicoView } from "./MedicoView";
import { useNavigationStore } from "../../stores/useNavigationStore";
import { usePatientStore } from "../../stores/usePatientStore";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const invokeMock = vi.mocked(invoke);

const PATIENT = {
  id: 7,
  entity_type: "patient",
  data: { nombre: "Ana", apellido: "Gómez", dni: "30111222" },
  created_at: "2026-01-01T00:00:00Z",
};

function mockModule(available: boolean) {
  invokeMock.mockImplementation((command: string, args?: unknown) => {
    if (command === "is_module_available") return Promise.resolve(available);
    if (command === "module_execute") {
      const action = (args as { req: { action: string } }).req.action;
      if (action === "solicitud.list" || action === "template.list") {
        return Promise.resolve({ payload: [] });
      }
    }
    return Promise.resolve(null);
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  useNavigationStore.setState({
    isDrawerOpen: false,
    activeView: "profesional",
    activeMedicoTab: "ehr",
    activeAdminTab: "usuarios",
  });
  usePatientStore.setState({ selected: null, isLoading: false, error: null });
});

describe("MedicoView — tab de solicitudes", () => {
  it("agrega el tab si el módulo está disponible y muestra el panel del paciente", async () => {
    mockModule(true);
    usePatientStore.setState({
      selected: { patient: PATIENT },
      isLoading: false,
      error: null,
    });
    useNavigationStore.setState({ activeMedicoTab: "solicitudes" });

    render(<MedicoView />);

    expect(
      await screen.findByRole("tab", { name: /Solicitudes/ }),
    ).toBeInTheDocument();
    expect(
      await screen.findByText("Solicitudes de estudios"),
    ).toBeInTheDocument();

    const patientBanner = screen.getByLabelText("Paciente seleccionado");
    expect(patientBanner).toHaveTextContent("Paciente: Ana Gómez");
    expect(patientBanner).toHaveTextContent("30111222");
  });

  it("no agrega el tab si el módulo no está disponible", async () => {
    mockModule(false);

    render(<MedicoView />);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("is_module_available", {
        moduleId: "solicitudes",
      });
    });
    expect(
      screen.queryByRole("tab", { name: /Solicitudes/ }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByLabelText("Paciente seleccionado"),
    ).not.toBeInTheDocument();
  });
});
