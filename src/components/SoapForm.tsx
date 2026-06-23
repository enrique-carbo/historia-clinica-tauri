import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface SoapFormProps {
  pacienteId: string;
  medicoId: string;
  onSuccess: () => void;
}

export function SoapForm({ pacienteId, medicoId, onSuccess }: SoapFormProps) {
  const [form, setForm] = useState({
    subjetivo: "",
    objetivo: "",
    analisis: "",
    plan: "",
  });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const { name, value } = e.target;
    setForm((prev) => ({ ...prev, [name]: value }));
    if (error) setError(null); // Limpia el error si el médico empieza a escribir de nuevo
  };

  const handleSubmit = async (e: React.SubmitEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    try {
      await invoke("save_soap_consultation", {
        form: { paciente_id: pacienteId, medico_id: medicoId, ...form },
      });
      setForm({ subjetivo: "", objetivo: "", analisis: "", plan: "" });
      onSuccess();
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <form
      onSubmit={handleSubmit}
      className="flex flex-col gap-4 bg-zinc-900 p-5 rounded-lg border border-zinc-800"
    >
      <h3 className="text-base font-semibold text-zinc-100 mb-0">
        Nueva Evolución (SOAP)
      </h3>

      {error && (
        <div className="bg-red-950 border border-red-800 text-red-400 p-3 rounded text-xs">
          Error al cifrar: {error}
        </div>
      )}

      <div>
        <label className="block text-xs text-zinc-500 mb-1 font-semibold">
          [S] Subjetivo
        </label>
        <textarea
          name="subjetivo"
          value={form.subjetivo}
          onChange={handleChange}
          required
          rows={3}
          className="w-full bg-zinc-950 text-zinc-200 border border-zinc-800 rounded px-3 py-2 text-sm font-[inherit] resize-y focus:outline-none focus:ring-1 focus:ring-blue-500 placeholder:text-zinc-700"
          placeholder="Síntomas relatados por el paciente..."
        />
      </div>

      <div>
        <label className="block text-xs text-zinc-500 mb-1 font-semibold">
          [O] Objetivo
        </label>
        <textarea
          name="objetivo"
          value={form.objetivo}
          onChange={handleChange}
          required
          rows={3}
          className="w-full bg-zinc-950 text-zinc-200 border border-zinc-800 rounded px-3 py-2 text-sm font-[inherit] resize-y focus:outline-none focus:ring-1 focus:ring-blue-500 placeholder:text-zinc-700"
          placeholder="Examen físico y signos vitales..."
        />
      </div>

      <div>
        <label className="block text-xs text-zinc-500 mb-1 font-semibold">
          [A] Análisis
        </label>
        <textarea
          name="analisis"
          value={form.analisis}
          onChange={handleChange}
          required
          rows={2}
          className="w-full bg-zinc-950 text-zinc-200 border border-zinc-800 rounded px-3 py-2 text-sm font-[inherit] resize-y focus:outline-none focus:ring-1 focus:ring-blue-500 placeholder:text-zinc-700"
          placeholder="Diagnóstico presuntivo o hipótesis..."
        />
      </div>

      <div>
        <label className="block text-xs text-zinc-500 mb-1 font-semibold">
          [P] Plan
        </label>
        <textarea
          name="plan"
          value={form.plan}
          onChange={handleChange}
          required
          rows={3}
          className="w-full bg-zinc-950 text-zinc-200 border border-zinc-800 rounded px-3 py-2 text-sm font-[inherit] resize-y focus:outline-none focus:ring-1 focus:ring-blue-500 placeholder:text-zinc-700"
          placeholder="Tratamiento, indicaciones y fármacos..."
        />
      </div>

      <button
        type="submit"
        disabled={loading}
        className="self-end px-5 py-2.5 bg-blue-600 hover:bg-blue-700 disabled:bg-blue-900 disabled:cursor-not-allowed text-white text-sm font-semibold rounded transition-colors cursor-pointer"
      >
        {loading ? "Cifrando en RAM..." : "Guardar en Registro Local"}
      </button>
    </form>
  );
}
