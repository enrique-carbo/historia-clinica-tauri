// src/utils/schema.test.ts
//
// Los tests de lógica usan fixtures sintéticas: NO se acoplan al contenido
// de entity_schema.json (mismo criterio que los tests de config_schema.rs
// en el backend — el schema lo puede editar el usuario sin romper CI).
// Solo hay un test "humo" con invariantes estructurales del schema embebido.

import { describe, expect, it } from "vitest";
import {
  FLAT_PATIENT_FIELDS,
  PATIENT_FIELDS,
  PATIENT_LABEL,
  flattenFields,
  type FieldDef,
} from "./schema";

const FIXTURE: FieldDef[] = [
  { name: "nombre", label: "Nombre", type: "text", required: true },
  {
    name: "responsable",
    label: "Responsable",
    type: "group",
    required: false,
    fields: [
      { name: "nombre_completo", label: "Nombre Completo", type: "text", required: false },
      { name: "telefono", label: "Teléfono", type: "text", required: false },
    ],
  },
  { name: "ciudad", label: "Ciudad", type: "text", required: false },
];

describe("flattenFields", () => {
  it("aplana groups con prefijo `{group}_{campo}` preservando el orden", () => {
    expect(flattenFields(FIXTURE).map((f) => f.name)).toEqual([
      "nombre",
      "responsable_nombre_completo",
      "responsable_telefono",
      "ciudad",
    ]);
  });

  it("los campos planos conservan label/type/required de la hoja", () => {
    const flat = flattenFields(FIXTURE);
    expect(flat[1]).toEqual({
      name: "responsable_nombre_completo",
      label: "Nombre Completo",
      type: "text",
      required: false,
    });
  });

  it("sin prefijo (primer nivel) los nombres no cambian", () => {
    expect(flattenFields(FIXTURE)[0].name).toBe("nombre");
  });

  it("recurre en grupos anidados con prefijo compuesto", () => {
    const nested: FieldDef[] = [
      {
        name: "a",
        label: "A",
        type: "group",
        required: false,
        fields: [
          {
            name: "b",
            label: "B",
            type: "group",
            required: false,
            fields: [{ name: "c", label: "C", type: "text", required: false }],
          },
        ],
      },
    ];
    expect(flattenFields(nested).map((f) => f.name)).toEqual(["a_b_c"]);
  });

  it("no muta la entrada", () => {
    flattenFields(FIXTURE);
    expect(FIXTURE[1].fields).toHaveLength(2);
    expect(FIXTURE[1].name).toBe("responsable");
  });
});

// Humo del schema embebido: invariantes estructurales, nunca contenido.
describe("schema embebido (entity_schema.json)", () => {
  it("PATIENT_LABEL no está vacío", () => {
    expect(PATIENT_LABEL.trim().length).toBeGreaterThan(0);
  });

  it("FLAT_PATIENT_FIELDS tiene claves únicas", () => {
    expect(PATIENT_FIELDS.length).toBeGreaterThan(0);
    const names = FLAT_PATIENT_FIELDS.map((f) => f.name);
    expect(new Set(names).size).toBe(names.length);
  });

  it("el listado aplanado no contiene contenedores group", () => {
    expect(
      FLAT_PATIENT_FIELDS.some((f) => f.fields && f.fields.length > 0),
    ).toBe(false);
  });
});
