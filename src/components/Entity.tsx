// src/components/Entity.tsx
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAuthStore } from "../stores/useAuthStore";
import { usePatientStore } from "../stores/usePatientStore";
import schemaConfig from "../config/schema.json";

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

const PATIENT_SCHEMA = schemaConfig.entities.patient;
const FIELDS_CONFIG = PATIENT_SCHEMA.fields;

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

  // Sincronizar formulario cuando cambia la selección global (modo edición)
  useEffect(() => {
    if (editingId && selected?.patient && selected.patient.id === editingId) {
      const formData: EntityData = {};
      FIELDS_CONFIG.forEach((field: any) => {
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
    FIELDS_CONFIG.forEach((field: any) => {
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
          userId: activeUser.user_id,
        });
        setResult(`✅ Actualizado: ID=${editingId}`);
        // Recargar selección global para reflejar cambios
        await selectPatient(editingId);
      } else {
        const res = await invoke<EntityCreated>("create_entity", {
          entityType: "patient",
          data,
          userId: activeUser.user_id,
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
        //await loadEntityForEdit(id);
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
      // Usar el store global para cargar paciente + clinical
      await selectPatient(id);
      setEditingId(id);

      const patient = usePatientStore.getState().selected?.patient;
      if (patient) {
        const formData: EntityData = {};
        FIELDS_CONFIG.forEach((field: any) => {
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
    // Selección global: actualiza EHR, MedicalHistoryForm y este componente
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
      "w-full rounded-lg bg-zinc-950 border border-zinc-700 px-3 py-2.5 text-sm text-zinc-200 focus:border-blue-500 focus:ring-1 focus:ring-blue-500 outline-none transition-all appearance-none";
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
    <div className="grid grid-cols-12 gap-6 h-[calc(100vh-180px)]">
      {/* PANEL IZQUIERDO: Búsqueda y Listado (compartido visualmente con EHR) */}
      <div className="col-span-4 flex flex-col border border-zinc-800 rounded-xl bg-zinc-900/50 backdrop-blur-sm overflow-hidden shadow-lg">
        <div className="p-4 border-b border-zinc-800 space-y-3 bg-zinc-900">
          <h3 className="text-xs font-bold text-zinc-500 uppercase tracking-widest">
            Padrón de Pacientes
          </h3>
          <div className="flex gap-2">
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleSearch()}
            placeholder="Buscar por nombre, teléfono..."
            className="w-full rounded-lg bg-zinc-950 border border-zinc-700 px-4 py-2.5 text-sm text-zinc-200 focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition-all outline-none"
            />
            <button
              onClick={handleSearch}
              disabled={loading}
              className="rounded-lg bg-amber-600 px-4 py-2 text-sm font-medium text-white hover:bg-amber-700 disabled:opacity-50 transition-colors"
            >
              🔍
            </button>
          </div>
          <div className="flex gap-2">
            <input
              type="text"
              value={searchDni}
              onChange={(e) => setSearchDni(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleFind()}
              placeholder="DNI exacto..."
              className="flex-1 rounded-lg bg-zinc-950 border border-zinc-700 px-3 py-2 text-sm text-zinc-200 focus:border-amber-500 focus:ring-1 focus:ring-amber-500 transition-all outline-none"
            />
            <button
              onClick={handleFind}
              disabled={loading}
              className="rounded-lg bg-amber-600 px-4 py-2 text-sm font-medium text-white hover:bg-amber-700 disabled:opacity-50 transition-colors"
            >
              🔍
            </button>
          </div>
        </div>
        <div className="flex-1 overflow-y-auto p-2 space-y-1 custom-scrollbar">
          {loading ? (
            <div className="flex justify-center py-8">
              <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-500"></div>
            </div>
          ) : entities.length === 0 ? (
            <div className="text-center py-12 text-zinc-600 text-sm">
              {searchQuery || searchDni
                ? "No se encontraron coincidencias"
                : "Busca un paciente o crea uno nuevo"}
            </div>
          ) : (
            entities.map((e) => (
              <div
                key={e.id}
                className={`w-full text-left p-3 rounded-lg transition-all duration-200 group flex justify-between items-center ${
                  selectedId === e.id
                    ? "bg-blue-900/20 border border-blue-500/50 shadow-md"
                    : "bg-zinc-950/50 border border-transparent hover:bg-zinc-800 hover:border-zinc-700"
                }`}
              >
                <div className="flex-1 cursor-pointer" onClick={() => handleSelect(e.id)}>
                  <div className="flex justify-between items-start">
                    <div
                      className={`font-semibold transition-colors ${
                        selectedId === e.id
                          ? "text-blue-400"
                          : "text-zinc-200 group-hover:text-white"
                      }`}
                    >
                      {e.data.nombre} {e.data.apellido}
                    </div>
                    {selectedId === e.id && (
                      <span className="h-2 w-2 rounded-full bg-blue-500 mt-1.5"></span>
                    )}
                  </div>
                  <div className="text-xs text-zinc-500 mt-1.5 flex items-center gap-2">
                    <span className="bg-zinc-800 px-1.5 py-0.5 rounded text-zinc-400">
                      DNI: {e.data.dni}
                    </span>
                    <span>{e.data.telefono}</span>
                  </div>
                </div>
                <button
                  onClick={() => loadEntityForEdit(e.id)}
                  className="ml-3 px-2.5 py-1.5 rounded-lg bg-zinc-800 text-zinc-400 hover:text-white hover:bg-amber-600 transition-colors text-xs font-bold uppercase tracking-wide border border-zinc-700 hover:border-amber-500 opacity-0 group-hover:opacity-100"
                >
                  ✏️
                </button>
              </div>
            ))
          )}
        </div>
      </div>

      {/* PANEL DERECHO: Formulario Dinámico */}
      <div className="col-span-8 flex flex-col gap-4 overflow-hidden">
        {result && (
          <div className="whitespace-pre-wrap rounded-lg border border-zinc-800 bg-zinc-950 p-3 text-sm text-zinc-300 font-mono shadow-inner animate-in fade-in duration-200">
            {result}
          </div>
        )}
        <div
          className={`flex-1 overflow-y-auto border p-6 rounded-xl shadow-lg transition-colors custom-scrollbar ${
            editingId
              ? "border-amber-700/50 bg-amber-900/10"
              : "border-zinc-800 bg-zinc-900/50"
          }`}
        >
          <div className="flex justify-between items-center mb-6">
            <h3 className="font-semibold text-zinc-300 uppercase tracking-wider text-sm flex items-center gap-2">
              {editingId ? "✏️ Editar" : "➕ Nuevo"} {PATIENT_SCHEMA.label}
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
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {FIELDS_CONFIG.map((field: any) => (
              <div key={field.name} className={field.type === "textarea" ? "col-span-2" : ""}>
                <label className="block text-xs font-bold text-zinc-500 mb-1.5 uppercase tracking-wide">
                  {field.label} {field.required && <span className="text-red-500">*</span>}
                </label>
                {renderFieldInput(field)}
              </div>
            ))}
          </div>
          <div className="pt-6 flex justify-end gap-3 sticky bottom-0 bg-linear-to-t from-zinc-900 via-zinc-900 to-transparent pb-2">
            {editingId && (
              <button
                onClick={resetForm}
                className="rounded-lg bg-zinc-700 px-6 py-2.5 text-sm font-bold text-white hover:bg-zinc-600 transition-all"
              >
                Cancelar
              </button>
            )}
            <button
              onClick={handleCreateOrUpdate}
              disabled={loading}
              className={`rounded-lg px-6 py-2.5 text-sm font-bold text-white shadow-lg transition-all ${
                editingId
                  ? "bg-amber-600 hover:bg-amber-700 shadow-amber-900/20"
                  : "bg-blue-600 hover:bg-blue-700 shadow-blue-900/20"
              } disabled:opacity-50`}
            >
              {loading ? "Procesando..." : editingId ? "Guardar Cambios" : "Crear y Cifrar"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
