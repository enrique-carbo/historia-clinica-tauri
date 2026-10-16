// src/components/EntryForm.test.tsx

import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { EntryForm } from "./EntryForm";
import { usePatientStore } from "../stores/usePatientStore";
import { useEntryStore } from "../stores/useEntryStore";
import type { Entity } from "../stores/useEntityStore";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const invokeMock = vi.mocked(invoke);

const PATIENT: Entity = {
  id: 1,
  entity_type: "patient",
  data: { nombre: "Ana", apellido: "Gómez", dni: "30111222" },
  created_at: "2026-01-01T00:00:00Z",
};

const FULL_PROFILE = {
  full_name: "Dra. Pérez",
  profession: "Médico",
  licenses: [{ number: "MP-12345" }],
};

beforeEach(() => {
  vi.clearAllMocks();
  usePatientStore.setState({ selected: null, isLoading: false, error: null });
  useEntryStore.setState({
    entries: [],
    selectedEntry: null,
    isLoading: false,
    isLoadingMore: false,
    error: null,
  });
});

describe("EntryForm — gates", () => {
  it("sin paciente seleccionado muestra el aviso y no el formulario", async () => {
    invokeMock.mockResolvedValue(FULL_PROFILE);

    render(<EntryForm />);

    expect(
      await screen.findByText(/Seleccioná un paciente primero/),
    ).toBeInTheDocument();
    expect(screen.queryByText("Nueva Entry")).not.toBeInTheDocument();
  });

  it("con perfil profesional incompleto muestra el gate de perfil", async () => {
    usePatientStore.setState({
      selected: { patient: PATIENT },
      isLoading: false,
      error: null,
    });
    invokeMock.mockResolvedValue({ full_name: "", profession: "", licenses: [] });

    render(<EntryForm />);

    expect(
      await screen.findByText(/completar tu perfil profesional/),
    ).toBeInTheDocument();
  });
});

describe("EntryForm — formulario", () => {
  beforeEach(() => {
    usePatientStore.setState({
      selected: { patient: PATIENT },
      isLoading: false,
      error: null,
    });
    invokeMock.mockResolvedValue(FULL_PROFILE);
  });

  it("renderiza el formulario y valida el título requerido al enviar", async () => {
    const user = userEvent.setup();
    render(<EntryForm />);

    expect(await screen.findByText("Nueva Entry")).toBeInTheDocument();

    // Seleccionar la primera categoría disponible (sin acoplar a un template).
    const categoryGroup = screen.getByRole("group", { name: "Categoría" });
    await user.click(within(categoryGroup).getAllByRole("button")[0]);

    await user.click(screen.getByRole("button", { name: "Crear Entry" }));

    expect(await screen.findByText("El título es requerido")).toBeInTheDocument();
  });

  it("todo label del formulario está asociado a su control (invariante a11y)", async () => {
    const user = userEvent.setup();
    render(<EntryForm />);

    const categoryGroup = await screen.findByRole("group", {
      name: "Categoría",
    });
    await user.click(within(categoryGroup).getAllByRole("button")[0]);

    const labels = document.querySelectorAll("label[for]");
    expect(labels.length).toBeGreaterThan(0);
    for (const label of labels) {
      expect(document.getElementById((label as HTMLLabelElement).htmlFor)).not.toBeNull();
    }
  });
});
