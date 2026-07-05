import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAuthStore } from "../stores/useAuthStore"; // ← NUEVO

type NoteFields = Record<string, string>;

interface NoteCreated {
  id: number;
  external_id: string;
  template_id: string;
  signature_hex: string;
}

interface NoteRecord {
  id: number;
  external_id?: string;
  entity_id: number;
  template_id: string;
  fields: NoteFields;
  is_verified: boolean;
  created_at: string;
}

export function NoteTest() {
  const { activeUser } = useAuthStore(); // ← NUEVO: obtener usuario logueado
  const [entityId, setEntityId] = useState("");
  const [fields, setFields] = useState<NoteFields>({
    subjetivo: "",
    objetivo: "",
    evaluacion: "",
    plan: "",
  });
  const [getId, setGetId] = useState("");
  const [result, setResult] = useState<string>("");
  const [loading, setLoading] = useState(false);

  // Si no hay usuario logueado, no permitir crear notas
  const userId = activeUser?.user_id;

  const handleFieldChange = (name: string, value: string) => {
    setFields((prev) => ({ ...prev, [name]: value }));
  };

  const handleCreate = async () => {
    if (!userId) {
      setResult("❌ Error: No hay usuario logueado");
      return;
    }

    setLoading(true);
    setResult("");
    try {
      const res = await invoke<NoteCreated>("create_note", {
        entityId: Number(entityId),
        templateId: "soap_v1",
        fields,
        userId, // ← PASAR EL UUID REAL DEL MÉDICO
      });
      setResult(
        `✅ Nota creada: ID=${res.id}\n` +
          `External ID: ${res.external_id.slice(0, 8)}\n` +
          `Template=${res.template_id}\n` +
          `Firma: ${res.signature_hex.slice(0, 16)}`,
      );
    } catch (err) {
      setResult(`❌ Error: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const handleGet = async () => {
    setLoading(true);
    setResult("");
    try {
      const res = await invoke<NoteRecord>("get_note", {
        id: Number(getId),
      });

      const fieldsText = Object.entries(res.fields)
        .map(
          ([k, v]) =>
            `  ${k}: ${v.slice(0, 100)}${v.length > 100 ? "..." : ""}`,
        )
        .join("\n");

      setResult(
        `📋 Nota #${res.id}\n` +
          `External ID: ${res.external_id?.slice(0, 8)}\n` +
          `Paciente: ${res.entity_id}\n` +
          `Template: ${res.template_id}\n` +
          `Verificada: ${res.is_verified ? "✅ SÍ" : "❌ NO"}\n` +
          `Creado: ${res.created_at}\n` +
          `Campos:\n${fieldsText}`,
      );
    } catch (err) {
      setResult(`❌ Error: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="p-6 max-w-2xl mx-auto space-y-6">
      <h2 className="text-xl font-bold text-zinc-300">
        🧪 Test: Note Commands (Template v1)
      </h2>

      {/* Crear nota */}
      <div className="space-y-3 border border-zinc-800 p-4 rounded-lg bg-zinc-900">
        <h3 className="font-semibold text-zinc-400">Crear Nota SOAP</h3>

        <div>
          <label className="block text-sm text-zinc-500 mb-1">
            ID del Paciente (Entity)
          </label>
          <input
            type="number"
            value={entityId}
            onChange={(e) => setEntityId(e.target.value)}
            className="w-full rounded bg-zinc-950 border border-zinc-700 px-3 py-2 text-sm text-zinc-200"
            placeholder="Ej: 1"
          />
        </div>

        {[
          { name: "subjetivo", label: "Subjetivo", rows: 4 },
          { name: "objetivo", label: "Objetivo", rows: 4 },
          { name: "evaluacion", label: "Evaluación", rows: 3 },
          { name: "plan", label: "Plan", rows: 3 },
        ].map((field) => (
          <div key={field.name}>
            <label className="block text-sm text-zinc-500 mb-1">
              {field.label}
            </label>
            <textarea
              value={fields[field.name]}
              onChange={(e) => handleFieldChange(field.name, e.target.value)}
              rows={field.rows}
              className="w-full rounded bg-zinc-950 border border-zinc-700 px-3 py-2 text-sm text-zinc-200 resize-y"
            />
          </div>
        ))}

        <button
          onClick={handleCreate}
          disabled={loading}
          className="rounded bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
        >
          {loading ? "Guardando..." : "Crear Nota + Firmar"}
        </button>
      </div>

      {/* Leer nota */}
      <div className="space-y-3 border border-zinc-800 p-4 rounded-lg bg-zinc-900">
        <h3 className="font-semibold text-zinc-400">Leer Nota por ID</h3>
        <input
          type="number"
          value={getId}
          onChange={(e) => setGetId(e.target.value)}
          placeholder="ID de nota"
          className="w-full rounded bg-zinc-950 border border-zinc-700 px-3 py-2 text-sm text-zinc-200"
        />
        <button
          onClick={handleGet}
          disabled={loading}
          className="rounded bg-emerald-600 px-4 py-2 text-sm font-medium text-white hover:bg-emerald-700 disabled:opacity-50"
        >
          Leer + Verificar Firma
        </button>
      </div>

      {/* Resultado */}
      {result && (
        <div className="whitespace-pre-wrap rounded-lg border border-zinc-800 bg-zinc-950 p-4 text-sm text-zinc-300 font-mono">
          {result}
        </div>
      )}
    </div>
  );
}
