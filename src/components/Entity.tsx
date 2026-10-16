// src/components/Entity.tsx
// Orquestador del padrón: estado y handlers; la UI vive en
// PatientListPanel (izq.), PatientForm y PatientAccessPanel (der.).
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAuthStore } from "../stores/useAuthStore";
import { usePatientStore } from "../stores/usePatientStore";
import { PATIENT_FIELDS, flattenFields } from "../utils/schema";
import { PatientListPanel, type EntityRecord } from "./PatientListPanel";
import { PatientForm } from "./PatientForm";
import { PatientAccessPanel } from "./PatientAccessPanel";

type EntityData = Record<string, string>;

interface EntityCreated {
  id: number;
  external_id: string;
  entity_type: string;
  blind_index_hex: string;
}

// Vista aplanada (con prefijo de groups) para prefill/reset del form.
const FLAT_FIELDS = flattenFields(PATIENT_FIELDS);

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

  const canManageAccess =
    activeUser?.role === "administrador" || activeUser?.role === "asistente";

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

  return (
    <div className="grid grid-cols-1 md:grid-cols-12 gap-6 h-auto md:h-[calc(100vh-180px)]">
      {/* PANEL IZQUIERDO: Búsqueda y Listado */}
      <PatientListPanel
        entities={entities}
        loading={loading}
        searchQuery={searchQuery}
        searchDni={searchDni}
        selectedId={selected?.patient?.id ?? null}
        onSearchQueryChange={setSearchQuery}
        onSearchDniChange={setSearchDni}
        onSearch={handleSearch}
        onFind={handleFind}
        onSelect={handleSelect}
        onEdit={loadEntityForEdit}
      />

      {/* PANEL DERECHO: Formulario Dinámico */}
      <div className="col-span-1 md:col-span-7 lg:col-span-8 flex flex-col gap-4 overflow-hidden">
        {result && (
          <div className="whitespace-pre-wrap rounded-lg border border-border bg-canvas p-3 text-sm text-text-primary font-mono shadow-inner animate-in fade-in duration-200 overflow-x-auto">
            {result}
          </div>
        )}

        <PatientForm
          form={form}
          editingId={editingId}
          loading={loading}
          onChange={handleChange}
          onReset={resetForm}
          onSubmit={handleCreateOrUpdate}
        />

        {/* Acceso del paciente (admin/asistente, solo sobre ficha existente) */}
        <PatientAccessPanel
          editingId={editingId}
          canManageAccess={canManageAccess}
        />
      </div>
    </div>
  );
}
