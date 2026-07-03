import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "./ui/Button";
import { Input } from "./ui/Input";
import { Alert } from "./ui/Alert";
import { useAuthStore } from "../stores/useAuthStore";

interface Props {
  pacienteId: string;
  onSuccess: () => void;
}

export function MetricQuickForm({ pacienteId, onSuccess }: Props) {
  const [tipo, setTipo] = useState("antropometria");
  const [subTipo, setSubTipo] = useState("peso");
  const [valor, setValor] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const { activeUser } = useAuthStore();

  // 🚀 Resetear la variable dependiente cuando cambia el tipo principal
  const handleTipoChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const newTipo = e.target.value;
    setTipo(newTipo);
    // Si cambia a antropometría, forzamos que el subtipo sea "peso"
    if (newTipo === "antropometria") setSubTipo("peso");
    // Si cambia a presión, forzamos que sea "sistolica"
    if (newTipo === "presion") setSubTipo("sistolica");
  };

  const handleSubmit = async (e: React.SubmitEvent) => {
    e.preventDefault();
    setError(null);
    setLoading(true);

    if (!activeUser) {
      console.error("No hay un médico activo para firmar esta métrica.");
      return;
    }

    try {
      await invoke("save_patient_metric", {
        form: {
          paciente_id: pacienteId,
          medico_id: activeUser.user_id,
          metric_type: tipo,
          sub_metric: subTipo,
          value_num: parseFloat(valor),
          value_text: "",
        },
      });
      setValor("");
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
      className="grid grid-cols-1 md:grid-cols-2 gap-3 items-end"
    >
      {/* 🚀 Mostrar error si lo hay - Ocupa todo el ancho en md */}
      {error && (
        <div className="text-xs col-span-1 md:col-span-2">
          <Alert variant="error"> Error al guardar métrica: {error} </Alert>
        </div>
      )}

      {/* Select Tipo - Fila 1 / Columna 1 */}
      <div className="w-full relative">
        <label className="block text-xs text-zinc-500 mb-1">Tipo</label>
        <select
          value={tipo}
          onChange={handleTipoChange}
          className="w-full bg-zinc-950 text-zinc-200 border border-zinc-800 rounded px-3 py-2 pr-8 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 appearance-none"
        >
          <option value="antropometria">Antropometría</option>
          <option value="presion">Presión Arterial</option>
        </select>
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

      {/* Select Variable - Fila 1 / Columna 2 */}
      <div className="w-full relative">
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

      {/* Input Valor Design System - Fila 2 / Columna 1 */}
      <div className="w-full">
        <Input
          type="number"
          label="Valor"
          step="0.1"
          value={valor}
          onChange={(e) => setValor(e.target.value)}
          required
          placeholder="Ej: 80.5"
        />
      </div>

      {/* 🚀 Botón del Design System - Fila 2 / Columna 2 */}
      <Button
        type="submit"
        isLoading={loading}
        className="w-full h-9.5 flex items-center justify-center whitespace-nowrap"
      >
        {loading ? "Guardando..." : "Registrar"}
      </Button>
    </form>
  );
}
