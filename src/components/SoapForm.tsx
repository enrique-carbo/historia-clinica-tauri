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

  const handleChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const { name, value } = e.target;
    setForm((prev) => ({ ...prev, [name]: value }));
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    try {
      await invoke("save_soap_consultation", {
        form: { paciente_id: pacienteId, medico_id: medicoId, ...form },
      });
      setForm({ subjetivo: "", objetivo: "", analisis: "", plan: "" });
      onSuccess(); // Avisa a App.tsx que hay que recargar el historial
    } catch (err) {
      alert(`Error en Core: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} style={formStyle}>
      <h3 style={{ margin: "0 0 10px 0", color: "#f4f4f5", fontSize: "16px" }}>
        Nueva Evolución (SOAP)
      </h3>

      <div>
        <label style={labelStyle}>[S] Subjetivo</label>
        <textarea
          name="subjetivo"
          value={form.subjetivo}
          onChange={handleChange}
          required
          rows={3}
          style={textareaStyle}
          placeholder="Síntomas relatados por el paciente..."
        />
      </div>

      <div>
        <label style={labelStyle}>[O] Objetivo</label>
        <textarea
          name="objetivo"
          value={form.objetivo}
          onChange={handleChange}
          required
          rows={3}
          style={textareaStyle}
          placeholder="Examen físico y signos vitales..."
        />
      </div>

      <div>
        <label style={labelStyle}>[A] Análisis</label>
        <textarea
          name="analisis"
          value={form.analisis}
          onChange={handleChange}
          required
          rows={2}
          style={textareaStyle}
          placeholder="Diagnóstico presuntivo o hipótesis..."
        />
      </div>

      <div>
        <label style={labelStyle}>[P] Plan</label>
        <textarea
          name="plan"
          value={form.plan}
          onChange={handleChange}
          required
          rows={3}
          style={textareaStyle}
          placeholder="Tratamiento, indicaciones y fármacos..."
        />
      </div>

      <button type="submit" disabled={loading} style={buttonStyle}>
        {loading ? "Cifrando en RAM..." : "Guardar en Registro Local"}
      </button>
    </form>
  );
}

// Estilos modulares reutilizables
const formStyle: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: "16px",
  background: "#18181b",
  padding: "20px",
  borderRadius: "8px",
  border: "1px solid #27272a",
};
const labelStyle: React.CSSProperties = {
  display: "block",
  fontSize: "13px",
  color: "#a1a1aa",
  marginBottom: "4px",
  fontWeight: "600",
};
const textareaStyle: React.CSSProperties = {
  width: "100%",
  padding: "10px",
  borderRadius: "6px",
  border: "1px solid #27272a",
  background: "#09090b",
  color: "#f4f4f5",
  fontSize: "14px",
  fontFamily: "inherit",
  resize: "vertical",
  boxSizing: "border-box",
};
const buttonStyle: React.CSSProperties = {
  padding: "10px 20px",
  background: "#2563eb",
  color: "#fff",
  border: "none",
  borderRadius: "6px",
  fontWeight: "600",
  cursor: "pointer",
  alignSelf: "flex-end",
};
