// src/components/MedicalHistoryForm.tsx
import { useState, useEffect } from "react";
import { usePatientStore, type ClinicalData } from "../stores/usePatientStore";
import { MEDICAL_HISTORY_FIELDS } from "../config/medicalHistorySchema";

export function MedicalHistoryForm() {
  const { selected, saveClinical, isLoading } = usePatientStore();

  const [form, setForm] = useState<ClinicalData>({});
  const [isDirty, setIsDirty] = useState(false);
  const [saveStatus, setSaveStatus] = useState<"idle" | "saving" | "saved" | "error">("idle");
  const [errorMsg, setErrorMsg] = useState("");

  const patientId = selected?.patient?.id;
  const clinical = selected?.clinical;

  // Sincronizar formulario cuando cambia el paciente seleccionado o sus datos clínicos
  useEffect(() => {
    if (clinical) {
      setForm({ ...clinical });
    } else {
      // Limpiar formulario si no hay antecedentes previos
      const empty: ClinicalData = {};
      MEDICAL_HISTORY_FIELDS.forEach((f) => {
        empty[f.name as keyof ClinicalData] = "";
      });
      setForm(empty);
    }
    setIsDirty(false);
    setSaveStatus("idle");
    setErrorMsg("");
  }, [patientId, clinical]);

  const handleChange = (field: string, value: string) => {
    setForm((prev) => ({ ...prev, [field]: value }));
    setIsDirty(true);
    setSaveStatus("idle");
  };

  const handleSave = async () => {
    if (!patientId) return;

    setSaveStatus("saving");
    try {
      // Filtrar campos vacíos antes de guardar
      const cleanData = Object.fromEntries(
        Object.entries(form).filter(([, v]) => v && v.trim() !== "")
      ) as ClinicalData;

      await saveClinical(patientId, cleanData);
      setSaveStatus("saved");
      setIsDirty(false);

      // Resetear mensaje de éxito después de 3 segundos
      setTimeout(() => setSaveStatus("idle"), 3000);
    } catch (err) {
      setSaveStatus("error");
      setErrorMsg(String(err));
    }
  };

  // Estado vacío: no hay paciente seleccionado
  if (!patientId) {
    return (
      <div className="border border-zinc-800 border-dashed rounded-xl p-8 text-center bg-zinc-900/20">
        <div className="text-4xl mb-3 opacity-30">📋</div>
        <p className="text-zinc-500 font-medium">
          Selecciona un paciente en la pestaña{" "}
          <span className="font-mono bg-zinc-800 px-1.5 py-0.5 rounded text-xs text-zinc-300">
            Historia Clínica
          </span>{" "}
          para editar sus antecedentes
        </p>
      </div>
    );
  }

  const renderFieldInput = (field: any) => {
    const commonClasses =
      "w-full rounded-lg bg-zinc-950 border border-zinc-700 px-3 py-2.5 text-sm text-zinc-200 focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500 outline-none transition-all appearance-none";

    switch (field.type) {
      case "textarea":
        return (
          <textarea
            value={form[field.name as keyof ClinicalData] || ""}
            onChange={(e) => handleChange(field.name, e.target.value)}
            className={`${commonClasses} resize-y min-h-24`}
            placeholder={field.placeholder || field.label}
          />
        );
      case "select":
        return (
          <div className="relative">
            <select
              value={form[field.name as keyof ClinicalData] || ""}
              onChange={(e) => handleChange(field.name, e.target.value)}
              className={`${commonClasses} pr-8 cursor-pointer`}
            >
              <option value="" disabled>
                Seleccionar...
              </option>
              {field.options?.map((opt: string) => (
                <option
                  key={opt}
                  value={opt}
                  className="bg-zinc-900 text-zinc-200"
                >
                  {opt}
                </option>
              ))}
            </select>
            <div className="pointer-events-none absolute inset-y-0 right-0 flex items-center px-2 text-zinc-400">
              <svg
                className="fill-current h-4 w-4"
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 20 20"
              >
                <path d="M9.293 12.95l.707.707L15.657 8l-1.414-1.414L10 10.828 5.757 6.586 4.343 8z" />
              </svg>
            </div>
          </div>
        );
      default:
        return (
          <input
            type="text"
            value={form[field.name as keyof ClinicalData] || ""}
            onChange={(e) => handleChange(field.name, e.target.value)}
            className={commonClasses}
            placeholder={field.placeholder || field.label}
          />
        );
    }
  };

  return (
    <div className="space-y-6">
      {/* Cabecera del formulario */}
      <div className="flex justify-between items-center">
        <div>
          <h2 className="text-lg font-semibold mb-4 text-zinc-300 flex items-center gap-2">
            <span className="p-1.5 rounded bg-blue-900/20 text-blue-400">📝</span>
            Antecedentes Médicos
          </h2>
          <p className="text-lg text-zinc-500 mt-1">
            Paciente:{" "}
            <span className="text-zinc-300 font-medium">
              {selected.patient.data.nombre} {selected.patient.data.apellido}
            </span>
            <span className="ml-2 font-mono text-zinc-600">
              ID: {patientId}
            </span>
          </p>
        </div>

        {/* Indicador de estado de guardado */}
        {saveStatus === "saved" && (
          <span className="text-xs font-bold text-emerald-500 bg-emerald-500/10 px-3 py-1.5 rounded-full animate-in fade-in duration-200">
            ✅ Guardado
          </span>
        )}
        {saveStatus === "error" && (
          <span className="text-xs font-bold text-red-500 bg-red-500/10 px-3 py-1.5 rounded-full">
            ❌ Error
          </span>
        )}
      </div>

      {/* Mensaje de error */}
      {saveStatus === "error" && errorMsg && (
        <div className="rounded-lg border border-red-900/50 bg-red-900/10 p-3 text-sm text-red-300">
          {errorMsg}
        </div>
      )}

      {/* Campos dinámicos desde medical_history_schema.json */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {MEDICAL_HISTORY_FIELDS.map((field: any) => (
          <div
            key={field.name}
            className={field.type === "textarea" ? "col-span-2" : ""}
          >
            <label className="block text-xs font-bold text-zinc-500 mb-1.5 uppercase tracking-wide">
              {field.label}{" "}
              {field.required && <span className="text-red-500">*</span>}
            </label>
            {renderFieldInput(field)}
          </div>
        ))}
      </div>

      {/* Botón de guardar */}
      <div className="flex justify-end pt-2 border-t border-zinc-800">
        <button
          onClick={handleSave}
          disabled={!isDirty || saveStatus === "saving" || isLoading}
          className={`rounded-lg px-6 py-2.5 text-sm font-bold text-white shadow-lg transition-all flex items-center gap-2 ${
            !isDirty
              ? "bg-zinc-700 text-zinc-400 cursor-not-allowed shadow-none"
              : "bg-emerald-600 hover:bg-emerald-700 shadow-emerald-900/20"
          } disabled:opacity-50`}
        >
          {saveStatus === "saving" ? (
            <>
              <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
              Guardando...
            </>
          ) : (
            <>💾 Guardar Antecedentes</>
          )}
        </button>
      </div>
    </div>
  );
}
