// src/components/PatientDetailView.test.tsx
//
// Ficha de solo lectura del paciente seleccionado. Los labels vienen del
// schema; el test usa solo invariantes (valores de fixture, "—" en campos
// vacíos, el botón Editar) para no acoplarse a entity_schema.json.

import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { PatientDetailView } from "./PatientDetailView";

describe("PatientDetailView", () => {
  it("muestra los valores cargados y «—» en los campos vacíos", () => {
    render(
      <PatientDetailView
        patient={{ data: { nombre: "Ana", apellido: "", dni: "30111222" } }}
        onEdit={vi.fn()}
      />,
    );

    expect(screen.getByText("Ana")).toBeInTheDocument();
    expect(screen.getByText("30111222")).toBeInTheDocument();
    // Al menos un campo sin dato (apellido) queda con em dash.
    expect(screen.getAllByText("—").length).toBeGreaterThan(0);
  });

  it("«Editar» invoca onEdit una vez", async () => {
    const user = userEvent.setup();
    const onEdit = vi.fn();
    render(<PatientDetailView patient={{ data: {} }} onEdit={onEdit} />);

    await user.click(screen.getByRole("button", { name: /editar/i }));

    expect(onEdit).toHaveBeenCalledTimes(1);
  });
});
