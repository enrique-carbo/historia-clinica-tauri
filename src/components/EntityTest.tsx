import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

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
  const [searchQuery, setSearchQuery] = useState("");
  const [getId, setGetId] = useState("");
  const [result, setResult] = useState<string>("");
  const [entities, setEntities] = useState<EntityRecord[]>([]);
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

      setResult(
        `✅ Creado: ID=${res.id}, External ID: ${res.external_id}, BlindIndex=${res.blind_index_hex.slice(0, 12)}`,
      );
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

  // ← NUEVO: Listar entidades
  const handleList = async () => {
    setLoading(true);
    setResult("");
    try {
      const res = await invoke<EntityRecord[]>("list_entities", {
        entityType: "patient",
        limit: 10,
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

  // ← NUEVO: Buscar entidades por texto
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
    <div className="p-6 max-w-2xl mx-auto space-y-6">
      <h2 className="text-xl font-bold text-zinc-300">
        🧪 Test: Entity Commands (Template v1)
      </h2>

      {/* Crear entidad */}
      <div className="space-y-3 border border-zinc-800 p-4 rounded-lg bg-zinc-900">
        <h3 className="font-semibold text-zinc-400">Crear Paciente</h3>
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
            <label className="block text-sm text-zinc-500 mb-1">
              {field.label}
            </label>
            <input
              type="text"
              value={form[field.name]}
              onChange={(e) => handleChange(field.name, e.target.value)}
              className="w-full rounded bg-zinc-950 border border-zinc-700 px-3 py-2 text-sm text-zinc-200"
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

      {/* Buscar por DNI (blind index exacto) */}
      <div className="space-y-3 border border-zinc-800 p-4 rounded-lg bg-zinc-900">
        <h3 className="font-semibold text-zinc-400">
          Buscar por DNI (Blind Index)
        </h3>
        <input
          type="text"
          value={searchDni}
          onChange={(e) => setSearchDni(e.target.value)}
          placeholder="Ingrese DNI"
          className="w-full rounded bg-zinc-950 border border-zinc-700 px-3 py-2 text-sm text-zinc-200"
        />
        <button
          onClick={handleFind}
          disabled={loading}
          className="rounded bg-amber-600 px-4 py-2 text-sm font-medium text-white hover:bg-amber-700 disabled:opacity-50"
        >
          Buscar Exacto
        </button>
      </div>

      {/* Leer por ID */}
      <div className="space-y-3 border border-zinc-800 p-4 rounded-lg bg-zinc-900">
        <h3 className="font-semibold text-zinc-400">Leer Entidad por ID</h3>
        <input
          type="number"
          value={getId}
          onChange={(e) => setGetId(e.target.value)}
          placeholder="ID de entidad"
          className="w-full rounded bg-zinc-950 border border-zinc-700 px-3 py-2 text-sm text-zinc-200"
        />
        <button
          onClick={handleGet}
          disabled={loading}
          className="rounded bg-emerald-600 px-4 py-2 text-sm font-medium text-white hover:bg-emerald-700 disabled:opacity-50"
        >
          Leer
        </button>
      </div>

      {/* ← NUEVO: Listar y Buscar */}
      <div className="space-y-3 border border-zinc-800 p-4 rounded-lg bg-zinc-900">
        <h3 className="font-semibold text-zinc-400">Listado y Búsqueda</h3>

        <div className="flex gap-2">
          <button
            onClick={handleList}
            disabled={loading}
            className="rounded bg-purple-600 px-4 py-2 text-sm font-medium text-white hover:bg-purple-700 disabled:opacity-50"
          >
            Listar Últimos 10
          </button>
        </div>

        <div className="flex gap-2">
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Buscar por nombre, apellido, etc."
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

      {/* Resultado de operación */}
      {result && (
        <div className="whitespace-pre-wrap rounded-lg border border-zinc-800 bg-zinc-950 p-4 text-sm text-zinc-300 font-mono">
          {result}
        </div>
      )}

      {/* ← NUEVO: Lista de entidades */}
      {entities.length > 0 && (
        <div className="space-y-2">
          <h3 className="font-semibold text-zinc-400">Resultados:</h3>
          {entities.map((e) => (
            <div
              key={e.id}
              className="border border-zinc-800 p-3 rounded bg-zinc-900 hover:bg-zinc-800 transition-colors cursor-pointer"
              onClick={() => setGetId(String(e.id))}
            >
              <div className="flex justify-between items-center">
                <span className="text-white font-medium">
                  {e.data.nombre} {e.data.apellido}
                </span>
                <span className="text-xs text-zinc-500">ID: {e.id}</span>
                <span className="text-xs text-zinc-500">
                  External_ID: {e.external_id?.slice(0, 8)}
                </span>
              </div>
              <div className="text-sm text-zinc-400">DNI: {e.data.dni}</div>
              <div className="text-xs text-zinc-600">{e.created_at}</div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
