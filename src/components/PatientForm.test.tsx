// src/components/PatientForm.test.tsx
//
// Widget de fecha de nacimiento (compuesto Día/Mes/Año → ISO).
// Fixtures propios: el test no depende del contenido de
// entity_schema.json (rompió la CI una vez).
//
// Sin fake timers: se usan años extremos (1985 / 2099) que son pasado y
// futuro con cualquier reloj. composeIso compara fechas locales contra
// medianoche local — no hay parseo UTC que corra un día.

import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { DatePartsInput, composeIso, splitIsoDate } from "./PatientForm";

const FIELD = {
  name: "fecha_nacimiento",
  label: "Fecha de Nacimiento",
  type: "date",
};

describe("composeIso", () => {
  it("incompleto: sin valor y sin error", () => {
    expect(composeIso({ day: "", month: "", year: "" })).toEqual({
      iso: "",
      error: null,
    });
    expect(composeIso({ day: "21", month: "", year: "1985" })).toEqual({
      iso: "",
      error: null,
    });
    // Año incompleto (el usuario va por la mitad).
    expect(composeIso({ day: "21", month: "03", year: "19" })).toEqual({
      iso: "",
      error: null,
    });
  });

  it("fecha válida: compone a ISO con día y mes paddeados", () => {
    expect(composeIso({ day: "5", month: "5", year: "1985" })).toEqual({
      iso: "1985-05-05",
      error: null,
    });
    expect(composeIso({ day: "21", month: "03", year: "1985" })).toEqual({
      iso: "1985-03-21",
      error: null,
    });
  });

  it("bisiesto: 29 de febrero existe solo en años bisiestos", () => {
    expect(composeIso({ day: "29", month: "02", year: "2024" }).iso).toBe(
      "2024-02-29",
    );
    expect(composeIso({ day: "29", month: "02", year: "2025" })).toEqual({
      iso: "",
      error: "Fecha inválida",
    });
    // 1900 no es bisiesto (divisible por 100, no por 400).
    expect(composeIso({ day: "29", month: "02", year: "1900" }).error).toBe(
      "Fecha inválida",
    );
  });

  it("día fuera del mes: 31 de abril y 31 de febrero no existen", () => {
    expect(composeIso({ day: "31", month: "04", year: "1985" })).toEqual({
      iso: "",
      error: "Fecha inválida",
    });
    expect(composeIso({ day: "31", month: "02", year: "1985" }).error).toBe(
      "Fecha inválida",
    );
    expect(composeIso({ day: "31", month: "12", year: "1985" }).iso).toBe(
      "1985-12-31",
    );
  });

  it("rango de año: menor que 1900 es inválido", () => {
    expect(composeIso({ day: "1", month: "01", year: "1899" })).toEqual({
      iso: "",
      error: "Fecha inválida",
    });
    expect(composeIso({ day: "15", month: "06", year: "1900" }).iso).toBe(
      "1900-06-15",
    );
  });

  it("futura: se rechaza con error propio", () => {
    expect(composeIso({ day: "1", month: "06", year: "2099" })).toEqual({
      iso: "",
      error: "No puede ser futura",
    });
    expect(composeIso({ day: "1", month: "01", year: "1985" }).error).toBeNull();
  });
});

describe("splitIsoDate", () => {
  it("descompone un ISO válido", () => {
    expect(splitIsoDate("1985-03-21")).toEqual({
      year: "1985",
      month: "03",
      day: "21",
    });
  });

  it("vacío o malformado da partes vacías (sin reventar)", () => {
    expect(splitIsoDate("")).toEqual({ day: "", month: "", year: "" });
    expect(splitIsoDate("1985-3-21")).toEqual({ day: "", month: "", year: "" });
    expect(splitIsoDate("no-es-fecha")).toEqual({
      day: "",
      month: "",
      year: "",
    });
  });
});

describe("DatePartsInput", () => {
  it("renderiza los tres campos y puebla desde un ISO existente", () => {
    const onChange = vi.fn();
    render(
      <DatePartsInput field={FIELD} value="1985-03-21" onChange={onChange} />,
    );

    expect(screen.getByLabelText("Fecha de Nacimiento: día")).toHaveValue("21");
    expect(screen.getByLabelText("Fecha de Nacimiento: mes")).toHaveValue("03");
    expect(screen.getByLabelText("Fecha de Nacimiento: año")).toHaveValue(
      "1985",
    );
  });

  it("al completar los tres componentes emite ISO por onChange", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<DatePartsInput field={FIELD} value="" onChange={onChange} />);

    await user.type(screen.getByLabelText("Fecha de Nacimiento: día"), "21");
    await user.selectOptions(screen.getByLabelText("Fecha de Nacimiento: mes"), "03");
    await user.type(screen.getByLabelText("Fecha de Nacimiento: año"), "1985");

    expect(onChange).toHaveBeenLastCalledWith(
      "fecha_nacimiento",
      "1985-03-21",
    );
  });

  it("fecha futura muestra el error y no emite valor", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<DatePartsInput field={FIELD} value="" onChange={onChange} />);

    await user.type(screen.getByLabelText("Fecha de Nacimiento: año"), "2099");
    await user.type(screen.getByLabelText("Fecha de Nacimiento: día"), "1");
    await user.selectOptions(screen.getByLabelText("Fecha de Nacimiento: mes"), "06");

    expect(screen.getByText("No puede ser futura")).toBeInTheDocument();
    expect(onChange).toHaveBeenLastCalledWith("fecha_nacimiento", "");
  });

  it("adopta cambios externos del value (re-sincronización del form)", () => {
    const onChange = vi.fn();
    const { rerender } = render(
      <DatePartsInput field={FIELD} value="" onChange={onChange} />,
    );

    rerender(
      <DatePartsInput field={FIELD} value="1990-01-02" onChange={onChange} />,
    );

    expect(screen.getByLabelText("Fecha de Nacimiento: día")).toHaveValue("02");
    expect(screen.getByLabelText("Fecha de Nacimiento: mes")).toHaveValue("01");
    expect(screen.getByLabelText("Fecha de Nacimiento: año")).toHaveValue(
      "1990",
    );
  });
});
