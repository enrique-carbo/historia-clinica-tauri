import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "./ui/Button";
import { Textarea } from "./ui/Textarea";

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
    if (error) setError(null);
  };

  // ✅ Estándar de React 19 (sin warnings de deprecación)
  const handleSubmit = async (e: React.SubmitEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    try {
      // ✅ Escritura explícita para evitar errores de compilación
      await invoke("save_soap_consultation", {
        form: {
          paciente_id: pacienteId,
          medico_id: medicoId,
          subjetivo: form.subjetivo,
          objetivo: form.objetivo,
          analisis: form.analisis,
          plan: form.plan,
        },
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

      {/* ✅ MAGIA DEL DESIGN SYSTEM: Puro contenido, cero HTML repetido */}
      <Textarea
        label="[S] Subjetivo"
        name="subjetivo"
        value={form.subjetivo}
        onChange={handleChange}
        required
        rows={3}
        placeholder="Síntomas relatados por el paciente..."
      />

      <Textarea
        label="[O] Objetivo"
        name="objetivo"
        value={form.objetivo}
        onChange={handleChange}
        required
        rows={3}
        placeholder="Examen físico y signos vitales..."
      />

      <Textarea
        label="[A] Análisis"
        name="analisis"
        value={form.analisis}
        onChange={handleChange}
        required
        rows={2}
        placeholder="Diagnóstico presuntivo o hipótesis..."
      />

      <Textarea
        label="[P] Plan"
        name="plan"
        value={form.plan}
        onChange={handleChange}
        required
        rows={3}
        placeholder="Tratamiento, indicaciones y fármacos..."
      />

      <Button type="submit" isLoading={loading} className="self-end mt-2">
        {loading ? "Cifrando en RAM..." : "Guardar en Registro Local"}
      </Button>
    </form>
  );
}
