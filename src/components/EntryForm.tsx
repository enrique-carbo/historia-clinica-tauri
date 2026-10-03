import { useState } from "react";
import { useEntryStore, EntryCategory, EntryStatus } from "../stores/useEntryStore";
import { usePatientStore } from "../stores/usePatientStore";
import { Button } from "./ui/Button";

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

type EntryTemplateJson = Template & {
  description?: string;
  signature_payload_order?: string[];
  immutable?: boolean;
  requires_signature?: boolean;
};

const CATEGORY_ORDER: EntryCategory[] = ["SOAP_NOTE", "ALLERGY", "MEDICATION", "CONDITION"];

const TEMPLATE_MODULES = import.meta.glob("../../config/entry_templates/*.json", {
  eager: true,
  import: "default",
});

function loadTemplates(): Record<EntryCategory, Template> {
  const templates = {} as Partial<Record<EntryCategory, Template>>;

  for (const path of Object.keys(TEMPLATE_MODULES)) {
    const raw = TEMPLATE_MODULES[path] as EntryTemplateJson;
    const valid =
      (CATEGORY_ORDER as string[]).includes(raw?.category) &&
      typeof raw?.label === "string" &&
      Array.isArray(raw?.fields);
    if (!valid) {
      console.error(`[EntryTemplates] Template inválido: ${path}`);
      continue;
    }
    templates[raw.category] = { category: raw.category, label: raw.label, fields: raw.fields };
  }

  const missing = CATEGORY_ORDER.filter((c) => !templates[c]);
  if (missing.length > 0) {
    console.error(`[EntryTemplates] Faltan templates: ${missing.join(", ")}`);
  }

  return templates as Record<EntryCategory, Template>;
}

const TEMPLATES = loadTemplates();

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
          {CATEGORY_ORDER.filter((category) => TEMPLATES[category]).map((category) => (
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
          <Button type="submit" disabled={isLoading} className="w-full">
            {isLoading ? "Creando..." : "Crear Entry"}
          </Button>
        </form>
      )}
    </div>
  );
}
