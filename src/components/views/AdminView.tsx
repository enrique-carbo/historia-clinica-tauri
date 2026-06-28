import { PatientManager } from "../../components/PatientManager";

interface AdminViewProps {
  userId: string;
}

export function AdminView({ userId }: AdminViewProps) {
  return (
    <div>
      <h2 className="text-lg font-semibold mb-5 text-zinc-300">
        Consola de Recepción y Admisión
      </h2>
      <PatientManager userId={userId} />
    </div>
  );
}
