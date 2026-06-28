import { useEffect } from "react";
import { AuthBox } from "./components/AuthBox";
import { DashboardLayout } from "./components/layouts/DashboardLayout";
import { AdminView } from "./components/views/AdminView";
import { MedicoView } from "./components/views/MedicoView";
import { PacienteView } from "./components/views/PacienteView";

import { useAuthStore } from "./stores/useAuthStore";
import { usePatientStore } from "./stores/usePatientStore";

export default function App() {
  const { activeUser: user } = useAuthStore();
  const { activePatient, history, fetchHistory, fetchMetrics } =
    usePatientStore();

  useEffect(() => {
    if (activePatient && user?.role === "medico") {
      fetchHistory();
    }
  }, [activePatient, user?.role, fetchHistory]);

  if (!user) {
    return <AuthBox />;
  }

  return (
    <DashboardLayout>
      {user.role === "admin" && <AdminView userId={user.user_id} />}

      {user.role === "medico" && (
        <MedicoView
          userId={user.user_id}
          history={history}
          fetchHistory={fetchHistory}
          fetchMetrics={fetchMetrics}
        />
      )}

      {user.role === "paciente" && <PacienteView />}
    </DashboardLayout>
  );
}
