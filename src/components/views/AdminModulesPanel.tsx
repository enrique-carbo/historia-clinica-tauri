import { useCallback, useEffect, useState } from "react";
import { formatError } from "../../utils/errors";
import {
  activateModule,
  deactivateModule,
  listModules,
  type ModuleStatusDto,
} from "../../utils/modules";
import { Alert } from "../ui/Alert";
import { Button } from "../ui/Button";
import { EmptyState } from "../ui/EmptyState";
import { notifyModulesChanged } from "../../hooks/useModuleAvailable";

export function AdminModulesPanel() {
  const [modules, setModules] = useState<ModuleStatusDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [togglingId, setTogglingId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const list = await listModules();
      setModules(list);
    } catch (err) {
      setError(formatError(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const toggle = async (module: ModuleStatusDto) => {
    setTogglingId(module.id);
    setError(null);
    try {
      if (module.state === "active") {
        await deactivateModule(module.id);
      } else {
        await activateModule(module.id);
      }
      await reload();
      // Avisar a las vistas del profesional para que revaliden el tab del
      // módulo y el ítem del drawer sin necesidad de reiniciar la app.
      notifyModulesChanged();
    } catch (err) {
      setError(formatError(err));
    } finally {
      setTogglingId(null);
    }
  };

  return (
    <div className="max-w-5xl mx-auto flex flex-col gap-4">
      <div>
        <h2 className="text-lg font-semibold text-text-primary">Módulos</h2>
        <p className="text-xs text-text-muted mt-1">
          Módulos incluidos en este binario. Desactivar un módulo lo oculta sin
          borrar su base de datos.
        </p>
      </div>

      {error && (
        <Alert variant="error">
          <div className="flex justify-between gap-4">
            <span>{error}</span>
            <Button
              size="icon"
              variant="ghost"
              onClick={() => setError(null)}
              aria-label="Cerrar"
            >
              ✕
            </Button>
          </div>
        </Alert>
      )}

      <div className="bg-surface border border-border rounded-xl overflow-hidden">
        {loading ? (
          <div className="p-6 flex items-center gap-2 text-sm text-text-muted">
            <span className="animate-spin inline-block h-4 w-4 rounded-full border-b-2 border-focus" />
            Cargando módulos…
          </div>
        ) : modules.length === 0 ? (
          <div className="p-6">
            <EmptyState
              title="Sin módulos compilados en este binario"
              hint="Esta edición contiene únicamente el Core"
            />
          </div>
        ) : (
          <ul className="divide-y divide-border/60">
            {modules.map((module) => {
              const isActive = module.state === "active";
              return (
                <li
                  key={module.id}
                  className="p-4 flex flex-wrap items-center justify-between gap-3"
                >
                  <div className="min-w-0">
                    <div className="flex items-center gap-2 flex-wrap">
                      <span className="font-semibold text-sm text-text-primary">
                        {module.name}
                      </span>
                      <span className="text-[11px] font-mono bg-surface-raised text-text-secondary px-1.5 py-0.5 rounded">
                        v{module.version}
                      </span>
                      <span
                        className={`text-[11px] font-semibold px-2 py-0.5 rounded ${
                          isActive
                            ? "bg-success-tint text-success-text border border-success-border"
                            : "bg-surface-raised text-text-muted border border-border"
                        }`}
                      >
                        {isActive ? "Activo" : "Inactivo"}
                      </span>
                    </div>
                    <div className="text-xs text-text-muted mt-1">
                      ID: <span className="font-mono">{module.id}</span> · Roles:{" "}
                      {module.allowed_roles.join(", ")}
                    </div>
                  </div>
                  <Button
                    size="sm"
                    variant={isActive ? "secondary" : "primary"}
                    isLoading={togglingId === module.id}
                    onClick={() => void toggle(module)}
                  >
                    {isActive ? "Desactivar" : "Activar"}
                  </Button>
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </div>
  );
}
