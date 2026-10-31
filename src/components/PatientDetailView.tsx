// Panel derecho: ficha del paciente seleccionado, en modo solo lectura.
// Se genera desde entity_schema.json (mismos labels y orden que
// PatientForm); los campos sin dato muestran "—". La edición entra por
// el botón Editar, que carga PatientForm en modo edición.
import { PATIENT_FIELDS, PATIENT_LABEL, type FieldDef } from "../utils/schema";
import { Button } from "./ui/Button";

interface PatientDetailViewProps {
  patient: { data: Record<string, string> };
  onEdit: () => void;
}

function DetailRow({ label, value }: { label: string; value?: string }) {
  const hasValue = value != null && value.trim() !== "";
  return (
    <div className="col-span-1">
      {/* Rótulo del campo: es UI, no dato — sin selección (index.css).
          El valor de abajo sí es seleccionable (copiar DNI, etc.). */}
      <div className="select-none text-xs font-bold text-text-muted mb-1.5 uppercase tracking-wide">
        {label}
      </div>
      <div
        className={`text-sm rounded-lg border border-border bg-canvas/40 px-3 py-2.5 min-h-10 break-words ${
          hasValue ? "text-text-primary whitespace-pre-wrap" : "text-text-muted"
        }`}
        aria-label={hasValue ? undefined : `${label}: sin dato`}
      >
        {hasValue ? value : "—"}
      </div>
    </div>
  );
}

function DetailField({
  field,
  data,
}: {
  field: FieldDef;
  data: Record<string, string>;
}) {
  if (field.type === "group") {
    return (
      <div className="col-span-1 sm:col-span-2 rounded-lg border border-border bg-canvas/40 p-4">
        <h4 className="text-xs font-bold text-text-secondary uppercase tracking-wide mb-3">
          {field.label}
        </h4>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
          {field.fields?.map((sub) => (
            <DetailRow
              key={`${field.name}_${sub.name}`}
              label={sub.label}
              value={data[`${field.name}_${sub.name}`]}
            />
          ))}
        </div>
      </div>
    );
  }
  return <DetailRow label={field.label} value={data[field.name]} />;
}

export function PatientDetailView({ patient, onEdit }: PatientDetailViewProps) {
  return (
    <div className="flex-1 overflow-y-auto p-4 sm:p-6 border border-border rounded-xl shadow-lg bg-surface custom-scrollbar">
      <div className="flex justify-between items-center mb-6 gap-3">
        <h3 className="font-semibold text-text-primary uppercase tracking-wider text-sm flex items-center gap-2">
          <span aria-hidden="true">👤</span> Ficha de {PATIENT_LABEL}
        </h3>
        <Button onClick={onEdit} className="shrink-0">
          <span aria-hidden="true">✏️</span> Editar
        </Button>
      </div>

      {/* Grid responsivo igual que PatientForm: 1 col en móvil, 2 en desktop */}
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        {PATIENT_FIELDS.map((field) => (
          <DetailField key={field.name} field={field} data={patient.data} />
        ))}
      </div>
    </div>
  );
}
