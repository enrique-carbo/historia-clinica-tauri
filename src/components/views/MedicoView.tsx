// src/components/views/MedicoView.tsx
import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { formatError } from "../../utils/errors";
import { isModuleAvailable } from "../../utils/modules";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { Navbar, Tab } from "../../components/ui/Navbar";
import { Button } from "../../components/ui/Button";
import { Alert } from "../../components/ui/Alert";
import { PatientEhrView } from "../../components/PatientEhrView";
import { SolicitudesPanel } from "../../components/SolicitudesPanel";
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

// El tab del módulo solo existe si `is_module_available` lo habilita
// (compilado con la feature + activo + rol con acceso). Se inserta antes de
// Entries para no separar la Historia Clínica de su tablero de entries.
const SOLICITUDES_TAB: Tab = { id: "solicitudes", label: "Solicitudes", icon: "🧪" };

interface ExportResult {
  path: string;
  filename: string;
  entries_count: number;
  sha256: string;
}

export function MedicoView() {
  const { activeMedicoTab, setActiveMedicoTab } = useNavigationStore();
  const selectedPatient = usePatientStore((s) => s.selected?.patient ?? null);
  const activeUser = useAuthStore((s) => s.activeUser);
  const [showEntryForm, setShowEntryForm] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [exportResult, setExportResult] = useState<ExportResult | null>(null);
  const [exportError, setExportError] = useState<string | null>(null);
  // `null` = todavía no se consultó: evita parpadear el tab y evita
  // redirigir antes de saber si el módulo está disponible.
  const [solicitudesAvailable, setSolicitudesAvailable] = useState<boolean | null>(null);

  useEffect(() => {
    let cancelled = false;
    isModuleAvailable("solicitudes")
      .then((enabled) => {
        if (!cancelled) setSolicitudesAvailable(enabled);
      })
      .catch(() => {
        if (!cancelled) setSolicitudesAvailable(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // Si el tab quedó seleccionado de una sesión anterior pero el módulo ya no
  // está disponible, volver a la Historia Clínica en vez de mostrar vacío.
  useEffect(() => {
    if (solicitudesAvailable === false && activeMedicoTab === "solicitudes") {
      setActiveMedicoTab("ehr");
    }
  }, [solicitudesAvailable, activeMedicoTab, setActiveMedicoTab]);

  const tabs = useMemo(() => {
    if (!solicitudesAvailable) return MEDICO_TABS;
    const entriesIndex = MEDICO_TABS.findIndex((tab) => tab.id === "entries");
    return [
      ...MEDICO_TABS.slice(0, entriesIndex),
      SOLICITUDES_TAB,
      ...MEDICO_TABS.slice(entriesIndex),
    ];
  }, [solicitudesAvailable]);

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
      // Un solo roundtrip: el backend descifra, renderiza y escribe —
      // el snapshot (datos sensibles) nunca cruza el IPC.
      const result = await invoke<ExportResult>("export_history", {
        subjectId: selectedPatient.id,
      });
      setExportResult(result);
    } catch (e) {
      setExportError(formatError(e));
    } finally {
      setExporting(false);
    }
  };

  const handleReveal = async () => {
    if (!exportResult) return;
    try {
      await revealItemInDir(exportResult.path);
    } catch (e) {
      setExportError(formatError(e));
    }
  };

  return (
    <Navbar tabs={tabs} activeTab={activeMedicoTab} onTabChange={(tab) => setActiveMedicoTab(tab as MedicoTab)}>
      {activeMedicoTab === "ehr" && <PatientEhrView />}

      {solicitudesAvailable && activeMedicoTab === "solicitudes" && (
        <div className="flex flex-col gap-6 mx-auto w-full p-2">
          {selectedPatient ? (
            <SolicitudesPanel entityId={selectedPatient.id} />
          ) : (
            <div className="p-4 rounded-lg bg-surface border border-border text-text-muted text-sm">
              Seleccioná un paciente primero para ver sus solicitudes.
            </div>
          )}
        </div>
      )}

      {activeMedicoTab === "paciente" && (
        <div className="flex flex-col mx-auto w-full p-2">
          <div>
            <Entity/>
          </div>
        </div>
      )}

      {activeMedicoTab === "perfil" && (
        <div className="flex flex-col max-w-3xl mx-auto w-full p-2">
          <div>
            <ProfileView />
          </div>
        </div>
      )}

      {activeMedicoTab === "entries" && (
        <div className="flex flex-col gap-6 mx-auto w-full p-6">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <h2 className="text-lg font-semibold text-text-primary flex items-center gap-2">
              Entries <span className="hidden md:inline">(Append-Only)</span>
              {selectedPatient && (
                <span className="font-normal text-md">
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
                  {showEntryForm ? "Ver Timeline" : "Nueva Entry"}
                </Button>
              </div>
            )}
          </div>

          {exportResult && (
            <Alert variant="success" className="text-sm">
              <div className="flex flex-wrap items-center justify-between gap-3">
                <span className="break-all">
                  Exportado: {exportResult.filename} ({exportResult.entries_count} entries)
                  <span className="text-success-text"> ✓ firmado</span>
                  {" · sha256 "}
                  <code title={exportResult.sha256}>
                    {exportResult.sha256.slice(0, 12)}…
                  </code>
                </span>
                <div className="flex items-center gap-2 shrink-0">
                  <Button variant="success" onClick={handleReveal}>
                    Revelar en carpeta
                  </Button>
                  <Button
                    size="icon"
                    variant="ghost"
                    onClick={() => setExportResult(null)}
                    aria-label="Cerrar"
                  >
                    ✕
                  </Button>
                </div>
              </div>
            </Alert>
          )}

          {exportError && (
            <Alert variant="error" className="text-sm">
              <div className="flex items-center justify-between gap-3">
                <span className="break-all">{exportError}</span>
                <Button
                  size="icon"
                  variant="ghost"
                  onClick={() => setExportError(null)}
                  aria-label="Cerrar"
                >
                  ✕
                </Button>
              </div>
            </Alert>
          )}

          {showEntryForm ? (
            <EntryForm onSuccess={() => setShowEntryForm(false)} />
          ) : (
            <>
              {selectedPatient ? (
                <EntryTimeline subjectId={selectedPatient.id} />
              ) : (
                <div className="p-4 rounded-lg bg-surface border border-border text-text-muted text-sm">
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
