// Panel derecho: formulario dinámico de paciente (alta/edición) generado
// desde entity_schema.json. Presentacional: recibe form y callbacks.
import { PATIENT_FIELDS, PATIENT_LABEL } from "../utils/schema";
import { Button } from "./ui/Button";

type EntityData = Record<string, string>;

interface PatientFormProps {
  form: EntityData;
  editingId: number | null;
  loading: boolean;
  onChange: (field: string, value: string) => void;
  onReset: () => void;
  onSubmit: () => void;
}

type SchemaField = {
  name: string;
  label: string;
  type: string;
  required?: boolean;
  placeholder?: string;
  options?: string[];
  fields?: SchemaField[];
};

const FIELD_INPUT_CLASSES =
  "w-full rounded-lg bg-field border border-border-strong px-3 py-2.5 text-sm text-text-primary focus:border-focus focus:ring-1 focus:ring-focus outline-none transition-colors appearance-none";

function FieldInput({
  field,
  value,
  onChange,
}: {
  field: SchemaField;
  value: string;
  onChange: (field: string, value: string) => void;
}) {
  switch (field.type) {
    case "textarea":
      return (
        <textarea
          id={field.name}
          value={value}
          onChange={(e) => onChange(field.name, e.target.value)}
          className={`${FIELD_INPUT_CLASSES} resize-y min-h-24`}
          placeholder={field.placeholder || field.label}
        />
      );
    case "select":
      return (
        <div className="relative">
          <select
            id={field.name}
            value={value}
            onChange={(e) => onChange(field.name, e.target.value)}
            className={`${FIELD_INPUT_CLASSES} pr-8 cursor-pointer`}
          >
            <option value="" disabled>
              Seleccionar...
            </option>
            {field.options?.map((opt) => (
              <option key={opt} value={opt} className="bg-surface text-text-primary">
                {opt}
              </option>
            ))}
          </select>
          <div className="pointer-events-none absolute inset-y-0 right-0 flex items-center px-2 text-text-secondary">
            <svg aria-hidden="true" className="fill-current h-4 w-4" viewBox="0 0 20 20">
              <path d="M9.293 12.95l.707.707L15.657 8l-1.414-1.414L10 10.828 5.757 6.586 4.343 8z" />
            </svg>
          </div>
        </div>
      );
    default:
      return (
        <input
          id={field.name}
          type={field.type === "date" ? "date" : "text"}
          value={value}
          onChange={(e) => onChange(field.name, e.target.value)}
          className={FIELD_INPUT_CLASSES}
          placeholder={field.placeholder || field.label}
        />
      );
  }
}

function FieldBlock({
  field,
  form,
  onChange,
}: {
  field: SchemaField;
  form: EntityData;
  onChange: (field: string, value: string) => void;
}) {
  if (field.type === "group") {
    return (
      <div className="col-span-1 sm:col-span-2 rounded-lg border border-border bg-canvas/40 p-4">
        <h4 className="text-xs font-bold text-text-secondary uppercase tracking-wide mb-3">
          {field.label}{" "}
          {field.required && <span className="text-red-500">*</span>}
        </h4>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
          {field.fields?.map((sub) => {
            // Misma convención de storage que el backend:
            // clave prefijada `{group}_{campo}`.
            const prefixed = {
              ...sub,
              name: `${field.name}_${sub.name}`,
            };
            return (
              <div
                key={prefixed.name}
                className={
                  prefixed.type === "textarea"
                    ? "col-span-1 sm:col-span-2"
                    : "col-span-1"
                }
              >
                <label
                  htmlFor={prefixed.name}
                  className="block text-xs font-bold text-text-muted mb-1.5 uppercase tracking-wide"
                >
                  {prefixed.label}{" "}
                  {prefixed.required && <span className="text-red-500">*</span>}
                </label>
                <FieldInput
                  field={prefixed}
                  value={form[prefixed.name] || ""}
                  onChange={onChange}
                />
              </div>
            );
          })}
        </div>
      </div>
    );
  }

  return (
    <div
      className={
        field.type === "textarea" ? "col-span-1 sm:col-span-2" : "col-span-1"
      }
    >
      <label
        htmlFor={field.name}
        className="block text-xs font-bold text-text-muted mb-1.5 uppercase tracking-wide"
      >
        {field.label}{" "}
        {field.required && <span className="text-red-500">*</span>}
      </label>
      <FieldInput field={field} value={form[field.name] || ""} onChange={onChange} />
    </div>
  );
}

export function PatientForm({
  form,
  editingId,
  loading,
  onChange,
  onReset,
  onSubmit,
}: PatientFormProps) {
  return (
    <div
      className={`flex-1 overflow-y-auto p-4 sm:p-6 border rounded-xl shadow-lg transition-colors custom-scrollbar ${
        editingId
          ? "border-amber-700/50 bg-amber-900/10"
          : "border-border bg-surface/50"
      }`}
    >
      <div className="flex justify-between items-center mb-6">
        <h3 className="font-semibold text-text-primary uppercase tracking-wider text-sm flex items-center gap-2">
          <span aria-hidden="true">{editingId ? "✏️" : "➕"}</span>{" "}
          {editingId ? "Editar" : "Nuevo"} {PATIENT_LABEL}
        </h3>
        {editingId && (
          <button
            onClick={onReset}
            className="text-xs text-text-secondary hover:text-text-inverse underline transition-colors"
          >
            Cancelar Edición
          </button>
        )}
      </div>

      {/* Grid responsivo de inputs: 1 col en móviles, 2 col en tablets/desktops */}
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        {PATIENT_FIELDS.map((field) => (
          <FieldBlock
            key={field.name}
            field={field as SchemaField}
            form={form}
            onChange={onChange}
          />
        ))}
      </div>

      {/* Footer sticky responsivo */}
      <div className="pt-6 mt-4 flex flex-col-reverse sm:flex-row justify-end gap-3 sticky bottom-0 bg-linear-to-t from-surface via-surface to-transparent pb-2">
        {editingId && (
          <Button
            variant="secondary"
            onClick={onReset}
            className="w-full sm:w-auto"
          >
            Cancelar
          </Button>
        )}
        <Button
          onClick={onSubmit}
          disabled={loading}
          className="w-full sm:w-auto"
        >
          {loading ? "Procesando..." : editingId ? "Guardar Cambios" : "Crear y Cifrar"}
        </Button>
      </div>
    </div>
  );
}
