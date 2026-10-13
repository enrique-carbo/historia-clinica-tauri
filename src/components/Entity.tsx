// src/components/Entity.tsx
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAuthStore } from "../stores/useAuthStore";
import { usePatientStore } from "../stores/usePatientStore";
import { PATIENT_FIELDS, PATIENT_LABEL, flattenFields } from "../utils/schema";
import { Button } from "./ui/Button";

type EntityData = Record<string, string>;

interface EntityCreated {
  id: number;
  external_id: string;
  entity_type: string;
  blind_index_hex: string;
}

interface EntityRecord {
  id: number;
  external_id?: string;
  entity_type: string;
  data: EntityData;
  created_at: string;
}

interface PatientAccess {
  user_id: string;
  username: string;
  entity_id: number;
  is_active: boolean;
}

const FIELDS_CONFIG = PATIENT_FIELDS;
// Vista aplanada (con prefijo de groups) para prefill/reset del form.
const FLAT_FIELDS = flattenFields(FIELDS_CONFIG);

export function Entity() {
  const { activeUser, isVaultUnlocked } = useAuthStore();
  const { selected, selectPatient } = usePatientStore();

  const [form, setForm] = useState<EntityData>({});
  const [editingId, setEditingId] = useState<number | null>(null);
  const [searchDni, setSearchDni] = useState("");
  const [searchQuery, setSearchQuery] = useState("");
  const [result, setResult] = useState<string>("");
  const [entities, setEntities] = useState<EntityRecord[]>([]);
  const [loading, setLoading] = useState(false);

  // Acceso de paciente (alta lazy desde la ficha)
  const canManageAccess =
    activeUser?.role === "administrador" || activeUser?.role === "asistente";
  const [access, setAccess] = useState<PatientAccess | null>(null);
  const [accessLoading, setAccessLoading] = useState(false);
  const [showAccessForm, setShowAccessForm] = useState(false);
  const [accessUsername, setAccessUsername] = useState("");
  const [accessPassword, setAccessPassword] = useState("");
  const [accessMsg, setAccessMsg] = useState<{
    type: "ok" | "err";
    text: string;
  } | null>(null);

  // Cargar estado del acceso al editar una ficha existente
  useEffect(() => {
    if (!editingId || !canManageAccess) return;
    let cancelled = false;
    setAccess(null);
    setAccessMsg(null);
    setShowAccessForm(false);
    setAccessLoading(true);
    invoke<PatientAccess | null>("get_entity_access", { entityId: editingId })
      .then((res) => {
        if (!cancelled) setAccess(res);
      })
      .catch((e) => {
        if (!cancelled) setAccessMsg({ type: "err", text: String(e) });
      })
      .finally(() => {
        if (!cancelled) setAccessLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [editingId, canManageAccess]);

  const handleGrantAccess = async () => {
    if (!editingId) return;
    setAccessMsg(null);
    if (accessPassword.length < 8) {
      setAccessMsg({
        type: "err",
        text: "La contraseña debe tener al menos 8 caracteres.",
      });
      return;
    }
    setAccessLoading(true);
    try {
      const res = await invoke<PatientAccess>("grant_patient_access", {
        entityId: editingId,
        username: accessUsername,
        passwordPlain: accessPassword,
      });
      setAccess(res);
      setShowAccessForm(false);
      setAccessUsername("");
      setAccessPassword("");
      setAccessMsg({ type: "ok", text: `Acceso otorgado: @${res.username}` });
    } catch (e) {
      setAccessMsg({ type: "err", text: String(e) });
    } finally {
      setAccessLoading(false);
    }
  };

  // Sincronizar formulario cuando cambia la selección global (modo edición)
  useEffect(() => {
    if (editingId && selected?.patient && selected.patient.id === editingId) {
      const formData: EntityData = {};
      FLAT_FIELDS.forEach((field) => {
        formData[field.name] = selected.patient.data[field.name] || "";
      });
      setForm(formData);
    }
  }, [selected?.patient, editingId]);

  const handleChange = (field: string, value: string) => {
    setForm((prev) => ({ ...prev, [field]: value }));
  };

  const resetForm = () => {
    const clean: EntityData = {};
    FLAT_FIELDS.forEach((field) => {
      clean[field.name] = "";
    });
    setForm(clean);
    setEditingId(null);
  };

  const handleCreateOrUpdate = async () => {
    if (!isVaultUnlocked || !activeUser) {
      setResult("❌ Error: No hay usuario logueado");
      return;
    }
    setLoading(true);
    setResult("");
    try {
      const data = Object.fromEntries(
        Object.entries(form).filter(([, v]) => v.trim() !== ""),
      );
      if (editingId) {
        await invoke("update_entity", {
          id: editingId,
          data,
        });
        setResult(`✅ Actualizado: ID=${editingId}`);
        // Recargar selección global para reflejar cambios
        await selectPatient(editingId);
      } else {
        const res = await invoke<EntityCreated>("create_entity", {
          entityType: "patient",
          data,
        });
        setResult(
          `✅ Creado: ID=${res.id}, BI=${res.blind_index_hex.slice(0, 12)}`,
        );
        // Seleccionar automáticamente el nuevo paciente
        await selectPatient(res.id);
      }
      resetForm();
      handleList();
    } catch (err) {
      setResult(`❌ Error: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const handleFind = async () => {
    setLoading(true);
    setResult("");
    try {
      const id = await invoke<number | null>("find_entity_by_blind_index", {
        entityType: "patient",
        dni: searchDni,
      });
      if (id) {
        setResult(`🔍 Encontrado: ID=${id}`);
        await handleSelect(id);
      } else {
        setResult("🔍 No encontrado");
      }
    } catch (err) {
      setResult(`❌ Error: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const loadEntityForEdit = async (id: number) => {
    setLoading(true);
    try {
      // Usar el store global para cargar el paciente
      await selectPatient(id);
      setEditingId(id);

      const patient = usePatientStore.getState().selected?.patient;
      if (patient) {
        const formData: EntityData = {};
        FLAT_FIELDS.forEach((field) => {
          formData[field.name] = patient.data[field.name] || "";
        });
        setForm(formData);
        setResult(
          `📋 Editando: ${patient.data.nombre} ${patient.data.apellido} (ID: ${patient.id})`,
        );
      }
    } catch (err) {
      setResult(`❌ Error: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const handleSelect = async (id: number) => {
    // Selección global: actualiza EHR y este componente
    await selectPatient(id);
    setResult(`👁️ Seleccionado: ID=${id}. Haz clic en "Editar" para modificar.`);
  };

  const handleList = async () => {
    setLoading(true);
    setResult("");
    try {
      const res = await invoke<EntityRecord[]>("list_entities", {
        entityType: "patient",
        limit: 50,
        offset: 0,
      });
      setEntities(res);
      setResult(`📋 Listado: ${res.length} pacientes`);
    } catch (err) {
      setResult(`❌ Error: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const handleSearch = async () => {
    setLoading(true);
    setResult("");
    try {
      const res = await invoke<EntityRecord[]>("search_entities", {
        entityType: "patient",
        query: searchQuery,
      });
      setEntities(res);
      setResult(`🔍 Búsqueda: ${res.length} coincidencias`);
    } catch (err) {
      setResult(`❌ Error: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const renderFieldInput = (field: any) => {
    const commonClasses =
      "w-full rounded-lg bg-zinc-950 border border-zinc-700 px-3 py-2.5 text-sm text-zinc-200 focus:border-blue-500 focus:ring-1 focus:ring-blue-500 outline-none transition-colors appearance-none";
    switch (field.type) {
      case "textarea":
        return (
          <textarea
            value={form[field.name] || ""}
            onChange={(e) => handleChange(field.name, e.target.value)}
            className={`${commonClasses} resize-y min-h-24`}
            placeholder={field.placeholder || field.label}
          />
        );
      case "select":
        return (
          <div className="relative">
            <select
              value={form[field.name] || ""}
              onChange={(e) => handleChange(field.name, e.target.value)}
              className={`${commonClasses} pr-8 cursor-pointer`}
            >
              <option value="" disabled>
                Seleccionar...
              </option>
              {field.options?.map((opt: string) => (
                <option key={opt} value={opt} className="bg-zinc-900 text-zinc-200">
                  {opt}
                </option>
              ))}
            </select>
            <div className="pointer-events-none absolute inset-y-0 right-0 flex items-center px-2 text-zinc-400">
              <svg className="fill-current h-4 w-4" viewBox="0 0 20 20">
                <path d="M9.293 12.95l.707.707L15.657 8l-1.414-1.414L10 10.828 5.757 6.586 4.343 8z" />
              </svg>
            </div>
          </div>
        );
      default:
        return (
          <input
            type={field.type === "date" ? "date" : "text"}
            value={form[field.name] || ""}
            onChange={(e) => handleChange(field.name, e.target.value)}
            className={commonClasses}
            placeholder={field.placeholder || field.label}
          />
        );
    }
  };

  const selectedId = selected?.patient?.id ?? null;

  return (
    <div className="grid grid-cols-1 md:grid-cols-12 gap-6 h-auto md:h-[calc(100vh-180px)]">
      {/* PANEL IZQUIERDO: Búsqueda y Listado */}
      <div className="col-span-1 md:col-span-5 lg:col-span-4 flex flex-col border border-zinc-800 rounded-xl bg-zinc-900/50 backdrop-blur-sm overflow-hidden shadow-lg min-h-100 md:min-h-0">
        <div className="p-4 border-b border-zinc-800 space-y-3 bg-zinc-900">
          <h3 className="text-xs font-bold text-zinc-500 uppercase tracking-widest">
            Padrón de Pacientes
          </h3>

          {/* Búsqueda por Nombre / Teléfono */}
          <div className="flex gap-2">
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleSearch()}
              placeholder="Buscar por nombre, teléfono..."
              className="w-full min-w-0 rounded-lg bg-zinc-950 border border-zinc-700 px-3.5 py-2 text-sm text-zinc-200 focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-colors outline-none"
            />
            <button
              onClick={handleSearch}
              disabled={loading}
              className="shrink-0 rounded-lg bg-amber-600 px-3.5 py-2 text-sm font-medium text-white hover:bg-amber-700 disabled:opacity-50 transition-colors"
            >
              🔍
            </button>
          </div>

          {/* Búsqueda por DNI */}
          <div className="flex gap-2">
            <input
              type="text"
              value={searchDni}
              onChange={(e) => setSearchDni(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleFind()}
              placeholder="DNI exacto..."
              className="w-full min-w-0 flex-1 rounded-lg bg-zinc-950 border border-zinc-700 px-3.5 py-2 text-sm text-zinc-200 focus:border-amber-500 focus:ring-1 focus:ring-amber-500 transition-colors outline-none"
            />
            <button
              onClick={handleFind}
              disabled={loading}
              className="shrink-0 rounded-lg bg-amber-600 px-3.5 py-2 text-sm font-medium text-white hover:bg-amber-700 disabled:opacity-50 transition-colors"
            >
              🔍
            </button>
          </div>
        </div>

        {/* Lista de Pacientes */}
        <div className="flex-1 overflow-y-auto p-2 space-y-1 custom-scrollbar max-h-[50vh] md:max-h-none">
          {loading ? (
            <div className="flex justify-center py-8">
              <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-500"></div>
            </div>
          ) : entities.length === 0 ? (
            <div className="text-center py-12 text-zinc-500 text-sm px-4">
              {searchQuery || searchDni
                ? "No se encontraron coincidencias"
                : "Busca un paciente o crea uno nuevo"}
            </div>
          ) : (
            entities.map((e) => (
              <div
                key={e.id}
                className={`w-full text-left p-3 rounded-lg transition-colors duration-200 group flex items-center justify-between gap-2 ${
                  selectedId === e.id
                    ? "bg-blue-900/20 border border-blue-500/50 shadow-md"
                    : "bg-zinc-950/50 border border-transparent hover:bg-zinc-800 hover:border-zinc-700"
                }`}
              >
                <div className="flex-1 min-w-0 cursor-pointer" onClick={() => handleSelect(e.id)}>
                  <div className="flex justify-between items-start gap-2">
                    <div
                      className={`font-semibold transition-colors truncate ${
                        selectedId === e.id
                          ? "text-blue-400"
                          : "text-zinc-200 group-hover:text-white"
                      }`}
                    >
                      {e.data.nombre} {e.data.apellido}
                    </div>
                    {selectedId === e.id && (
                      <span className="h-2 w-2 shrink-0 rounded-full bg-blue-500 mt-1.5"></span>
                    )}
                  </div>
                  <div className="text-xs text-zinc-500 mt-1.5 flex flex-wrap items-center gap-2">
                    <span className="bg-zinc-800 px-1.5 py-0.5 rounded text-zinc-400 font-mono">
                      DNI: {e.data.dni}
                    </span>
                    <span className="truncate">{e.data.telefono}</span>
                  </div>
                </div>

                {/* En móviles siempre visible, en desktop sólo en hover */}
                <button
                  onClick={() => loadEntityForEdit(e.id)}
                  className="shrink-0 px-2.5 py-1.5 rounded-lg bg-zinc-800 text-zinc-400 hover:text-white hover:bg-amber-600 transition-[color,background-color,border-color,opacity] text-xs font-bold border border-zinc-700 hover:border-amber-500 opacity-100 md:opacity-0 md:group-hover:opacity-100"
                  title="Editar paciente"
                >
                  ✏️
                </button>
              </div>
            ))
          )}
        </div>
      </div>

      {/* PANEL DERECHO: Formulario Dinámico */}
      <div className="col-span-1 md:col-span-7 lg:col-span-8 flex flex-col gap-4 overflow-hidden">
        {result && (
          <div className="whitespace-pre-wrap rounded-lg border border-zinc-800 bg-zinc-950 p-3 text-sm text-zinc-300 font-mono shadow-inner animate-in fade-in duration-200 overflow-x-auto">
            {result}
          </div>
        )}

        <div
          className={`flex-1 overflow-y-auto p-4 sm:p-6 border rounded-xl shadow-lg transition-colors custom-scrollbar ${
            editingId
              ? "border-amber-700/50 bg-amber-900/10"
              : "border-zinc-800 bg-zinc-900/50"
          }`}
        >
          <div className="flex justify-between items-center mb-6">
            <h3 className="font-semibold text-zinc-300 uppercase tracking-wider text-sm flex items-center gap-2">
              {editingId ? "✏️ Editar" : "➕ Nuevo"} {PATIENT_LABEL}
            </h3>
            {editingId && (
              <button
                onClick={resetForm}
                className="text-xs text-zinc-400 hover:text-white underline transition-colors"
              >
                Cancelar Edición
              </button>
            )}
          </div>

          {/* Grid responsivo de inputs: 1 col en móviles, 2 col en tablets/desktops */}
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            {FIELDS_CONFIG.map((field: any) =>
              field.type === "group" ? (
                <div
                  key={field.name}
                  className="col-span-1 sm:col-span-2 rounded-lg border border-zinc-800 bg-zinc-950/40 p-4"
                >
                  <h4 className="text-xs font-bold text-zinc-400 uppercase tracking-wide mb-3">
                    {field.label}{" "}
                    {field.required && <span className="text-red-500">*</span>}
                  </h4>
                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                    {field.fields.map((sub: any) => {
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
                          <label className="block text-xs font-bold text-zinc-500 mb-1.5 uppercase tracking-wide">
                            {prefixed.label}{" "}
                            {prefixed.required && (
                              <span className="text-red-500">*</span>
                            )}
                          </label>
                          {renderFieldInput(prefixed)}
                        </div>
                      );
                    })}
                  </div>
                </div>
              ) : (
                <div
                  key={field.name}
                  className={
                    field.type === "textarea"
                      ? "col-span-1 sm:col-span-2"
                      : "col-span-1"
                  }
                >
                  <label className="block text-xs font-bold text-zinc-500 mb-1.5 uppercase tracking-wide">
                    {field.label}{" "}
                    {field.required && <span className="text-red-500">*</span>}
                  </label>
                  {renderFieldInput(field)}
                </div>
              ),
            )}
          </div>

          {/* Footer sticky responsivo */}
          <div className="pt-6 mt-4 flex flex-col-reverse sm:flex-row justify-end gap-3 sticky bottom-0 bg-linear-to-t from-zinc-900 via-zinc-900 to-transparent pb-2">
            {editingId && (
              <Button
                variant="secondary"
                onClick={resetForm}
                className="w-full sm:w-auto"
              >
                Cancelar
              </Button>
            )}
            <Button
              onClick={handleCreateOrUpdate}
              disabled={loading}
              className="w-full sm:w-auto"
            >
              {loading ? "Procesando..." : editingId ? "Guardar Cambios" : "Crear y Cifrar"}
            </Button>
          </div>
        </div>

        {/* Acceso del paciente (admin/asistente, solo sobre ficha existente) */}
        {editingId && canManageAccess && (
          <div className="shrink-0 rounded-xl border border-zinc-800 bg-zinc-900/50 p-4 sm:p-6">
            <div className="flex items-center justify-between gap-3 mb-4">
              <h3 className="font-semibold text-zinc-300 uppercase tracking-wider text-sm">
                🎟️ Acceso del paciente
              </h3>
              {accessMsg && (
                <span
                  className={`text-xs font-medium ${
                    accessMsg.type === "ok" ? "text-emerald-400" : "text-red-400"
                  }`}
                >
                  {accessMsg.type === "ok" ? "✓ " : "✕ "}
                  {accessMsg.text}
                </span>
              )}
            </div>

            {accessLoading ? (
              <div className="flex items-center gap-2 text-sm text-zinc-500">
                <span className="animate-spin inline-block h-4 w-4 rounded-full border-b-2 border-blue-500"></span>
                Consultando…
              </div>
            ) : access && access.is_active ? (
              <p className="text-sm text-zinc-400">
                Esta ficha ya tiene acceso activo:{" "}
                <span className="font-mono text-zinc-200">@{access.username}</span>
                . Para revocarlo, desactivalo desde el panel de Usuarios.
              </p>
            ) : (
              <>
                {access && !access.is_active && (
                  <p className="text-sm text-zinc-500 mb-3">
                    Acceso anterior{" "}
                    <span className="font-mono">@{access.username}</span>{" "}
                    (inactivo).
                  </p>
                )}
                {showAccessForm ? (
                  <div className="grid grid-cols-1 sm:grid-cols-[1fr_1fr_auto] gap-3 items-end">
                    <div>
                      <label className="block text-xs font-semibold text-zinc-500 mb-1.5 uppercase">
                        Usuario
                      </label>
                      <input
                        value={accessUsername}
                        onChange={(e) => setAccessUsername(e.target.value)}
                        placeholder="ej. maria.gomez"
                        className="w-full rounded-lg bg-zinc-950 border border-zinc-700 px-3 py-2.5 text-sm text-zinc-200 focus:border-blue-500 outline-none transition-colors"
                      />
                    </div>
                    <div>
                      <label className="block text-xs font-semibold text-zinc-500 mb-1.5 uppercase">
                        Contraseña temporal
                      </label>
                      <input
                        type="password"
                        value={accessPassword}
                        onChange={(e) => setAccessPassword(e.target.value)}
                        placeholder="mínimo 8 caracteres"
                        className="w-full rounded-lg bg-zinc-950 border border-zinc-700 px-3 py-2.5 text-sm text-zinc-200 focus:border-blue-500 outline-none transition-colors"
                      />
                    </div>
                    <div className="flex gap-2">
                      <Button
                        variant="secondary"
                        onClick={() => {
                          setShowAccessForm(false);
                          setAccessMsg(null);
                        }}
                      >
                        Cancelar
                      </Button>
                      <Button
                        onClick={handleGrantAccess}
                        disabled={accessLoading || !accessUsername.trim()}
                      >
                        Otorgar
                      </Button>
                    </div>
                  </div>
                ) : (
                  <div className="flex items-center justify-between gap-3">
                    <p className="text-sm text-zinc-500">
                      Todavía no tiene usuario para entrar a la app.
                    </p>
                    <Button onClick={() => setShowAccessForm(true)}>
                      Dar acceso al paciente
                    </Button>
                  </div>
                )}
              </>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
