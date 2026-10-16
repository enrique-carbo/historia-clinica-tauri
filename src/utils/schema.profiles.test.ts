// src/utils/schema.profiles.test.ts
//
// Invariantes estructurales de AMBOS perfiles de producto
// (med y vet, docs/PLUGIN_ARCHITECTURE.md "Perfiles de producto").
// Sin acoplar a labels ni a un listado concreto de campos (AGENTS.md):
// los perfiles pueden divergir en labels y campos, pero no en el contrato
// que el Core asume.

import { describe, it, expect } from "vitest";
import medSchema from "../../profiles/med/entity_schema.json";
import vetSchema from "../../profiles/vet/entity_schema.json";
import { flattenFields } from "./schema";

const profiles = [
  ["med", medSchema],
  ["vet", vetSchema],
] as const;

describe.each(profiles)("invariantes del schema (%s)", (_name, schema) => {
  const patient = schema.entities.patient;

  it("declara patient con labels no vacíos", () => {
    expect(patient.label).toBeTruthy();
    expect(patient.label_plural).toBeTruthy();
  });

  it("blind_index_field apunta a un campo declarado con blind_index", () => {
    const declared = patient.fields.find((f) => f.name === patient.blind_index_field);
    expect(declared).toBeDefined();
    expect(declared?.blind_index).toBe(true);
    expect(declared?.required).toBe(true);
    expect(declared?.encrypted).toBe(false);
  });

  it("los storage keys (aplanados) son únicos", () => {
    const flat = flattenFields(patient.fields as Parameters<typeof flattenFields>[0]);
    const names = flat.map((f) => f.name);
    expect(new Set(names).size).toBe(names.length);
  });

  it("al menos un campo requerido y todos con label", () => {
    expect(patient.fields.some((f) => f.required)).toBe(true);
    for (const field of patient.fields) {
      expect(field.label).toBeTruthy();
      expect(field.type).toBeTruthy();
    }
  });

  it("los selects declaran options no vacías", () => {
    const selects = flattenFields(
      patient.fields as Parameters<typeof flattenFields>[0],
    ).filter((f) => f.type === "select");
    for (const select of selects) {
      expect(select.options, `select ${select.name}`).toBeDefined();
      expect(select.options!.length).toBeGreaterThan(0);
    }
  });
});

describe("compatibilidad entre perfiles", () => {
  it("comparten los nombres de campo que el Core asume (dni/nombre/apellido)", () => {
    const medFlat = new Set(
      flattenFields(medSchema.entities.patient.fields as Parameters<typeof flattenFields>[0]).map(
        (f) => f.name,
      ),
    );
    const vetFlat = new Set(
      flattenFields(vetSchema.entities.patient.fields as Parameters<typeof flattenFields>[0]).map(
        (f) => f.name,
      ),
    );
    for (const field of ["dni", "nombre", "apellido", "fecha_nacimiento"]) {
      expect(medFlat.has(field), `med: ${field}`).toBe(true);
      expect(vetFlat.has(field), `vet: ${field}`).toBe(true);
    }
  });

  it("ambos usan 'dni' como blind_index_field (clave del índice HMAC)", () => {
    expect(medSchema.entities.patient.blind_index_field).toBe("dni");
    expect(vetSchema.entities.patient.blind_index_field).toBe("dni");
  });
});
