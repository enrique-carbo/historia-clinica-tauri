// src/components/ui/Input.test.tsx

import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { Input } from "./Input";
import { Textarea } from "./Textarea";

describe("Input", () => {
  it("asocia el label al input vía htmlFor/id (consultable por getByLabelText)", () => {
    render(<Input label="Usuario" placeholder="ej: dra.perez" />);

    const input = screen.getByLabelText("Usuario");
    expect(input).toBeInTheDocument();
    expect(input).toHaveAttribute("placeholder", "ej: dra.perez");
  });

  it("respeta un id externo en lugar del auto-generado", () => {
    render(<Input label="Rol" id="new-user-role" />);

    expect(screen.getByLabelText("Rol")).toHaveAttribute("id", "new-user-role");
  });

  it("sin label renderiza solo el input (sin labels huérfanos)", () => {
    render(<Input aria-label="DNI" />);

    expect(screen.getByLabelText("DNI")).toBeInTheDocument();
    expect(document.querySelector("label")).toBeNull();
  });
});

describe("Textarea", () => {
  it("asocia el label al textarea vía htmlFor/id", () => {
    render(<Textarea label="Motivo de consulta" />);

    expect(screen.getByLabelText("Motivo de consulta")).toBeInTheDocument();
  });
});
