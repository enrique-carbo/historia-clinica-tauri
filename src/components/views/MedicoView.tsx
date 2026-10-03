// src/components/views/MedicoView.tsx
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { Navbar, Tab } from "../../components/ui/Navbar";
import { Button } from "../../components/ui/Button";
import { PatientEhrView } from "../../components/PatientEhrView";
import { Entity } from "../Entity";
import { ProfileView } from "../../components/ProfileView";
import { EntryTimeline } from "../../components/EntryTimeline";
import { EntryForm } from "../../components/EntryForm";
import { useNavigationStore, MedicoTab } from "../../stores/useNavigationStore";
import { usePatientStore } from "../../stores/usePatientStore";
import { useAuthStore } from "../../stores/useAuthStore";

const MEDICO_TABS: Tab[] = [
  { id: "perfil", label: "Perfil", icon: "👨🏻‍⚕️" },
  { id: "paciente", label: "Paciente", icon: "👤" },
  { id: "ehr", label: "Historia Clínica", icon: "📋" },
  { id: "entries", label: "Entries", icon: "⏱️" },
];

interface ExportResult {
  path: string;
  filename: string;
  entries_count: number;
}

export function MedicoView() {
  const { activeMedicoTab, setActiveMedicoTab } = useNavigationStore();
  const selectedPatient = usePatientStore((s) => s.selected?.patient ?? null);
  const activeUser = useAuthStore((s) => s.activeUser);
  const [showEntryForm, setShowEntryForm] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [exportResult, setExportResult] = useState<ExportResult | null>(null);
  const [exportError, setExportError] = useState<string | null>(null);

  useEffect(() => {
    setExportResult(null);
    setExportError(null);
  }, [selectedPatient?.id]);

  const handleExport = async () => {
    if (!selectedPatient || !activeUser || exporting) return;
    setExporting(true);
    setExportResult(null);
    setExportError(null);
    try {
      const snapshot = await invoke("get_export_snapshot", {
        subjectId: selectedPatient.id,
        professionalUserId: activeUser.user_id,
      });
      const result = await invoke<ExportResult>("export_history", { snapshot });
      setExportResult(result);
    } catch (e) {
      setExportError(typeof e === "string" ? e : "Error al exportar la historia clínica.");
    } finally {
      setExporting(false);
    }
  };

  const handleReveal = async () => {
    if (!exportResult) return;
    try {
      await revealItemInDir(exportResult.path);
    } catch (e) {
      setExportError(typeof e === "string" ? e : "No se pudo revelar el archivo.");
    }
  };

  return (
    <Navbar tabs={MEDICO_TABS} activeTab={activeMedicoTab} onTabChange={(tab) => setActiveMedicoTab(tab as MedicoTab)}>
      {activeMedicoTab === "ehr" && <PatientEhrView />}

      {activeMedicoTab === "paciente" && (
        <div className="flex flex-col gap-8 mx-auto w-full p-6">
          <div>
            <h2 className="text-lg font-semibold mb-4 text-zinc-300 flex items-center gap-2">
              <span className="p-1.5 rounded bg-blue-900/20 text-blue-400">👤</span>
              Admitir / Editar Paciente
            </h2>
            <Entity/>
          </div>
        </div>
      )}

      {activeMedicoTab === "perfil" && (
        <div className="flex flex-col gap-8 max-w-3xl mx-auto w-full p-6">
          <div>
            <h2 className="text-lg font-semibold mb-4 text-zinc-300">Perfil Profesional</h2>
            <ProfileView />
          </div>
        </div>
      )}

      {activeMedicoTab === "entries" && (
        <div className="flex flex-col gap-6 mx-auto w-full p-6">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <h2 className="text-lg font-semibold text-zinc-300 flex items-center gap-2">
              <span className="p-1.5 rounded bg-purple-900/20 text-purple-400">⏱️</span>
              Entries <span className="hidden md:inline">(Append-Only)</span>
              {selectedPatient && (
                <span className="text-zinc-500 font-normal text-sm">
                  · {selectedPatient.data.nombre} {selectedPatient.data.apellido}
                </span>
              )}
            </h2>
            {selectedPatient && (
              <div className="flex items-center gap-2">
                <Button
                  variant="success"
                  onClick={handleExport}
                  disabled={exporting}
                >
                  {exporting ? "Exportando…" : "Exportar .md"}
                </Button>
                <Button onClick={() => setShowEntryForm(!showEntryForm)}>
                  {showEntryForm ? "Ver Timeline" : "+ Nueva Entry"}
                </Button>
              </div>
            )}
          </div>

          {exportResult && (
            <div className="p-3 rounded-lg bg-emerald-950/40 border border-emerald-800 text-emerald-300 text-sm flex flex-wrap items-center justify-between gap-3">
              <span className="break-all">
                Exportado: {exportResult.filename} ({exportResult.entries_count} entries)
              </span>
              <div className="flex items-center gap-2 shrink-0">
                <Button variant="success" onClick={handleReveal}>
                  Revelar en carpeta
                </Button>
                <button
                  onClick={() => setExportResult(null)}
                  aria-label="Cerrar"
                  className="px-2 py-1.5 text-emerald-400 hover:text-emerald-200 transition-colors"
                >
                  ✕
                </button>
              </div>
            </div>
          )}

          {exportError && (
            <div className="p-3 rounded-lg bg-red-950/40 border border-red-800 text-red-400 text-sm flex items-center justify-between gap-3">
              <span className="break-all">{exportError}</span>
              <button
                onClick={() => setExportError(null)}
                aria-label="Cerrar"
                className="shrink-0 px-2 text-red-400 hover:text-red-200 transition-colors"
              >
                ✕
              </button>
            </div>
          )}

          {showEntryForm ? (
            <EntryForm onSuccess={() => setShowEntryForm(false)} />
          ) : (
            <>
              {selectedPatient ? (
                <EntryTimeline subjectId={selectedPatient.id} />
              ) : (
                <div className="p-4 rounded-lg bg-zinc-900 border border-zinc-800 text-zinc-500 text-sm">
                  Seleccioná un paciente primero para ver sus entries.
                </div>
              )}
            </>
          )}
        </div>
      )}
    </Navbar>
  );
}
