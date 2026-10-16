import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useEntryStore, EntryCategory, EntryStatus } from "../stores/useEntryStore";
import { usePatientStore } from "../stores/usePatientStore";
import { useNavigationStore } from "../stores/useNavigationStore";
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
const MISSING_TEMPLATES = CATEGORY_ORDER.filter((c) => !TEMPLATES[c]);

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
  const setActiveMedicoTab = useNavigationStore((s) => s.setActiveMedicoTab);
  const [profileBlocked, setProfileBlocked] = useState<boolean | null>(null);

  useEffect(() => {
    let cancelled = false;
    invoke<{ full_name: string; profession: string; licenses: { number: string }[] }>(
      "get_my_profile"
    )
      .then((p) => {
        if (cancelled) return;
        const hasLicense = (p.licenses ?? []).some((l) => l.number?.trim());
        setProfileBlocked(
          !(p.full_name?.trim() && p.profession?.trim() && hasLicense)
        );
      })
      .catch(() => {
        // Sin perfil (o error): el backend también rechaza — mostrar gate.
        if (!cancelled) setProfileBlocked(true);
      });
    return () => {
      cancelled = true;
    };
  }, []);

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

  if (profileBlocked) {
    return (
      <div className="p-4 rounded-lg bg-amber-950/40 border border-amber-800 text-amber-200 text-sm flex flex-col gap-3">
        <p>
          Para crear entries necesitás completar tu perfil profesional:
          <strong> nombre, profesión y matrícula</strong>.
        </p>
        <div>
          <Button onClick={() => setActiveMedicoTab("perfil")}>
            Completar mi perfil
          </Button>
        </div>
      </div>
    );
  }

  if (!selectedPatient) {
    return (
      <div className="p-4 rounded-lg bg-surface border border-border text-text-muted text-sm">
        Seleccioná un paciente primero para crear una entry.
      </div>
    );
  }

  return (
    <div className="bg-surface border border-border rounded-xl p-6">
      {MISSING_TEMPLATES.length > 0 && (
        <div className="mb-4 p-3 bg-red-950/30 border border-red-800/50 rounded-lg">
          <p className="text-xs text-red-400">
            Faltan templates de entry: {MISSING_TEMPLATES.join(", ")}. Revisá
            config/entry_templates/.
          </p>
        </div>
      )}
      <h3 className="text-lg font-semibold text-text-primary mb-4 flex items-center gap-2">
        <span className="text-2xl" aria-hidden="true">➕</span>
        Nueva Entry
      </h3>

      {/* Selector de categoría */}
      <div className="mb-6">
        <span
          id="entry-category-label"
          className="block text-sm font-medium text-text-secondary mb-2"
        >
          Categoría
        </span>
        <div
          role="group"
          aria-labelledby="entry-category-label"
          className="grid grid-cols-2 sm:grid-cols-4 gap-2"
        >
          {CATEGORY_ORDER.filter((category) => TEMPLATES[category]).map((category) => (
            <button
              key={category}
              type="button"
              onClick={() => handleCategoryChange(category)}
              className={`p-3 rounded-lg border text-sm font-medium transition-colors ${
                selectedCategory === category
                  ? "bg-blue-900/30 border-blue-700 text-blue-300"
                  : "bg-field border-border text-text-secondary hover:border-border-strong"
              }`}
            >
              <span className="text-lg block mb-1" aria-hidden="true">{CATEGORY_ICONS[category]}</span>
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
            <label
              htmlFor="entry-title"
              className="block text-sm font-medium text-text-secondary mb-1"
            >
              Título *
            </label>
            <input
              id="entry-title"
              type="text"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="Resumen de la entry..."
              className="w-full px-3 py-2 bg-field border border-border-strong rounded-lg text-text-primary text-sm placeholder:text-text-placeholder focus:outline-none focus:border-focus"
            />
          </div>

          {/* Status */}
          <div>
            <label
              htmlFor="entry-status"
              className="block text-sm font-medium text-text-secondary mb-1"
            >
              Estado
            </label>
            <select
              id="entry-status"
              value={status}
              onChange={(e) => setStatus(e.target.value as EntryStatus)}
              className="w-full px-3 py-2 bg-field border border-border-strong rounded-lg text-text-primary text-sm focus:outline-none focus:border-focus appearance-none"
            >
              <option value="ACTIVE">Activo</option>
              <option value="RESOLVED">Resuelto</option>
              <option value="COMPLETED">Completado</option>
            </select>
          </div>

          {/* Campos dinámicos */}
          {template.fields.map((field) => (
            <div key={field.name}>
              <label
                htmlFor={`entry-${field.name}`}
                className="block text-sm font-medium text-text-secondary mb-1"
              >
                {field.label} {field.required && "*"}
              </label>
              {field.type === "textarea" ? (
                <textarea
                  id={`entry-${field.name}`}
                  value={formData[field.name] || ""}
                  onChange={(e) => handleFieldChange(field.name, e.target.value)}
                  placeholder={field.placeholder}
                  rows={3}
                  className="w-full px-3 py-2 bg-field border border-border-strong rounded-lg text-text-primary text-sm placeholder:text-text-placeholder focus:outline-none focus:border-focus resize-none"
                />
              ) : field.type === "select" ? (
                <select
                  id={`entry-${field.name}`}
                  value={formData[field.name] || ""}
                  onChange={(e) => handleFieldChange(field.name, e.target.value)}
                  className="w-full px-3 py-2 bg-field border border-border-strong rounded-lg text-text-primary text-sm focus:outline-none focus:border-focus appearance-none"
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
                  id={`entry-${field.name}`}
                  type="text"
                  value={formData[field.name] || ""}
                  onChange={(e) => handleFieldChange(field.name, e.target.value)}
                  placeholder={field.placeholder}
                  className="w-full px-3 py-2 bg-field border border-border-strong rounded-lg text-text-primary text-sm placeholder:text-text-placeholder focus:outline-none focus:border-focus"
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
