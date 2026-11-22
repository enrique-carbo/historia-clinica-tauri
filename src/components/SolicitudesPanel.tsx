// Panel del módulo de solicitudes de estudios (`docs/MODULES/module-solicitudes-v1.md`).
//
// Se monta dentro de la ficha del paciente. Si el módulo no está compilado,
// activo o el rol no tiene acceso, `is_module_available` devuelve false y no
// se renderiza nada. Toda la comunicación pasa por `module_execute`.
//
// Modelo v2: template = título + lista de estudios (checklist); solicitud =
// ítems tildados + diagnóstico + notas.

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

interface TemplateRecord {
  id: string;
  name: string;
  items: string[];
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
  const [selectedItems, setSelectedItems] = useState<string[]>([]);
  const [manualItems, setManualItems] = useState<string[]>([""]);
  const [diagnostico, setDiagnostico] = useState("");
  const [notas, setNotas] = useState("");
  const [isSaving, setIsSaving] = useState(false);

  const selectedTemplate = useMemo(
    () => templates.find((template) => template.id === templateId),
    [templates, templateId],
  );

  useEffect(() => {
    setSelectedItems([]);
    setStudyName(selectedTemplate?.name ?? "");
  }, [selectedTemplate]);

  const toggleItem = (item: string) => {
    setSelectedItems((current) =>
      current.includes(item)
        ? current.filter((selected) => selected !== item)
        : [...current, item],
    );
  };

  const submit = async () => {
    const items = selectedTemplate
      ? selectedItems
      : manualItems.map((item) => item.trim()).filter(Boolean);

    if (items.length === 0) {
      onError("Seleccioná o agregá al menos un estudio.");
      return;
    }

    const payload = {
      template_id: templateId || null,
      study_name: templateId ? null : studyName.trim(),
      items,
      diagnostico: diagnostico.trim(),
      notas: notas.trim(),
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
          <option value="">Sin template (elegir estudios a mano)</option>
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
        <div className="space-y-2">
          <div className="flex items-center justify-between gap-2">
            <span className="text-xs font-semibold text-text-secondary">
              Estudios del template
            </span>
            <div className="flex gap-1">
              <Button
                size="sm"
                variant="ghost"
                onClick={() => setSelectedItems(selectedTemplate.items)}
              >
                Todos
              </Button>
              <Button size="sm" variant="ghost" onClick={() => setSelectedItems([])}>
                Ninguno
              </Button>
            </div>
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-1.5">
            {selectedTemplate.items.map((item) => (
              <label
                key={item}
                className="flex items-center gap-2 rounded border border-border bg-surface px-2.5 py-1.5 text-sm text-text-primary cursor-pointer hover:bg-surface-raised/50"
              >
                <input
                  type="checkbox"
                  checked={selectedItems.includes(item)}
                  onChange={() => toggleItem(item)}
                />
                <span className="truncate">{item}</span>
              </label>
            ))}
          </div>
          {selectedTemplate.items.length === 0 && (
            <p className="text-[11px] text-text-muted">
              El template no tiene estudios cargados.
            </p>
          )}
        </div>
      ) : (
        <div className="space-y-2">
          <Input
            label="Título de la solicitud"
            value={studyName}
            onChange={(event) => setStudyName(event.target.value)}
            placeholder="Ej. Cultivo de orina"
          />
          <span className="block text-xs font-semibold text-text-secondary">
            Estudios
          </span>
          {manualItems.map((item, index) => (
            <div key={index} className="flex items-end gap-2">
              <div className="flex-1">
                <Input
                  aria-label={`Estudio ${index + 1}`}
                  value={item}
                  onChange={(event) =>
                    setManualItems((current) =>
                      current.map((value, i) =>
                        i === index ? event.target.value : value,
                      ),
                    )
                  }
                  placeholder="Ej. Hemograma completo"
                />
              </div>
              <Button
                size="sm"
                variant="ghost-danger"
                onClick={() =>
                  setManualItems((current) =>
                    current.length === 1
                      ? [""]
                      : current.filter((_, i) => i !== index),
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
            onClick={() => setManualItems((current) => [...current, ""])}
          >
            Agregar estudio
          </Button>
        </div>
      )}

      <Textarea
        label="Diagnóstico / indicación"
        value={diagnostico}
        rows={2}
        onChange={(event) => setDiagnostico(event.target.value)}
        placeholder="Motivo clínico de la solicitud"
      />
      <Textarea
        label="Notas"
        value={notas}
        rows={2}
        onChange={(event) => setNotas(event.target.value)}
        placeholder="Indicaciones adicionales (ayuno, preparación, etc.)"
      />

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

// =======================================================================
// TEMPLATES
// =======================================================================

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
  const [items, setItems] = useState<string[]>([]);
  const [isSaving, setIsSaving] = useState(false);

  const startCreate = () => {
    setEditingId(null);
    setName("");
    setItems([""]);
  };

  const startEdit = (template: TemplateRecord) => {
    setEditingId(template.id);
    setName(template.name);
    setItems(template.items.length > 0 ? template.items : [""]);
  };

  const reset = () => {
    setEditingId(null);
    setName("");
    setItems([]);
  };

  const save = async () => {
    if (!name.trim()) {
      onError("El template necesita un nombre.");
      return;
    }

    const cleanItems = items.map((item) => item.trim()).filter(Boolean);
    if (cleanItems.length === 0) {
      onError("Agregá al menos un estudio al template.");
      return;
    }
    if (new Set(cleanItems).size !== cleanItems.length) {
      onError("Hay estudios repetidos en el template.");
      return;
    }

    setIsSaving(true);
    try {
      if (editingId) {
        await moduleExecute(MODULE_ID, "template.update", {
          id: editingId,
          name: name.trim(),
          items: cleanItems,
        });
      } else {
        await moduleExecute(MODULE_ID, "template.create", {
          name: name.trim(),
          items: cleanItems,
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

  const isEditing = items.length > 0;

  return (
    <Dialog title="Mis templates de estudios" onClose={onClose}>
      {isEditing ? (
        <div className="space-y-3">
          <Input
            label="Nombre del template"
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder="Ej. Laboratorio general"
          />

          <div className="space-y-2">
            <span className="block text-xs font-semibold text-text-secondary">
              Estudios
            </span>
            {items.map((item, index) => (
              <div key={index} className="flex items-end gap-2">
                <div className="flex-1">
                  <Input
                    aria-label={`Estudio ${index + 1}`}
                    value={item}
                    onChange={(event) =>
                      setItems((current) =>
                        current.map((value, i) =>
                          i === index ? event.target.value : value,
                        ),
                      )
                    }
                    placeholder="Ej. Hemograma completo"
                  />
                </div>
                <Button
                  size="sm"
                  variant="ghost-danger"
                  onClick={() =>
                    setItems((current) =>
                      current.length === 1
                        ? [""]
                        : current.filter((_, i) => i !== index),
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
              onClick={() => setItems((current) => [...current, ""])}
            >
              Agregar estudio
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
              hint="Un template es un título con la lista de estudios que pedís seguido"
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
                      {template.items.length} estudio
                      {template.items.length === 1 ? "" : "s"}
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
