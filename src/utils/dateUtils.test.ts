// src/utils/dateUtils.test.ts
//
// El "hoy" se congela con fake timers en una fecha lejana (2030-05-01):
// si el fake fallara, los asserts de edad darían otra cosa y el test
// fallaría de forma ruidosa en vez de ir pudriendo con el paso del tiempo.
//
// Criterio timezone-safe: las fechas de nacimiento usan meses distintos al
// mes del "hoy" fake (o un día lejano dentro del mismo mes), porque
// `new Date("YYYY-MM-DD")` parsea en UTC y los componentes locales pueden
// correr un día hacia atrás (ej: UTC-3).

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { calculateAge, formatDateTime } from "./dateUtils";

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2030, 4, 1)); // 1 May 2030, hora local
});

afterEach(() => {
  vi.useRealTimers();
});

describe("calculateAge", () => {
  it("devuelve null con fecha vacía o ausente", () => {
    expect(calculateAge()).toBeNull();
    expect(calculateAge("")).toBeNull();
  });

  it("devuelve null con fecha inválida", () => {
    expect(calculateAge("no-es-fecha")).toBeNull();
  });

  it("edad con cumpleaños ya pasado en el año", () => {
    expect(calculateAge("1996-02-20")).toBe(34);
  });

  it("edad con cumpleaños aún no cumplido (mismo mes, día posterior)", () => {
    expect(calculateAge("1996-05-15")).toBe(33);
  });

  it("edad con mes posterior al actual", () => {
    expect(calculateAge("1996-06-20")).toBe(33);
  });
});

describe("formatDateTime", () => {
  it("devuelve em dash con fecha ausente o vacía", () => {
    expect(formatDateTime()).toBe("—");
    expect(formatDateTime("")).toBe("—");
    expect(formatDateTime(null)).toBe("—");
  });

  it("devuelve em dash con fecha inválida", () => {
    expect(formatDateTime("garbage")).toBe("—");
  });

  it("formatea un timestamp ISO en es-AR (dd/mm/yyyy, hh:mm)", () => {
    // Mediodía UTC: mismo día en UTC (CI) y en UTC-3 (hora local);
    // la hora del assert es variable, por eso el regex.
    expect(formatDateTime("2026-07-15T12:00:00Z")).toMatch(
      /^15\/07\/2026, \d{2}:\d{2} [ap]\. m\.$/,
    );
  });
});
