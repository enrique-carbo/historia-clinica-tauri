import { useState } from "react";
import { useEntryStore, EntryCategory, EntryStatus } from "../stores/useEntryStore";
import { usePatientStore } from "../stores/usePatientStore";

interface EntryFormProps {
  onSuccess?: () => void;
}

interface TemplateField {
  name: string;
  label: string;
  type: "text" | "textarea" | "select";
  required: boolean;
  placeholder?: string;
  options?: string[];
}

interface Template {
  category: EntryCategory;
  label: string;
  fields: TemplateField[];
}

const TEMPLATES: Record<EntryCategory, Template> = {
  SOAP_NOTE: {
    category: "SOAP_NOTE",
    label: "Nota SOAP",
    fields: [
      { name: "subjetivo", label: "Subjetivo", type: "textarea", required: true, placeholder: "Síntomas y preocupaciones del paciente..." },
      { name: "objetivo", label: "Objetivo", type: "textarea", required: true, placeholder: "Signos vitales, hallazgos del examen físico..." },
      { name: "evaluacion", label: "Evaluación", type: "textarea", required: true, placeholder: "Diagnóstico diferencial, impresión clínica..." },
      { name: "plan", label: "Plan", type: "textarea", required: true, placeholder: "Tratamiento, medicación, próxima consulta..." },
    ],
  },
  ALLERGY: {
    category: "ALLERGY",
    label: "Alergia",
    fields: [
      { name: "sustancia", label: "Sustancia", type: "text", required: true, placeholder: "Ej: Penicilina, Polen, Mariscos..." },
      { name: "tipo_reaccion", label: "Tipo de Reacción", type: "select", required: true, options: ["Leve", "Moderada", "Severa", "Anafilaxia"] },
      { name: "descripcion", label: "Descripción", type: "textarea", required: false, placeholder: "Detalles de la reacción alérgica..." },
    ],
  },
  MEDICATION: {
    category: "MEDICATION",
    label: "Medicación",
    fields: [
      { name: "medicamento", label: "Medicamento", type: "text", required: true, placeholder: "Ej: Enalapril, Ibuprofeno..." },
      { name: "dosis", label: "Dosis", type: "text", required: true, placeholder: "Ej: 10mg, 500mg..." },
      { name: "frecuencia", label: "Frecuencia", type: "select", required: true, options: ["Cada 8 horas", "Cada 12 horas", "Cada 24 horas", "Según necesidad", "Una vez"] },
      { name: "via_administracion", label: "Vía", type: "select", required: true, options: ["Oral", "Intravenosa", "Intramuscular", "Subcutánea", "Tópica", "Inhalatoria"] },
      { name: "indicacion", label: "Indicación", type: "textarea", required: false, placeholder: "Motivo de la prescripción..." },
    ],
  },
  CONDITION: {
    category: "CONDITION",
    label: "Diagnóstico / Antecedentes",
    fields: [
      { name: "diagnostico", label: "Diagnóstico", type: "text", required: true, placeholder: "Ej: Hipertensión arterial..." },
      { name: "codigo_cie10", label: "Código CIE-10", type: "text", required: false, placeholder: "Ej: I10, E11.9" },
      { name: "estado", label: "Estado", type: "select", required: true, options: ["Activo", "En tratamiento", "Resuelto", "Crónico"] },
      { name: "observaciones", label: "Observaciones", type: "textarea", required: false, placeholder: "Notas adicionales..." },
    ],
  },
};

const CATEGORY_ICONS: Record<EntryCategory, string> = {
  SOAP_NOTE: "📋",
  ALLERGY: "⚠️",
  MEDICATION: "💊",
  CONDITION: "🏥",
};

export function EntryForm({ onSuccess }: EntryFormProps) {
  const [selectedCategory, setSelectedCategory] = useState<EntryCategory | null>(null);
  const [formData, setFormData] = useState<Record<string, string>>({});
  const [title, setTitle] = useState("");
  const [status, setStatus] = useState<EntryStatus>("ACTIVE");
  const [error, setError] = useState<string | null>(null);

  const { createEntry, isLoading } = useEntryStore();
  const selectedPatient = usePatientStore((s) => s.selected);

  const template = selectedCategory ? TEMPLATES[selectedCategory] : null;

  const handleCategoryChange = (category: EntryCategory) => {
    setSelectedCategory(category);
    setFormData({});
    setTitle("");
    setError(null);
  };

  const handleFieldChange = (fieldName: string, value: string) => {
    setFormData((prev) => ({ ...prev, [fieldName]: value }));
  };

  const handleSubmit = async (e: React.SubmitEvent) => {
    e.preventDefault();
    setError(null);

    if (!selectedPatient) {
      setError("No hay paciente seleccionado");
      return;
    }

    if (!selectedCategory || !template) {
      setError("Seleccioná una categoría");
      return;
    }

    if (!title.trim()) {
      setError("El título es requerido");
      return;
    }

    // Validar campos requeridos
    for (const field of template.fields) {
      if (field.required && !formData[field.name]?.trim()) {
        setError(`El campo "${field.label}" es requerido`);
        return;
      }
    }

    const result = await createEntry({
      category: selectedCategory,
      subjectId: selectedPatient.patient.id,
      title: title.trim(),
      status,
      payload: formData,
    });

    if (result) {
      // Éxito
      setFormData({});
      setTitle("");
      setSelectedCategory(null);
      onSuccess?.();
    }
  };

  if (!selectedPatient) {
    return (
      <div className="p-4 rounded-lg bg-zinc-900 border border-zinc-800 text-zinc-500 text-sm">
        Seleccioná un paciente primero para crear una entry.
      </div>
    );
  }

  return (
    <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6">
      <h3 className="text-lg font-semibold text-zinc-200 mb-4 flex items-center gap-2">
        <span className="text-2xl">➕</span>
        Nueva Entry
      </h3>

      {/* Selector de categoría */}
      <div className="mb-6">
        <label className="block text-sm font-medium text-zinc-400 mb-2">
          Categoría
        </label>
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
          {(Object.keys(TEMPLATES) as EntryCategory[]).map((category) => (
            <button
              key={category}
              type="button"
              onClick={() => handleCategoryChange(category)}
              className={`p-3 rounded-lg border text-sm font-medium transition-colors ${
                selectedCategory === category
                  ? "bg-blue-900/30 border-blue-700 text-blue-300"
                  : "bg-zinc-950 border-zinc-800 text-zinc-400 hover:border-zinc-700"
              }`}
            >
              <span className="text-lg block mb-1">{CATEGORY_ICONS[category]}</span>
              {TEMPLATES[category].label}
            </button>
          ))}
        </div>
      </div>

      {/* Formulario */}
      {template && (
        <form onSubmit={handleSubmit} className="space-y-4">
          {/* Título */}
          <div>
            <label className="block text-sm font-medium text-zinc-400 mb-1">
              Título *
            </label>
            <input
              type="text"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="Resumen de la entry..."
              className="w-full px-3 py-2 bg-zinc-950 border border-zinc-700 rounded-lg text-zinc-200 text-sm placeholder-zinc-600 focus:outline-none focus:border-blue-600"
            />
          </div>

          {/* Status */}
          <div>
            <label className="block text-sm font-medium text-zinc-400 mb-1">
              Estado
            </label>
            <select
              value={status}
              onChange={(e) => setStatus(e.target.value as EntryStatus)}
              className="w-full px-3 py-2 bg-zinc-950 border border-zinc-700 rounded-lg text-zinc-200 text-sm focus:outline-none focus:border-blue-600 appearance-none"
            >
              <option value="ACTIVE">Activo</option>
              <option value="RESOLVED">Resuelto</option>
              <option value="COMPLETED">Completado</option>
            </select>
          </div>

          {/* Campos dinámicos */}
          {template.fields.map((field) => (
            <div key={field.name}>
              <label className="block text-sm font-medium text-zinc-400 mb-1">
                {field.label} {field.required && "*"}
              </label>
              {field.type === "textarea" ? (
                <textarea
                  value={formData[field.name] || ""}
                  onChange={(e) => handleFieldChange(field.name, e.target.value)}
                  placeholder={field.placeholder}
                  rows={3}
                  className="w-full px-3 py-2 bg-zinc-950 border border-zinc-700 rounded-lg text-zinc-200 text-sm placeholder-zinc-600 focus:outline-none focus:border-blue-600 resize-none"
                />
              ) : field.type === "select" ? (
                <select
                  value={formData[field.name] || ""}
                  onChange={(e) => handleFieldChange(field.name, e.target.value)}
                  className="w-full px-3 py-2 bg-zinc-950 border border-zinc-700 rounded-lg text-zinc-200 text-sm focus:outline-none focus:border-blue-600 appearance-none"
                >
                  <option value="">Seleccionar...</option>
                  {field.options?.map((opt) => (
                    <option key={opt} value={opt}>
                      {opt}
                    </option>
                  ))}
                </select>
              ) : (
                <input
                  type="text"
                  value={formData[field.name] || ""}
                  onChange={(e) => handleFieldChange(field.name, e.target.value)}
                  placeholder={field.placeholder}
                  className="w-full px-3 py-2 bg-zinc-950 border border-zinc-700 rounded-lg text-zinc-200 text-sm placeholder-zinc-600 focus:outline-none focus:border-blue-600"
                />
              )}
            </div>
          ))}

          {/* Error */}
          {error && (
            <div className="p-3 bg-red-950/30 border border-red-800/50 rounded-lg">
              <p className="text-xs text-red-400">{error}</p>
            </div>
          )}

          {/* Submit */}
          <button
            type="submit"
            disabled={isLoading}
            className="w-full px-4 py-3 bg-blue-600 hover:bg-blue-500 disabled:bg-zinc-700 text-white rounded-lg font-medium transition-colors"
          >
            {isLoading ? "Creando..." : "Crear Entry"}
          </button>
        </form>
      )}
    </div>
  );
}
