// Panel del módulo de solicitudes de estudios (`docs/MODULES/module-solicitudes-v1.md`).
//
// Se monta dentro de la ficha del paciente. Si el módulo no está compilado,
// activo o el rol no tiene acceso, `is_module_available` devuelve false y no
// se renderiza nada. Toda la comunicación pasa por `module_execute`.

import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import { formatError } from "../utils/errors";
import { isModuleAvailable, moduleExecute } from "../utils/modules";
import { canShare, downloadText, printHtml, shareText } from "../utils/print";
import { Alert } from "./ui/Alert";
import { Button } from "./ui/Button";
import { EmptyState } from "./ui/EmptyState";
import { Input } from "./ui/Input";
import { Textarea } from "./ui/Textarea";

const MODULE_ID = "solicitudes";

type FieldType = "text" | "textarea" | "select" | "date";

interface TemplateField {
  name: string;
  label: string;
  type: FieldType;
  required?: boolean;
  options?: string[];
}

interface TemplateRecord {
  id: string;
  name: string;
  fields: TemplateField[];
  created_at: string;
  updated_at: string;
}

interface SolicitudSummary {
  id: string;
  entity_id: number;
  study_name: string;
  template_id: string | null;
  status: string;
  created_at: string;
}

interface RenderedSolicitud {
  markdown: string;
  html: string;
}

interface PreviewState {
  title: string;
  markdown: string;
  html: string;
}

function formatDate(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return date.toLocaleString("es-AR", { dateStyle: "short", timeStyle: "short" });
}

function slugify(value: string): string {
  return value
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");
}

function safeFilename(title: string): string {
  const base = slugify(title) || "solicitud";
  return `Solicitud_${base}_${new Date().toISOString().slice(0, 10)}`;
}

export function SolicitudesPanel({ entityId }: { entityId: number }) {
  const [available, setAvailable] = useState<boolean | null>(null);
  const [solicitudes, setSolicitudes] = useState<SolicitudSummary[]>([]);
  const [templates, setTemplates] = useState<TemplateRecord[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isFormOpen, setIsFormOpen] = useState(false);
  const [isTemplatesOpen, setIsTemplatesOpen] = useState(false);
  const [preview, setPreview] = useState<PreviewState | null>(null);

  const reload = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const [list, templateList] = await Promise.all([
        moduleExecute<SolicitudSummary[]>(MODULE_ID, "solicitud.list", {}, entityId),
        moduleExecute<TemplateRecord[]>(MODULE_ID, "template.list"),
      ]);
      setSolicitudes(list);
      setTemplates(templateList);
    } catch (err) {
      setError(formatError(err));
    } finally {
      setIsLoading(false);
    }
  }, [entityId]);

  useEffect(() => {
    let cancelled = false;
    isModuleAvailable(MODULE_ID)
      .then((enabled) => {
        if (!cancelled) setAvailable(enabled);
      })
      .catch(() => {
        if (!cancelled) setAvailable(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // Cerrar formulario y vista previa al cambiar de paciente: un borrador
  // abierto en el Paciente A nunca debe quedar listo para emitirse sobre el
  // Paciente B (mismo criterio que `showSoapForm` en `PatientEhrView.tsx`).
  useEffect(() => {
    setIsFormOpen(false);
    setPreview(null);
  }, [entityId]);

  useEffect(() => {
    if (available) void reload();
    else {
      setSolicitudes([]);
      setTemplates([]);
    }
  }, [available, reload]);

  const openPreview = async (id: string, title: string) => {
    try {
      const rendered = await moduleExecute<RenderedSolicitud>(
        MODULE_ID,
        "solicitud.render",
        { id },
      );
      setPreview({ title, html: rendered.html, markdown: rendered.markdown });
    } catch (err) {
      setError(formatError(err));
    }
  };

  if (!available) return null;

  return (
    <div className="border border-border rounded-xl bg-surface shadow-lg overflow-hidden">
      <div className="p-4 border-b border-border bg-surface flex items-center justify-between gap-3">
        <h3 className="text-xs font-bold text-text-muted uppercase tracking-widest">
          Solicitudes de estudios
        </h3>
        <div className="flex items-center gap-2 shrink-0">
          <Button size="sm" variant="ghost" onClick={() => setIsTemplatesOpen(true)}>
            Mis templates
          </Button>
          <Button
            size="sm"
            variant="secondary"
            onClick={() => setIsFormOpen((open) => !open)}
            aria-expanded={isFormOpen}
          >
            {isFormOpen ? "Cancelar" : "Nueva solicitud"}
          </Button>
        </div>
      </div>

      <div className="p-4">
        {error && (
          <Alert variant="error" className="mb-3 text-sm">
            {error}
          </Alert>
        )}

        {isFormOpen && (
          <SolicitudForm
            key={entityId}
            entityId={entityId}
            templates={templates}
            onCancel={() => setIsFormOpen(false)}
            onCreated={() => {
              setIsFormOpen(false);
              void reload();
            }}
            onError={setError}
          />
        )}

        {isLoading ? (
          <div className="flex justify-center py-6">
            <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-focus" />
          </div>
        ) : solicitudes.length === 0 ? (
          <EmptyState
            title="Sin solicitudes registradas"
            hint="Usá «Nueva solicitud» para emitir la primera"
          />
        ) : (
          <ul className="space-y-2">
            {solicitudes.map((solicitud) => (
              <li
                key={solicitud.id}
                className="flex items-center justify-between gap-3 rounded-lg border border-border bg-canvas/40 px-3 py-2"
              >
                <div className="min-w-0">
                  <div className="text-sm font-semibold text-text-primary truncate">
                    {solicitud.study_name}
                  </div>
                  <div className="text-[11px] text-text-muted">
                    {formatDate(solicitud.created_at)} · {solicitud.status}
                  </div>
                </div>
                <Button
                  size="sm"
                  variant="secondary"
                  className="shrink-0"
                  onClick={() => void openPreview(solicitud.id, solicitud.study_name)}
                >
                  Ver / Imprimir
                </Button>
              </li>
            ))}
          </ul>
        )}
      </div>

      {isTemplatesOpen && (
        <TemplatesDialog
          templates={templates}
          onClose={() => setIsTemplatesOpen(false)}
          onChanged={() => void reload()}
          onError={setError}
        />
      )}

      {preview && (
        <PreviewDialog
          preview={preview}
          onClose={() => setPreview(null)}
          onError={setError}
        />
      )}
    </div>
  );
}

// =======================================================================
// ALTA DE SOLICITUD
// =======================================================================

interface SolicitudFormProps {
  entityId: number;
  templates: TemplateRecord[];
  onCancel: () => void;
  onCreated: () => void;
  onError: (message: string) => void;
}

function SolicitudForm({
  entityId,
  templates,
  onCancel,
  onCreated,
  onError,
}: SolicitudFormProps) {
  const [templateId, setTemplateId] = useState("");
  const [studyName, setStudyName] = useState("");
  const [values, setValues] = useState<Record<string, string>>({});
  const [manualRows, setManualRows] = useState([{ key: "", value: "" }]);
  const [isSaving, setIsSaving] = useState(false);

  const selectedTemplate = useMemo(
    () => templates.find((template) => template.id === templateId),
    [templates, templateId],
  );

  useEffect(() => {
    setValues({});
    setStudyName(selectedTemplate?.name ?? "");
  }, [selectedTemplate]);

  const setValue = (key: string, value: string) =>
    setValues((current) => ({ ...current, [key]: value }));

  const submit = async () => {
    const manualValues = Object.fromEntries(
      manualRows
        .map((row) => [row.key.trim(), row.value] as const)
        .filter(([key]) => key.length > 0),
    );

    const payload = {
      template_id: templateId || null,
      study_name: templateId ? null : studyName.trim(),
      values: selectedTemplate ? values : manualValues,
    };

    setIsSaving(true);
    try {
      await moduleExecute<{ id: string }>(
        MODULE_ID,
        "solicitud.create",
        payload,
        entityId,
      );
      onCreated();
    } catch (err) {
      onError(formatError(err));
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div className="mb-4 rounded-lg border border-border bg-canvas/40 p-4 space-y-3">
      <div>
        <label
          htmlFor="solicitud-template"
          className="block text-xs text-text-muted mb-1 font-semibold"
        >
          Template
        </label>
        <select
          id="solicitud-template"
          value={templateId}
          onChange={(event) => setTemplateId(event.target.value)}
          className="w-full bg-field text-text-primary border border-border rounded px-3 py-2.5 text-sm focus:outline-none focus:ring-1 focus:ring-focus"
        >
          <option value="">Sin template (completar a mano)</option>
          {templates.map((template) => (
            <option key={template.id} value={template.id}>
              {template.name}
            </option>
          ))}
        </select>
        {templates.length === 0 && (
          <p className="mt-1 text-[11px] text-text-muted">
            Todavía no tenés templates. Creá uno en «Mis templates».
          </p>
        )}
      </div>

      {selectedTemplate ? (
        <div className="space-y-3">
          {selectedTemplate.fields.map((field) => (
            <FieldInput
              key={field.name}
              field={field}
              value={values[field.name] ?? ""}
              onChange={(value) => setValue(field.name, value)}
            />
          ))}
        </div>
      ) : (
        <div className="space-y-3">
          <Input
            label="Estudio"
            value={studyName}
            onChange={(event) => setStudyName(event.target.value)}
            placeholder="Ej. Cultivo de orina"
          />
          <div className="space-y-2">
            {manualRows.map((row, index) => (
              <div key={index} className="grid grid-cols-1 sm:grid-cols-[1fr_2fr_auto] gap-2 items-end">
                <Input
                  label={index === 0 ? "Campo" : undefined}
                  value={row.key}
                  onChange={(event) =>
                    setManualRows((rows) =>
                      rows.map((item, i) =>
                        i === index ? { ...item, key: event.target.value } : item,
                      ),
                    )
                  }
                  placeholder="material"
                />
                <Input
                  label={index === 0 ? "Valor" : undefined}
                  value={row.value}
                  onChange={(event) =>
                    setManualRows((rows) =>
                      rows.map((item, i) =>
                        i === index ? { ...item, value: event.target.value } : item,
                      ),
                    )
                  }
                  placeholder="orina"
                />
                <Button
                  size="sm"
                  variant="ghost-danger"
                  onClick={() =>
                    setManualRows((rows) =>
                      rows.length === 1
                        ? [{ key: "", value: "" }]
                        : rows.filter((_, i) => i !== index),
                    )
                  }
                >
                  Quitar
                </Button>
              </div>
            ))}
            <Button
              size="sm"
              variant="secondary"
              onClick={() =>
                setManualRows((rows) => [...rows, { key: "", value: "" }])
              }
            >
              Agregar campo
            </Button>
          </div>
        </div>
      )}

      <div className="flex justify-end gap-2 pt-1">
        <Button size="sm" variant="ghost" onClick={onCancel}>
          Cancelar
        </Button>
        <Button size="sm" isLoading={isSaving} onClick={() => void submit()}>
          Emitir solicitud
        </Button>
      </div>
    </div>
  );
}

function FieldInput({
  field,
  value,
  onChange,
}: {
  field: TemplateField;
  value: string;
  onChange: (value: string) => void;
}) {
  const label = `${field.label}${field.required ? " *" : ""}`;

  if (field.type === "textarea") {
    return (
      <Textarea
        label={label}
        value={value}
        rows={3}
        onChange={(event) => onChange(event.target.value)}
      />
    );
  }

  if (field.type === "select") {
    const selectId = `solicitud-field-${field.name}`;
    return (
      <div>
        <label
          htmlFor={selectId}
          className="block text-xs text-text-muted mb-1 font-semibold"
        >
          {label}
        </label>
        <select
          id={selectId}
          value={value}
          onChange={(event) => onChange(event.target.value)}
          className="w-full bg-field text-text-primary border border-border rounded px-3 py-2.5 text-sm focus:outline-none focus:ring-1 focus:ring-focus"
        >
          <option value="">Seleccionar...</option>
          {(field.options ?? []).map((option) => (
            <option key={option} value={option}>
              {option}
            </option>
          ))}
        </select>
      </div>
    );
  }

  return (
    <Input
      label={label}
      type={field.type === "date" ? "date" : "text"}
      value={value}
      onChange={(event) => onChange(event.target.value)}
    />
  );
}

// =======================================================================
// TEMPLATES
// =======================================================================

interface EditableField {
  label: string;
  type: FieldType;
  required: boolean;
  options: string;
}

function editableFrom(field: TemplateField): EditableField {
  return {
    label: field.label,
    type: field.type,
    required: Boolean(field.required),
    options: (field.options ?? []).join(", "),
  };
}

interface TemplatesDialogProps {
  templates: TemplateRecord[];
  onClose: () => void;
  onChanged: () => void;
  onError: (message: string) => void;
}

function TemplatesDialog({
  templates,
  onClose,
  onChanged,
  onError,
}: TemplatesDialogProps) {
  const [editingId, setEditingId] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [fields, setFields] = useState<EditableField[]>([]);
  const [isSaving, setIsSaving] = useState(false);

  const startCreate = () => {
    setEditingId(null);
    setName("");
    setFields([{ label: "", type: "text", required: false, options: "" }]);
  };

  const startEdit = (template: TemplateRecord) => {
    setEditingId(template.id);
    setName(template.name);
    setFields(template.fields.map(editableFrom));
  };

  const reset = () => {
    setEditingId(null);
    setName("");
    setFields([]);
  };

  const save = async () => {
    if (!name.trim()) {
      onError("El template necesita un nombre.");
      return;
    }
    if (fields.length === 0) {
      onError("El template necesita al menos un campo.");
      return;
    }

    const payloadFields = fields.map((field) => ({
      name: slugify(field.label),
      label: field.label.trim(),
      type: field.type,
      required: field.required,
      options:
        field.type === "select"
          ? field.options.split(",").map((option) => option.trim()).filter(Boolean)
          : [],
    }));

    if (payloadFields.some((field) => !field.name || !field.label)) {
      onError("Cada campo necesita una etiqueta.");
      return;
    }
    if (payloadFields.some((field) => field.type === "select" && field.options.length === 0)) {
      onError("Un campo de tipo selección necesita al menos una opción.");
      return;
    }
    const names = payloadFields.map((field) => field.name);
    if (new Set(names).size !== names.length) {
      onError("Hay etiquetas repetidas: cada campo debe ser distinto.");
      return;
    }

    setIsSaving(true);
    try {
      if (editingId) {
        await moduleExecute(MODULE_ID, "template.update", {
          id: editingId,
          name: name.trim(),
          fields: payloadFields,
        });
      } else {
        await moduleExecute(MODULE_ID, "template.create", {
          name: name.trim(),
          fields: payloadFields,
        });
      }
      reset();
      onChanged();
    } catch (err) {
      onError(formatError(err));
    } finally {
      setIsSaving(false);
    }
  };

  const remove = async (template: TemplateRecord) => {
    if (!window.confirm(`¿Eliminar el template «${template.name}»?`)) return;
    try {
      await moduleExecute(MODULE_ID, "template.delete", { id: template.id });
      onChanged();
    } catch (err) {
      onError(formatError(err));
    }
  };

  const isEditing = fields.length > 0;

  return (
    <Dialog title="Mis templates de estudios" onClose={onClose}>
      {isEditing ? (
        <div className="space-y-3">
          <Input
            label="Nombre del template"
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder="Ej. Laboratorio"
          />

          <div className="space-y-2">
            {fields.map((field, index) => (
              <div key={index} className="rounded-lg border border-border p-3 space-y-2">
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
                  <Input
                    label="Etiqueta"
                    value={field.label}
                    onChange={(event) =>
                      setFields((current) =>
                        current.map((item, i) =>
                          i === index ? { ...item, label: event.target.value } : item,
                        ),
                      )
                    }
                    placeholder="Ej. Indicación"
                  />
                  <div>
                    <label
                      htmlFor={`template-field-type-${index}`}
                      className="block text-xs text-text-muted mb-1 font-semibold"
                    >
                      Tipo
                    </label>
                    <select
                      id={`template-field-type-${index}`}
                      value={field.type}
                      onChange={(event) =>
                        setFields((current) =>
                          current.map((item, i) =>
                            i === index
                              ? { ...item, type: event.target.value as FieldType }
                              : item,
                          ),
                        )
                      }
                      className="w-full bg-field text-text-primary border border-border rounded px-3 py-2.5 text-sm focus:outline-none focus:ring-1 focus:ring-focus"
                    >
                      <option value="text">Texto</option>
                      <option value="textarea">Texto largo</option>
                      <option value="select">Selección</option>
                      <option value="date">Fecha</option>
                    </select>
                  </div>
                </div>

                {field.type === "select" && (
                  <Input
                    label="Opciones (separadas por coma)"
                    value={field.options}
                    onChange={(event) =>
                      setFields((current) =>
                        current.map((item, i) =>
                          i === index ? { ...item, options: event.target.value } : item,
                        ),
                      )
                    }
                    placeholder="Rutina, Urgente"
                  />
                )}

                <div className="flex items-center justify-between">
                  <label className="flex items-center gap-2 text-xs text-text-secondary">
                    <input
                      type="checkbox"
                      checked={field.required}
                      onChange={(event) =>
                        setFields((current) =>
                          current.map((item, i) =>
                            i === index
                              ? { ...item, required: event.target.checked }
                              : item,
                          ),
                        )
                      }
                    />
                    Obligatorio
                  </label>
                  <Button
                    size="sm"
                    variant="ghost-danger"
                    onClick={() =>
                      setFields((current) =>
                        current.filter((_, i) => i !== index),
                      )
                    }
                  >
                    Quitar
                  </Button>
                </div>
              </div>
            ))}
            <Button
              size="sm"
              variant="secondary"
              onClick={() =>
                setFields((current) => [
                  ...current,
                  { label: "", type: "text", required: false, options: "" },
                ])
              }
            >
              Agregar campo
            </Button>
          </div>

          <div className="flex justify-end gap-2 pt-1">
            <Button size="sm" variant="ghost" onClick={reset}>
              Cancelar
            </Button>
            <Button size="sm" isLoading={isSaving} onClick={() => void save()}>
              Guardar template
            </Button>
          </div>
        </div>
      ) : (
        <div className="space-y-3">
          {templates.length === 0 ? (
            <EmptyState
              title="Todavía no creaste templates"
              hint="Un template define los campos de tus solicitudes"
            />
          ) : (
            <ul className="space-y-2">
              {templates.map((template) => (
                <li
                  key={template.id}
                  className="flex items-center justify-between gap-3 rounded-lg border border-border bg-canvas/40 px-3 py-2"
                >
                  <div className="min-w-0">
                    <div className="text-sm font-semibold text-text-primary truncate">
                      {template.name}
                    </div>
                    <div className="text-[11px] text-text-muted">
                      {template.fields.length} campo
                      {template.fields.length === 1 ? "" : "s"}
                    </div>
                  </div>
                  <div className="flex gap-2 shrink-0">
                    <Button size="sm" variant="secondary" onClick={() => startEdit(template)}>
                      Editar
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost-danger"
                      onClick={() => void remove(template)}
                    >
                      Eliminar
                    </Button>
                  </div>
                </li>
              ))}
            </ul>
          )}
          <div className="flex justify-end">
            <Button size="sm" onClick={startCreate}>
              Nuevo template
            </Button>
          </div>
        </div>
      )}
    </Dialog>
  );
}

// =======================================================================
// PREVIEW / IMPRESIÓN
// =======================================================================

function PreviewDialog({
  preview,
  onClose,
  onError,
}: {
  preview: PreviewState;
  onClose: () => void;
  onError: (message: string) => void;
}) {
  const print = async () => {
    if (printHtml(preview.html)) return;
    if (await shareText(preview.title, preview.markdown)) return;
    onError("La impresión no está disponible en este dispositivo.");
  };

  return (
    <Dialog title={`Solicitud: ${preview.title}`} onClose={onClose}>
      <div className="space-y-3">
        <iframe
          title={`Vista previa de ${preview.title}`}
          srcDoc={preview.html}
          sandbox=""
          className="h-[55vh] w-full rounded-lg border border-border bg-white"
        />
        <div className="flex flex-wrap justify-end gap-2">
          {canShare() && (
            <Button
              size="sm"
              variant="secondary"
              onClick={() => void shareText(preview.title, preview.markdown)}
            >
              Compartir
            </Button>
          )}
          <Button
            size="sm"
            variant="secondary"
            onClick={() => downloadText(`${safeFilename(preview.title)}.md`, preview.markdown)}
          >
            Descargar .md
          </Button>
          <Button
            size="sm"
            variant="secondary"
            onClick={() =>
              downloadText(
                `${safeFilename(preview.title)}.html`,
                preview.html,
                "text/html;charset=utf-8",
              )
            }
          >
            Descargar .html
          </Button>
          <Button size="sm" onClick={() => void print()}>
            Imprimir / PDF
          </Button>
        </div>
        <p className="text-[11px] text-text-muted">
          «Imprimir / PDF» abre el diálogo del sistema, donde podés elegir
          guardar como PDF.
        </p>
      </div>
    </Dialog>
  );
}

// =======================================================================
// DIALOG base
// =======================================================================

function Dialog({
  title,
  onClose,
  children,
}: {
  title: string;
  onClose: () => void;
  children: ReactNode;
}) {
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
      role="dialog"
      aria-modal="true"
      aria-label={title}
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div className="w-full max-w-3xl max-h-[90vh] overflow-y-auto rounded-xl border border-border bg-surface shadow-xl">
        <div className="flex items-center justify-between gap-3 border-b border-border p-4">
          <h2 className="text-sm font-bold text-text-primary">{title}</h2>
          <Button size="sm" variant="ghost" onClick={onClose}>
            Cerrar
          </Button>
        </div>
        <div className="p-4">{children}</div>
      </div>
    </div>
  );
}
