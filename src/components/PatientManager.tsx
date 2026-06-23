import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface PatientManagerProps {
  userId: string;
}

export function PatientManager({ userId }: PatientManagerProps) {
  const [fullName, setFullName] = useState("");
  const [identityDoc, setIdentityDoc] = useState("");
  const [patients, setPatients] = useState<any[]>([]);
  const [error, setError] = useState<string | null>(null);

  const fetchPatients = async () => {
    try {
      const res = await invoke<any[]>("get_patients_list");
      setPatients(res);
    } catch (err) {
      console.error(err);
    }
  };

  useEffect(() => {
    fetchPatients();
  }, []);

  const handleSubmit = async (e: React.SubmitEvent) => {
    e.preventDefault();
    setError(null);

    try {
      await invoke("create_patient", {
        form: {
          created_by_user_id: userId,
          full_name: fullName,
          identity_doc: identityDoc,
        },
      });
      setFullName("");
      setIdentityDoc("");
      fetchPatients();
    } catch (err) {
      setError(String(err));
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
          <div>
            <label className="block text-xs text-zinc-500 mb-1 font-semibold">
              Nombre Completo
            </label>
            <input
              type="text"
              value={fullName}
              onChange={(e) => setFullName(e.target.value)}
              required
              className="w-full bg-zinc-950 text-zinc-200 border border-zinc-800 rounded px-3 py-2.5 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 placeholder:text-zinc-700"
              placeholder="ej: Juan Pérez"
            />
          </div>

          <div>
            <label className="block text-xs text-zinc-500 mb-1 font-semibold">
              Documento de Identidad (DNI / Pasaporte)
            </label>
            <input
              type="text"
              value={identityDoc}
              onChange={(e) => setIdentityDoc(e.target.value)}
              required
              className="w-full bg-zinc-950 text-zinc-200 border border-zinc-800 rounded px-3 py-2.5 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 placeholder:text-zinc-700"
              placeholder="ej: 95123456"
            />
          </div>

          <button
            type="submit"
            className="w-full py-2.5 bg-blue-600 hover:bg-blue-700 text-white text-sm font-semibold rounded transition-colors cursor-pointer mt-2"
          >
            Cifrar e Ingresar
          </button>
        </form>
      </div>

      <div className="bg-zinc-900 p-5 rounded-lg border border-zinc-800">
        <h3 className="text-base font-semibold text-zinc-100 mb-4">
          📇 Padrón de Pacientes Registrados
        </h3>

        <div className="flex flex-col gap-2.5">
          {patients.length === 0 ? (
            <p className="text-sm text-zinc-600">
              No hay pacientes en el sistema.
            </p>
          ) : (
            patients.map((p) => (
              <div
                key={p.id}
                className="bg-zinc-950 p-3 rounded border border-zinc-800"
              >
                <div>
                  <strong className="text-sm text-zinc-200">
                    {p.full_name}
                  </strong>
                </div>
                <div className="text-[11px] text-zinc-600 font-mono mt-1">
                  UUID: {p.id}
                </div>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
