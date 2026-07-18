// src/components/Entity.tsx

import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAuthStore } from "../stores/useAuthStore";
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

  // Estado del formulario
  const [form, setForm] = useState<EntityData>(() => {
    const initial: EntityData = {};
    FIELDS_CONFIG.forEach((field: any) => {
      initial[field.name] = "";
    });
    return initial;
  });

  const [editingId, setEditingId] = useState<number | null>(null);
  const [selectedId, setSelectedId] = useState<number | null>(null); // Nuevo estado para selección visual

  const [searchDni, setSearchDni] = useState("");
  const [searchQuery, setSearchQuery] = useState("");
  const [result, setResult] = useState<string>("");
  const [entities, setEntities] = useState<EntityRecord[]>([]);
  const [loading, setLoading] = useState(false);

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
        Object.entries(form).filter(([, v]) => v.trim() !== "")
      );

      if (editingId) {
        await invoke("update_entity", {
          id: editingId,
          data,
          userId: activeUser.user_id,
        });
        setResult(`✅ Actualizado: ID=${editingId}`);
      } else {
        const res = await invoke<EntityCreated>("create_entity", {
          entityType: "patient",
          data,
          userId: activeUser.user_id,
        });
        setResult(`✅ Creado: ID=${res.id}, BlindIndex=${res.blind_index_hex.slice(0, 12)}`);
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
      } else {
        setResult("🔍 No encontrado");
      }
    } catch (err) {
      setResult(`❌ Error: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  // Esta función ahora solo carga los datos para editar si se llama explícitamente
  const loadEntityForEdit = async (id: number) => {
    setLoading(true);
    try {
      const res = await invoke<EntityRecord>("get_entity", {
        id: Number(id),
      });

      const formData: EntityData = {};
      FIELDS_CONFIG.forEach((field: any) => {
        formData[field.name] = res.data[field.name] || "";
      });

      setForm(formData);
      setEditingId(res.id);
      setResult(`📋 Editando: ${res.data.nombre} ${res.data.apellido} (ID: ${res.id})`);
    } catch (err) {
      setResult(`❌ Error: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const handleSelect = (id: number) => {
    setSelectedId(id);
    // Opcional: Mostrar un mensaje simple de selección sin cargar el formulario
    setResult(`👁️ Seleccionado: ID=${id}. Haz clic en "Editar" para modificar.`);
  };

  const handleList = async () => {
    setLoading(true);
    setResult("");
    try {
      const res = await invoke<EntityRecord[]>("list_entities", {
        entityType: "patient",
        limit: 20,
        offset: 0,
      });
      setEntities(res);
      setResult(`📋 Listado: ${res.length} entidades`);
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
    const commonClasses = "w-full rounded bg-zinc-950 border border-zinc-700 px-3 py-2 text-sm text-zinc-200 focus:border-blue-500 outline-none transition-colors appearance-none";

    switch (field.type) {
      case 'textarea':
        return (
          <textarea
            value={form[field.name]}
            onChange={(e) => handleChange(field.name, e.target.value)}
            className={`${commonClasses} resize-y min-h-10`}
            placeholder={field.label}
          />
        );
      case 'select':
        return (
          <div className="relative">
            <select
              value={form[field.name]}
              onChange={(e) => handleChange(field.name, e.target.value)}
              className={`${commonClasses} pr-8 cursor-pointer`}
            >
              <option value="" disabled>Seleccionar...</option>
              {field.options?.map((opt: string) => (
                <option key={opt} value={opt} className="bg-zinc-900 text-zinc-200">{opt}</option>
              ))}
            </select>
            <div className="pointer-events-none absolute inset-y-0 right-0 flex items-center px-2 text-zinc-400">
              <svg className="fill-current h-4 w-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20"><path d="M9.293 12.95l.707.707L15.657 8l-1.414-1.414L10 10.828 5.757 6.586 4.343 8z"/></svg>
            </div>
          </div>
        );
      default:
        return (
          <input
            type={field.type === 'date' ? 'date' : 'text'}
            value={form[field.name]}
            onChange={(e) => handleChange(field.name, e.target.value)}
            className={commonClasses}
            placeholder={field.label}
          />
        );
    }
  };

  return (
    <div className="p-6 max-w-3xl mx-auto space-y-8">
      <div className="border-b border-zinc-800 pb-4 flex justify-between items-center">
        <div>
          <h2 className="text-2xl font-bold text-zinc-200 flex items-center gap-2">
            🧪 Sandbox de Entidades <span className="text-xs font-mono text-blue-500 bg-blue-900/20 px-2 py-1 rounded">Template v1</span>
          </h2>
          <p className="text-sm text-zinc-500 mt-1">
            Configuración cargada dinámicamente desde <code className="bg-zinc-800 px-1 rounded">schema.json</code>
          </p>
        </div>
        {editingId && (
          <button
            onClick={resetForm}
            className="text-xs text-zinc-400 hover:text-white underline"
          >
            Cancelar Edición
          </button>
        )}
      </div>

      {/* Formulario Dinámico */}
      <div className={`space-y-4 border p-6 rounded-xl shadow-lg transition-colors ${editingId ? 'border-amber-700/50 bg-amber-900/10' : 'border-zinc-800 bg-zinc-900/50'}`}>
        <h3 className="font-semibold text-zinc-300 uppercase tracking-wider text-sm mb-4 flex items-center gap-2">
          {editingId ? '✏️ Editar Paciente' : '➕ Nueva Entidad'}: {PATIENT_SCHEMA.label}
        </h3>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {FIELDS_CONFIG.map((field: any) => (
            <div key={field.name} className={field.type === 'textarea' ? 'col-span-2' : ''}>
              <label className="block text-xs font-bold text-zinc-500 mb-1.5 uppercase">
                {field.label} {field.required && <span className="text-red-500">*</span>}
              </label>
              {renderFieldInput(field)}
            </div>
          ))}
        </div>

        <div className="pt-4 flex justify-end gap-3">
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
                ? 'bg-amber-600 hover:bg-amber-700 shadow-amber-900/20'
                : 'bg-blue-600 hover:bg-blue-700 shadow-blue-900/20'
            } disabled:opacity-50`}
          >
            {loading ? "Procesando..." : editingId ? "Guardar Cambios" : "Crear y Cifrar"}
          </button>
        </div>
      </div>

      {/* Herramientas de Búsqueda y Gestión */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* Búsqueda por Blind Index (DNI) */}
        <div className="space-y-3 border border-zinc-800 p-4 rounded-xl bg-zinc-900/30">
          <h3 className="font-semibold text-zinc-400 text-sm">Búsqueda Exacta (Blind Index)</h3>
          <div className="flex gap-2">
            <input
              type="text"
              value={searchDni}
              onChange={(e) => setSearchDni(e.target.value)}
              placeholder="DNI exacto..."
              className="flex-1 rounded bg-zinc-950 border border-zinc-700 px-3 py-2 text-sm text-zinc-200"
            />
            <button
              onClick={handleFind}
              disabled={loading}
              className="rounded bg-amber-600 px-4 py-2 text-sm font-medium text-white hover:bg-amber-700 disabled:opacity-50"
            >
              Buscar
            </button>
          </div>
        </div>

        {/* Búsqueda por Texto (Substring) */}
        <div className="space-y-3 border border-zinc-800 p-4 rounded-xl bg-zinc-900/30">
          <h3 className="font-semibold text-zinc-400 text-sm">Búsqueda General</h3>
          <div className="flex gap-2">
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Nombre, apellido, teléfono..."
              className="flex-1 rounded bg-zinc-950 border border-zinc-700 px-3 py-2 text-sm text-zinc-200"
            />
            <button
              onClick={handleSearch}
              disabled={loading}
              className="rounded bg-pink-600 px-4 py-2 text-sm font-medium text-white hover:bg-pink-700 disabled:opacity-50"
            >
              Buscar
            </button>
          </div>
        </div>
      </div>

      {/* Resultado de Operaciones */}
      {result && (
        <div className="whitespace-pre-wrap rounded-lg border border-zinc-800 bg-zinc-950 p-4 text-sm text-zinc-300 font-mono shadow-inner">
          {result}
        </div>
      )}

      {/* Listado de Resultados con Botón Editar */}
      {entities.length > 0 && (
        <div className="space-y-3">
          <h3 className="font-semibold text-zinc-400 text-sm uppercase tracking-wider">Resultados Recientes</h3>
          <div className="grid gap-3">
            {entities.map((e) => (
              <div
                key={e.id}
                className={`border p-4 rounded-xl transition-all group flex justify-between items-center ${
                  selectedId === e.id
                    ? 'border-blue-500 bg-blue-900/10 ring-1 ring-blue-500/50'
                    : 'border-zinc-800 bg-zinc-900 hover:bg-zinc-800'
                }`}
              >
                <div
                  className="flex-1 cursor-pointer"
                  onClick={() => handleSelect(e.id)}
                >
                  <div className={`font-bold text-lg transition-colors ${selectedId === e.id ? 'text-blue-400' : 'text-white group-hover:text-blue-400'}`}>
                    {e.data.nombre} {e.data.apellido}
                  </div>
                  <div className="text-sm text-zinc-400 mt-1">
                    DNI: <span className="text-zinc-300 font-mono">{e.data.dni}</span>
                  </div>
                </div>

                {/* Botón de Edición Específico */}
                <button
                  onClick={() => loadEntityForEdit(e.id)}
                  className="ml-4 px-3 py-1.5 rounded-lg bg-zinc-800 text-zinc-400 hover:text-white hover:bg-amber-600 transition-colors text-xs font-bold uppercase tracking-wide border border-zinc-700 hover:border-amber-500"
                >
                  ✏️ Editar
                </button>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
