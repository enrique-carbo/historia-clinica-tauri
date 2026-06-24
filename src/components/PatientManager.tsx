import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "./ui/Button";
import { Input } from "./ui/Input"; // 🚀 Importamos el Design System
import { usePatientRegistryStore } from "../stores/usePatientRegistryStore";

interface PatientManagerProps {
  userId: string;
}

export function PatientManager({ userId }: PatientManagerProps) {
  const [fullName, setFullName] = useState("");
  const [identityDoc, setIdentityDoc] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const { patients, isLoading, fetchPatients, addPatientLocally } =
    usePatientRegistryStore();

  useEffect(() => {
    fetchPatients();
  }, [fetchPatients]);

  const handleSubmit = async (e: React.SubmitEvent) => {
    // ✅ Cambié a FormEvent que es el estándar para <form>
    e.preventDefault();
    setError(null);
    setLoading(true);
    try {
      const newId = await invoke<string>("create_patient", {
        form: {
          created_by_user_id: userId,
          full_name: fullName,
          identity_doc: identityDoc,
        },
      });

      addPatientLocally({
        id: newId,
        full_name: fullName,
        created_at: new Date().toISOString(),
      });

      setFullName("");
      setIdentityDoc("");
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-10">
      {/* Formulario de Alta */}
      <div className="bg-zinc-900 p-5 rounded-lg border border-zinc-800">
        <h3 className="text-base font-semibold text-zinc-100 mb-4">
          ➕ Dar de Alta Paciente
        </h3>

        {error && (
          <div className="bg-red-950 border border-red-800 text-red-400 p-3 rounded text-xs mb-4">
            {error}
          </div>
        )}

        <form onSubmit={handleSubmit} className="flex flex-col gap-3">
          {/* ✅ MAGIA DEL DESIGN SYSTEM: 5 líneas de código en lugar de 12 */}
          <Input
            label="Nombre Completo"
            type="text"
            value={fullName}
            onChange={(e) => setFullName(e.target.value)}
            required
            placeholder="ej: Juan Pérez"
          />

          <Input
            label="Documento de Identidad (DNI / Pasaporte)"
            type="text"
            value={identityDoc}
            onChange={(e) => setIdentityDoc(e.target.value)}
            required
            placeholder="ej: 95123456"
          />

          <Button type="submit" isLoading={loading} className="w-full mt-2">
            Cifrar e Ingresar
          </Button>
        </form>
      </div>

      {/* Padrón de Pacientes (Sin cambios, ya estaba perfecto) */}
      <div className="bg-zinc-900 p-5 rounded-lg border border-zinc-800">
        <h3 className="text-base font-semibold text-zinc-100 mb-4">
          📇 Padrón de Pacientes Registrados
        </h3>

        {isLoading ? (
          <p className="text-sm text-zinc-600 animate-pulse">
            Cargando padrón cifrado...
          </p>
        ) : patients.length === 0 ? (
          <p className="text-sm text-zinc-600">
            No hay pacientes en el sistema.
          </p>
        ) : (
          <div className="flex flex-col gap-2.5 max-h-75 overflow-y-auto pr-1">
            {patients.map((p) => (
              <div
                key={p.id}
                className="bg-zinc-950 p-3 rounded border border-zinc-800 hover:border-zinc-700 transition-colors"
              >
                <strong className="text-sm text-zinc-200">{p.full_name}</strong>
                <div
                  className="text-[11px] text-zinc-600 font-mono mt-1 truncate"
                  title={p.id}
                >
                  UUID: {p.id}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
