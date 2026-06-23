import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Props {
  pacienteId: string;
  onSuccess: () => void;
}

export function MetricQuickForm({ pacienteId, onSuccess }: Props) {
  const [tipo, setTipo] = useState("antropometria");
  const [subTipo, setSubTipo] = useState("peso");
  const [valor, setValor] = useState("");

  const handleSubmit = async (e: React.SubmitEvent) => {
    e.preventDefault();
    try {
      await invoke("save_patient_metric", {
        form: {
          paciente_id: pacienteId,
          metric_type: tipo,
          sub_metric: subTipo,
          value_num: parseFloat(valor),
          value_text: "",
        },
      });
      setValor("");
      onSuccess();
    } catch (err) {
      console.error(err);
    }
  };

  return (
    <form
      onSubmit={handleSubmit}
      className="flex flex-col md:flex-row gap-3 items-end"
    >
      {/* Select Tipo */}
      <div className="w-full md:w-auto relative">
        <label className="block text-xs text-zinc-500 mb-1">Tipo</label>
        <select
          value={tipo}
          onChange={(e) => setTipo(e.target.value)}
          className="w-full bg-zinc-950 text-zinc-200 border border-zinc-800 rounded px-3 py-2 pr-8 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 appearance-none"
        >
          <option value="antropometria">Antropometría</option>
          <option value="presion">Presión Arterial</option>
        </select>
        {/* Icono de flecha custom */}
        <div className="pointer-events-none absolute inset-y-0 right-0 flex items-center pr-2 top-6">
          <svg
            className="h-4 w-4 text-zinc-500"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M19 9l-7 7-7-7"
            />
          </svg>
        </div>
      </div>

      {/* Select Variable */}
      <div className="w-full md:w-auto relative">
        <label className="block text-xs text-zinc-500 mb-1">Variable</label>
        <select
          value={subTipo}
          onChange={(e) => setSubTipo(e.target.value)}
          className="w-full bg-zinc-950 text-zinc-200 border border-zinc-800 rounded px-3 py-2 pr-8 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 appearance-none"
        >
          {tipo === "presion" ? (
            <>
              <option value="sistolica">Sistólica</option>
              <option value="diastolica">Diastólica</option>
            </>
          ) : (
            <option value="peso">Peso (kg)</option>
          )}
        </select>
        {/* Icono de flecha custom */}
        <div className="pointer-events-none absolute inset-y-0 right-0 flex items-center pr-2 top-6">
          <svg
            className="h-4 w-4 text-zinc-500"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M19 9l-7 7-7-7"
            />
          </svg>
        </div>
      </div>

      {/* Input Valor */}
      <div className="w-full md:flex-1">
        <label className="block text-xs text-zinc-500 mb-1">Valor</label>
        <input
          type="number"
          step="0.1"
          value={valor}
          onChange={(e) => setValor(e.target.value)}
          required
          placeholder="Ej: 80.5"
          className="w-full bg-zinc-950 text-zinc-200 border border-zinc-800 rounded px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 placeholder:text-zinc-700"
        />
      </div>

      {/* Botón */}
      <button
        type="submit"
        className="w-full md:w-auto px-5 py-2 bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium rounded transition-colors cursor-pointer"
      >
        Guardar
      </button>
    </form>
  );
}
