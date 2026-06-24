import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { SoapForm } from "./components/SoapForm";
import { SoapHistory } from "./components/SoapHistory";
import { AuthBox } from "./components/AuthBox";
import { PatientManager } from "./components/PatientManager";
import { PatientSelector } from "./components/PatientSelector";
import { MetricQuickForm } from "./components/MetricQuickForm";
import { MetricViewer } from "./components/MetricViewer";

// 🚀 Importamos los dos stores
import { useAuthStore } from "./stores/useAuthStore";
import { usePatientStore } from "./stores/usePatientStore";

export default function App() {
  // 🚀 Auth State
  const { activeUser: user, lockVault } = useAuthStore();

  // 🚀 Patient State
  const { activePatient, history, selectPatient, fetchHistory, fetchMetrics } =
    usePatientStore();

  useEffect(() => {
    if (activePatient && user?.role === "medico") {
      fetchHistory();
    }
  }, [activePatient, user?.role, fetchHistory]);

  // Cerrar sesión segura
  const handleLogout = async () => {
    try {
      await invoke("lock_vault");
      lockVault();
      usePatientStore.getState().clearPatient(); // Limpiamos la selección de paciente al salir
    } catch (err) {
      console.error("Error al cerrar la bóveda:", err);
    }
  };

  if (!user) {
    return <AuthBox />;
  }

  return (
    <div className="min-h-screen bg-zinc-950 text-zinc-200 p-10 font-sans">
      {/* HEADER (Sin cambios significativos) */}
      <header className="mb-8 pb-5 border-b border-zinc-800 flex justify-between items-center">
        <div>
          <h1 className="text-2xl font-bold text-zinc-100 m-0">
            🩺 Simplex Health Core
          </h1>
          <p className="text-sm text-zinc-500 mt-1">
            Ecosistema Clínico Descentralizado — Rol:{" "}
            <strong className="text-blue-500 uppercase">{user.role}</strong>
          </p>
        </div>
        <div className="text-right flex items-center gap-4">
          <span className="text-sm text-zinc-500">
            Usuario: <strong className="text-zinc-300">{user.username}</strong>
          </span>
          <button
            onClick={handleLogout}
            className="px-3 py-1.5 bg-zinc-800 border border-zinc-700 text-zinc-300 rounded text-xs hover:bg-red-900/50 hover:border-red-800 hover:text-red-300 transition-colors cursor-pointer"
          >
            🔒 Cerrar Bóveda
          </button>
        </div>
      </header>

      {/* VISTA ADMIN */}
      {user.role === "admin" && (
        <div>
          <h2 className="text-lg font-semibold mb-5 text-zinc-300">
            Consola de Recepción y Admisión
          </h2>
          <PatientManager userId={user.user_id} />
        </div>
      )}

      {/* VISTA MÉDICO */}
      {user.role === "medico" && (
        <div className="flex flex-col gap-8">
          <div>
            <h2 className="text-lg font-semibold mb-4 text-zinc-300">
              Admitir Nuevo Paciente
            </h2>
            <PatientManager userId={user.user_id} />
          </div>

          <hr className="border-zinc-800" />

          {/* 🚀 El PatientSelector ahora usa la función del store directamente */}
          <PatientSelector
            onSelectPatient={selectPatient}
            onPatientSelected={() => {
              fetchHistory(); // Carga el SOAP
              fetchMetrics(); // Carga las métricas previas
            }}
            selectedPatientId={activePatient ? activePatient.id : null}
          />

          {activePatient ? (
            <div>
              <h2 className="text-lg font-semibold mb-4 text-zinc-300">
                Consulta Activa:{" "}
                <span className="text-blue-400">{activePatient.full_name}</span>
              </h2>

              <div className="grid grid-cols-2 gap-10 max-w-6xl">
                <div className="flex flex-col gap-6">
                  <div className="text-xs text-zinc-600 font-mono">
                    VÍNCULO RELACIONAL:{" "}
                    <span className="text-zinc-400">{activePatient.id}</span>
                  </div>

                  {/* 🚀 Los formularios disparan fetchHistory del store al tener éxito */}
                  <SoapForm
                    pacienteId={activePatient.id}
                    medicoId={user.user_id}
                    onSuccess={fetchHistory}
                  />

                  <div className="p-5 bg-zinc-900 rounded-lg border border-zinc-800">
                    <h3 className="text-base font-semibold text-zinc-200 mb-4">
                      📊 Registro de Métrica Rápida
                    </h3>
                    <MetricQuickForm
                      pacienteId={activePatient.id}
                      onSuccess={() => {
                        fetchHistory();
                        fetchMetrics();
                      }}
                    />
                  </div>

                  <MetricViewer />
                </div>

                <div>
                  <h3 className="text-base font-semibold text-zinc-200 mb-4">
                    Historial Clínico Cifrado
                  </h3>
                  {/* 🚀 Le pasamos el historial que vive en el store */}
                  <SoapHistory records={history} />
                </div>
              </div>
            </div>
          ) : (
            <div className="p-10 text-center bg-zinc-900/50 rounded-lg border border-dashed border-zinc-800 text-zinc-600">
              💡 Seleccioná un paciente del buscador para abrir su ficha clínica
              y redactar una evolución SOAP.
            </div>
          )}
        </div>
      )}
    </div>
  );
}
