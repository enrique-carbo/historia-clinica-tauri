// src/components/Entity.test.tsx
//
// Flujo del padrón como administrador: buscar → seleccionar → la ficha de
// solo lectura y el bloque «Acceso del paciente» aparecen a la derecha;
// «Editar» carga el formulario sin perder el bloque. Los asserts usan
// valores de fixture e invariantes de los componentes — no el contenido de
// entity_schema.json.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { Entity } from "./Entity";
import { useAuthStore } from "../stores/useAuthStore";
import { usePatientStore } from "../stores/usePatientStore";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const invokeMock = vi.mocked(invoke);

const PATIENT_ENTITY = {
  id: 7,
  entity_type: "patient",
  data: { nombre: "Carla", apellido: "Gómez", dni: "30111222" },
  created_at: "2026-01-01T00:00:00Z",
};

beforeEach(() => {
  vi.clearAllMocks();
  invokeMock.mockImplementation(async (cmd: string) => {
    switch (cmd) {
      case "search_entities":
        return [PATIENT_ENTITY];
      case "get_entity":
        return PATIENT_ENTITY;
      case "get_entity_access":
        return null;
      default:
        return null;
    }
  });
  useAuthStore.setState({
    isVaultUnlocked: true,
    activeUser: { user_id: "u1", username: "admin", role: "administrador" },
  });
  usePatientStore.setState({ selected: null, isLoading: false, error: null });
});

async function searchAndSelect(
  user: ReturnType<typeof userEvent.setup>,
): Promise<void> {
  await user.type(
    screen.getByPlaceholderText(/buscar por nombre/i),
    "carla",
  );
  await user.click(
    screen.getByRole("button", { name: /buscar por nombre o teléfono/i }),
  );
  await user.click(await screen.findByRole("button", { name: /carla gómez/i }));
}

describe("Entity — panel derecho (admin)", () => {
  it("sin selección no muestra el bloque de Acceso del paciente", () => {
    render(<Entity />);
    expect(
      screen.queryByRole("heading", { name: /acceso del paciente/i }),
    ).not.toBeInTheDocument();
  });

  it("seleccionar un paciente muestra la ficha y el bloque de Acceso", async () => {
    const user = userEvent.setup();
    render(<Entity />);

    await searchAndSelect(user);

    // Ficha de solo lectura a la derecha (detalle con valores y "—").
    expect(
      screen.getByRole("heading", { name: /ficha de/i }),
    ).toBeInTheDocument();
    expect(screen.getAllByText("30111222").length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText("—").length).toBeGreaterThan(0);
    // Bloque de acceso visible para el admin sobre la ficha seleccionada.
    expect(
      screen.getByRole("heading", { name: /acceso del paciente/i }),
    ).toBeInTheDocument();
  });

  it("«Editar» carga el formulario y el bloque de Acceso sigue visible", async () => {
    const user = userEvent.setup();
    render(<Entity />);

    await searchAndSelect(user);
    await user.click(screen.getByRole("button", { name: /editar/i }));

    // Modo edición: el form reemplaza a la ficha…
    expect(
      screen.getByRole("button", { name: /guardar cambios/i }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("heading", { name: /ficha de/i }),
    ).not.toBeInTheDocument();
    // …y el bloque de acceso no desaparece.
    expect(
      screen.getByRole("heading", { name: /acceso del paciente/i }),
    ).toBeInTheDocument();
  });
});
