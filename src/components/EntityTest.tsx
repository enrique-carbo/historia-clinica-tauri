import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type EntityData = Record<string, string>;

interface EntityCreated {
  id: number;
  entity_type: string;
  blind_index_hex: string;
}

interface EntityRecord {
  id: number;
  entity_type: string;
  data: EntityData;
  created_at: string;
}

export function EntityTest() {
  const [form, setForm] = useState<EntityData>({
    nombre: "",
    apellido: "",
    dni: "",
    fecha_nacimiento: "",
    telefono: "",
    email: "",
    direccion: "",
  });

  const [searchDni, setSearchDni] = useState("");
  const [getId, setGetId] = useState("");
  const [result, setResult] = useState<string>("");
  const [loading, setLoading] = useState(false);

  const handleChange = (field: string, value: string) => {
    setForm((prev) => ({ ...prev, [field]: value }));
  };

  const handleCreate = async () => {
    setLoading(true);
    setResult("");
    try {
      const data = Object.fromEntries(
        Object.entries(form).filter(([, v]) => v.trim() !== ""),
      );

      const res = await invoke<EntityCreated>("create_entity", {
        entityType: "patient",
        data,
      });

      setResult(`✅ Creado: ID=${res.id}, BlindIndex=${res.blind_index_hex}`);
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
        setGetId(String(id));
      } else {
        setResult("🔍 No encontrado");
      }
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
      const res = await invoke<EntityRecord>("get_entity", {
        id: Number(getId),
      });

      const fields = Object.entries(res.data)
        .map(([k, v]) => `  ${k}: ${v}`)
        .join("\n");

      setResult(
        `📋 Entidad #${res.id}\nTipo: ${res.entity_type}\nCreado: ${res.created_at}\nDatos:\n${fields}`,
      );
    } catch (err) {
      setResult(`❌ Error: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="p-6 max-w-2xl mx-auto space-y-6">
      <h2 className="text-xl font-bold text-slate-200">
        🧪 Test: Entity Commands (Template v1)
      </h2>

      {/* Crear entidad */}
      <div className="space-y-3 border p-4 rounded-lg bg-zinc-950">
        <h3 className="font-semibold text-slate-200">Crear Paciente</h3>
        {[
          { name: "nombre", label: "Nombre *", required: true },
          { name: "apellido", label: "Apellido *", required: true },
          { name: "dni", label: "DNI *", required: true },
          { name: "fecha_nacimiento", label: "Fecha Nacimiento" },
          { name: "telefono", label: "Teléfono" },
          { name: "email", label: "Email" },
          { name: "direccion", label: "Dirección" },
        ].map((field) => (
          <div key={field.name}>
            <label className="block text-sm font-medium text-slate-200">
              {field.label}
            </label>
            <input
              type="text"
              value={form[field.name]}
              onChange={(e) => handleChange(field.name, e.target.value)}
              className="text-white mt-1 w-full rounded border border-slate-300 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none"
            />
          </div>
        ))}
        <button
          onClick={handleCreate}
          disabled={loading}
          className="rounded bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
        >
          {loading ? "Guardando..." : "Crear Entidad"}
        </button>
      </div>

      {/* Buscar por DNI */}
      <div className="space-y-3 border p-4 rounded-lg bg-zinc-950">
        <h3 className="font-semibold text-slate-200">Buscar por DNI</h3>
        <input
          type="text"
          value={searchDni}
          onChange={(e) => setSearchDni(e.target.value)}
          placeholder="Ingrese DNI"
          className="text-white w-full rounded border border-slate-300 px-3 py-2 text-sm"
        />
        <button
          onClick={handleFind}
          disabled={loading}
          className="rounded bg-amber-600 px-4 py-2 text-sm font-medium text-white hover:bg-amber-700 disabled:opacity-50"
        >
          Buscar
        </button>
      </div>

      {/* Leer por ID */}
      <div className="space-y-3 border p-4 rounded-lg bg-zinc-950">
        <h3 className="font-semibold text-slate-200">Leer Entidad por ID</h3>
        <input
          type="number"
          value={getId}
          onChange={(e) => setGetId(e.target.value)}
          placeholder="ID de entidad"
          className="text-white w-full rounded border border-slate-300 px-3 py-2 text-sm"
        />
        <button
          onClick={handleGet}
          disabled={loading}
          className="rounded bg-emerald-600 px-4 py-2 text-sm font-medium text-white hover:bg-emerald-700 disabled:opacity-50"
        >
          Leer
        </button>
      </div>

      {/* Resultado */}
      {result && (
        <div className="whitespace-pre-wrap rounded-lg border border-slate-200 bg-slate-50 p-4 text-sm text-slate-800">
          {result}
        </div>
      )}
    </div>
  );
}
