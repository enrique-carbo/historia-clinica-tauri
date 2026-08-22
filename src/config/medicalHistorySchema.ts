// src/config/medicalHistorySchema.ts
import schema from './medical_history_schema.json';

export const MEDICAL_HISTORY_FIELDS = schema.fields;
export type MedicalHistoryFieldName = typeof MEDICAL_HISTORY_FIELDS[number]['name'];
export type ClinicalData = Record<string, string>;
