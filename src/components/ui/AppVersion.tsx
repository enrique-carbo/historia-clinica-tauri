// src/components/ui/AppVersion.tsx
//
// Versión visible de la app + modal "Acerca de".
//
// La versión no se hardcodea: `getVersion()` la lee de `tauri.conf.json` en
// build, así que la única fuente sigue siendo `pnpm version:sync`
// (docs/release.md). El nombre del producto sale de `APP_NAME` (perfil
// Care/Vet, `src/utils/branding.ts`).
//
// Se usa en el pie del drawer (con sesión) y en el login (sin sesión): el
// label resuelve el caso de soporte ("¿qué versión tenés?") y el modal da
// el detalle (licencia, descargas).

import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { APP_NAME } from "../../utils/branding";
import { Button } from "./Button";

const REPO_URL = "https://github.com/enrique-carbo/historia-clinica-tauri";
const RELEASES_URL = `${REPO_URL}/releases`;
const LICENSE = "AGPL-3.0-or-later";

export function AppVersion({ className = "" }: { className?: string }) {
  const [version, setVersion] = useState<string | null>(null);
  const [isOpen, setIsOpen] = useState(false);

  useEffect(() => {
    let cancelled = false;
    getVersion()
      .then((v) => {
        if (!cancelled) setVersion(v);
      })
      .catch(() => {
        // Sin runtime de Tauri (tests/entorno raro): el label queda con el
        // nombre del producto y el modal avisa que la versión no está
        // disponible. No es un error para el usuario.
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!isOpen) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        // En captura: evita que el Escape cierre también el drawer que
        // contiene este label (su handler escucha en fase burbuja).
        e.stopPropagation();
        setIsOpen(false);
      }
    };
    window.addEventListener("keydown", handleKeyDown, true);
    return () => window.removeEventListener("keydown", handleKeyDown, true);
  }, [isOpen]);

  const label = version ? `${APP_NAME} v${version}` : APP_NAME;

  return (
    <>
      <button
        type="button"
        onClick={() => setIsOpen(true)}
        className={`text-xs text-text-muted hover:text-text-secondary transition-colors cursor-pointer ${className}`}
        aria-label={`Acerca de ${APP_NAME}`}
      >
        {label}
      </button>

      {isOpen && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm p-4"
          onClick={() => setIsOpen(false)}
        >
          <div
            role="dialog"
            aria-modal="true"
            aria-labelledby="about-title"
            className="bg-surface border border-border-strong rounded-xl p-6 w-full max-w-sm shadow-2xl"
            onClick={(e) => e.stopPropagation()}
          >
            <h2
              id="about-title"
              className="text-base font-bold text-text-primary mb-1"
            >
              Acerca de {APP_NAME}
            </h2>
            <p className="text-xs text-text-muted mb-4">
              Historia clínica con datos cifrados en reposo.
            </p>

            <dl className="text-sm flex flex-col gap-2 mb-6">
              <div className="flex justify-between gap-4">
                <dt className="text-text-muted">Versión</dt>
                <dd className="text-text-primary font-mono">
                  {version ?? "no disponible"}
                </dd>
              </div>
              <div className="flex justify-between gap-4">
                <dt className="text-text-muted">Licencia</dt>
                <dd className="text-text-primary">{LICENSE}</dd>
              </div>
            </dl>

            <div className="flex flex-col gap-2">
              <Button
                type="button"
                variant="secondary"
                onClick={() => {
                  void openUrl(RELEASES_URL);
                }}
              >
                Ver versiones y descargas
              </Button>
              <Button
                type="button"
                variant="ghost"
                onClick={() => setIsOpen(false)}
              >
                Cerrar
              </Button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
