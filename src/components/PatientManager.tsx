import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "./ui/Button";
import { Input } from "./ui/Input";
import { Alert } from "./ui/Alert";
import { usePatientRegistryStore } from "../stores/usePatientRegistryStore";

interface PatientManagerProps {
  userId: string;
}

export function PatientManager({ userId }: PatientManagerProps) {
  // Estados del formulario
  const [fullName, setFullName] = useState("");
  const [identityDoc, setIdentityDoc] = useState("");
  const [birthDate, setBirthDate] = useState("");
  const [phone, setPhone] = useState("");
  const [email, setEmail] = useState("");
  const [address, setAddress] = useState("");

  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const { patients, isLoading, fetchPatients, addPatientLocally } =
    usePatientRegistryStore();

  useEffect(() => {
    fetchPatients();
  }, [fetchPatients]);

  const handleSubmit = async (e: React.SubmitEvent) => {
    e.preventDefault();
    setError(null);
    setLoading(true);
    try {
      // Enviamos todos los datos al backend de Rust
      const newId = await invoke<string>("create_patient", {
        form: {
          created_by_user_id: userId,
          full_name: fullName,
          identity_doc: identityDoc,
          // Enviamos los nuevos datos opcionales (si están vacíos, se envían como null)
          birth_date: birthDate || null,
          phone: phone || null,
          email: email || null,
          address: address || null,
        },
      });

      // Actualizamos el estado local de Zustand instantáneamente
      addPatientLocally({
        id: newId,
        full_name: fullName,
        created_at: new Date().toISOString(),
      });

      // Limpiamos el formulario
      setFullName("");
      setIdentityDoc("");
      setBirthDate("");
      setPhone("");
      setEmail("");
      setAddress("");
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
          <div className="p-3 text-xs mb-4">
            <Alert variant="error">{error}</Alert>
          </div>
        )}

        <form onSubmit={handleSubmit} className="flex flex-col gap-3">
          <Input
            label="Nombre Completo *"
            type="text"
            value={fullName}
            onChange={(e) => setFullName(e.target.value)}
            required
            placeholder="ej: Juan Pérez"
          />

          <Input
            label="Documento de Identidad (DNI / Pasaporte) *"
            type="text"
            value={identityDoc}
            onChange={(e) => setIdentityDoc(e.target.value)}
            required
            placeholder="ej: 95123456"
          />

          <div className="grid grid-cols-2 gap-3">
            <Input
              label="Fecha de Nacimiento"
              type="date"
              value={birthDate}
              onChange={(e) => setBirthDate(e.target.value)}
            />
            <Input
              label="Teléfono"
              type="tel"
              value={phone}
              onChange={(e) => setPhone(e.target.value)}
              placeholder="ej: 11 5555-5555"
            />
          </div>

          <Input
            label="Email"
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder="ej: paciente@correo.com"
          />

          <Input
            label="Dirección"
            type="text"
            value={address}
            onChange={(e) => setAddress(e.target.value)}
            placeholder="ej: Av. Siempre Viva 742"
          />

          <Button type="submit" isLoading={loading} className="w-full mt-2">
            Cifrar e Ingresar
          </Button>
        </form>
      </div>

      {/* Padrón de Pacientes */}
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
          <div className="flex flex-col gap-2.5 max-h-125 overflow-y-auto pr-1">
            {patients.map((p) => (
              <div
                key={p.id}
                className="bg-zinc-950 p-3 rounded border border-zinc-800 hover:border-zinc-700 transition-colors cursor-pointer"
                // En el futuro, aquí podrías agregar un onClick para seleccionar al paciente
                // onClick={() => usePatientStore.getState().selectPatient(p)}
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
