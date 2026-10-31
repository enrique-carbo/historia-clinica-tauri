// src/utils/errors.test.ts
//
// Contrato del backend: `invoke` rechaza con `{ code, params? }` y el texto
// del usuario sale de este diccionario. Los rechazos legacy (string, Error)
// se muestran tal cual para no romper helpers/tests que todavía los usan.

import { describe, expect, it } from "vitest";
import {
  errorCode,
  formatError,
  isErrorCode,
  parseCommandError,
} from "./errors";

describe("formatError", () => {
  it("traduce un código conocido", () => {
    expect(formatError({ code: "ERR_WRONG_PASSWORD" })).toBe(
      "Contraseña incorrecta.",
    );
  });

  it("interpola los parámetros del backend", () => {
    expect(
      formatError({ code: "ERR_PASSWORD_TOO_SHORT", params: { min: "10" } }),
    ).toBe("La contraseña debe tener al menos 10 caracteres.");
  });

  it("usa el detalle técnico como fallback de ERR_INTERNAL", () => {
    expect(
      formatError({
        code: "ERR_INTERNAL",
        params: { detail: "Lock poisoned" },
      }),
    ).toContain("Lock poisoned");
  });

  it("ERR_INTERNAL sin detalle no filtra nada raro", () => {
    expect(formatError({ code: "ERR_INTERNAL" })).toBe(
      "Ocurrió un error inesperado.",
    );
  });

  it("un código desconocido cae al detalle o al propio código", () => {
    expect(formatError({ code: "ERR_FUTURO", params: { detail: "x" } })).toBe("x");
    expect(formatError({ code: "ERR_FUTURO" })).toBe("ERR_FUTURO");
  });

  it("muestra tal cual los rechazos legacy (string y Error)", () => {
    expect(formatError("boom")).toBe("boom");
    expect(formatError(new Error("falló"))).toBe("falló");
  });

  it("no devuelve [object Object] para objetos sin code", () => {
    expect(formatError({ detalle: "algo" })).toBe(
      "Ocurrió un error inesperado.",
    );
  });
});

describe("errorCode / isErrorCode / parseCommandError", () => {
  it("extrae el código y los params", () => {
    expect(parseCommandError({ code: "ERR_X", params: { a: "1" } })).toEqual({
      code: "ERR_X",
      params: { a: "1" },
    });
    expect(errorCode({ code: "ERR_X" })).toBe("ERR_X");
  });

  it("devuelve null para rechazos que no son del backend", () => {
    expect(parseCommandError("boom")).toBeNull();
    expect(parseCommandError(null)).toBeNull();
    expect(errorCode(new Error("x"))).toBeNull();
  });

  it("compara códigos sin mirar el texto traducido", () => {
    expect(isErrorCode({ code: "SEED_REQUIRED" }, "SEED_REQUIRED")).toBe(true);
    expect(isErrorCode("SEED_REQUIRED", "SEED_REQUIRED")).toBe(false);
  });
});
