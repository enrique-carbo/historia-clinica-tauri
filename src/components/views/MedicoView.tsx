import { useState } from "react";
import { Navbar, Tab } from "../../components/ui/Navbar";
import { PatientManager } from "../../components/PatientManager";
import { PatientSelector } from "../../components/PatientSelector";
import { SoapForm } from "../../components/SoapForm";
import { SoapHistory } from "../../components/SoapHistory";
import { MetricQuickForm } from "../../components/MetricQuickForm";
import { MetricViewer } from "../../components/MetricViewer";
import { EntityTest } from "../../components/EntityTest";
import { NoteTest } from "../../components/NoteTest";
import { usePatientStore, SoapRecord } from "../../stores/usePatientStore";

const MEDICO_TABS: Tab[] = [
  { id: "paciente", label: "Paciente", icon: "👤" },
  { id: "consulta", label: "Consulta", icon: "🩺" },
  { id: "test", label: "Test Entities", icon: "🧪" },
  { id: "test-note", label: "Test Notas", icon: "📝" },
];

interface MedicoViewProps {
  userId: string;
  history: SoapRecord[];
  fetchHistory: () => void;
  fetchMetrics: () => void;
}

export function MedicoView({
  userId,
  history,
  fetchHistory,
  fetchMetrics,
}: MedicoViewProps) {
  const [activeTab, setActiveTab] = useState("consulta");
  const { activePatient, selectPatient } = usePatientStore();

  return (
    <Navbar tabs={MEDICO_TABS} activeTab={activeTab} onTabChange={setActiveTab}>
      {activeTab === "paciente" && (
        <div className="flex flex-col gap-8">
          <div>
            <h2 className="text-lg font-semibold mb-4 text-zinc-300">
              Admitir Nuevo Paciente
            </h2>
            <PatientManager userId={userId} />
          </div>

          <hr className="border-zinc-800" />
        </div>
      )}

      {activeTab === "consulta" && (
        <div className="flex flex-col gap-8">
          <PatientSelector
            onSelectPatient={selectPatient}
            onPatientSelected={() => {
              fetchHistory();
              fetchMetrics();
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

                  <SoapForm
                    pacienteId={activePatient.id}
                    medicoId={userId}
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

      {activeTab === "test" && <EntityTest />}
      {activeTab === "test-note" && <NoteTest />}
    </Navbar>
  );
}
