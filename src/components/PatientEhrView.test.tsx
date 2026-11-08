// src/components/PatientEhrView.test.tsx

import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { PatientEhrView } from "./PatientEhrView";
import { usePatientStore } from "../stores/usePatientStore";
import { useEntityStore, type Entity } from "../stores/useEntityStore";
import { useEntryStore } from "../stores/useEntryStore";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const invokeMock = vi.mocked(invoke);

const PATIENT_A: Entity = {
  id: 1,
  entity_type: "patient",
  data: { nombre: "Ana", apellido: "Gómez", dni: "30111222" },
  created_at: "2026-01-01T00:00:00Z",
};

const PATIENT_B: Entity = {
  id: 2,
  entity_type: "patient",
  data: { nombre: "Bruno", apellido: "Díaz", dni: "31000333" },
  created_at: "2026-01-02T00:00:00Z",
};

const FULL_PROFILE = {
  full_name: "Dra. Pérez",
  profession: "Médico",
  licenses: [{ number: "MP-12345" }],
};

beforeEach(() => {
  vi.clearAllMocks();
  useEntityStore.setState({ entities: [], isLoading: false, error: null });
  usePatientStore.setState({ selected: null, isLoading: false, error: null });
  useEntryStore.setState({
    entries: [],
    selectedEntry: null,
    isLoading: false,
    isLoadingMore: false,
    error: null,
  });

  invokeMock.mockImplementation((command: string) => {
    if (command === "get_my_profile") return Promise.resolve(FULL_PROFILE);
    if (command === "get_entries_by_category") return Promise.resolve([]);
    if (command === "get_entries_by_subject") return Promise.resolve([]);
    return Promise.resolve(null);
  });
});

describe("PatientEhrView — atajo de nota SOAP", () => {
  it("muestra el form de SOAP arriba del historial y lo cierra al cambiar de paciente", async () => {
    const user = userEvent.setup();
    usePatientStore.setState({
      selected: { patient: PATIENT_A },
      isLoading: false,
      error: null,
    });

    render(<PatientEhrView />);

    // El atajo abre el form con la categoría fija (sin selector).
    await user.click(
      await screen.findByRole("button", { name: "Nueva nota SOAP" }),
    );
    expect(await screen.findByLabelText(/Título/)).toBeInTheDocument();
    expect(
      screen.queryByRole("group", { name: "Categoría" }),
    ).not.toBeInTheDocument();

    // Cambiar de paciente cierra el form: evita firmar la nota en la ficha
    // equivocada (EntryForm lee `selected` en el submit).
    act(() => {
      usePatientStore.setState({ selected: { patient: PATIENT_B } });
    });

    expect(screen.queryByLabelText(/Título/)).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Nueva nota SOAP" }),
    ).toBeInTheDocument();

    // Los datos del historial corresponden al paciente nuevo.
    expect(screen.getByText("Bruno Díaz")).toBeInTheDocument();
  });
});
