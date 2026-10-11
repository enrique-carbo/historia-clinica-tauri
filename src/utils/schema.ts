// src/utils/schema.ts
//
// Único punto de consumo del schema de entities en el frontend.
// Convención de storage (igual que el backend, ver config_schema.rs):
// los subcampos de un group se guardan aplanados con la clave
// `{group}_{campo}` (ej: responsable_nombre_completo).

import schemaConfig from "../../src-tauri/config/entity_schema.json";

export interface FieldDef {
  name: string;
  label: string;
  type: string;
  required: boolean;
  encrypted?: boolean;
  fields?: FieldDef[];
}

export const PATIENT_LABEL: string = schemaConfig.entities.patient.label;
export const PATIENT_FIELDS: FieldDef[] = schemaConfig.entities.patient.fields;

/** Aplana groups con prefijo `{group}_{campo}`, preservando el orden. */
export function flattenFields(
  fields: FieldDef[],
  prefix = "",
): FieldDef[] {
  const out: FieldDef[] = [];
  for (const field of fields) {
    const name = prefix ? `${prefix}_${field.name}` : field.name;
    if (field.fields && field.fields.length > 0) {
      out.push(...flattenFields(field.fields, name));
    } else {
      out.push({ ...field, name });
    }
  }
  return out;
}

export const FLAT_PATIENT_FIELDS: FieldDef[] = flattenFields(PATIENT_FIELDS);
