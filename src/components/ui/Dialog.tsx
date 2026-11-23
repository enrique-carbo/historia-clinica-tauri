// DIALOG base compartido: modal accesible con foco contenido y cierre
// consistente (Escape, click en el backdrop y botón Cerrar).
//
// No es una librería: cubre lo que necesitan los dos diálogos del app
// (templates/preview de solicitudes y "Acerca de") sin arrastrar radix ni
// reimplementar el foco en cada uno.

import {
  useEffect,
  useId,
  useRef,
  type KeyboardEvent as ReactKeyboardEvent,
  type ReactNode,
} from "react";
import { Button } from "./Button";

interface DialogProps {
  title: string;
  onClose: () => void;
  children: ReactNode;
  size?: "sm" | "lg";
}

const FOCUSABLE =
  'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), iframe, [tabindex]:not([tabindex="-1"])';

export function Dialog({ title, onClose, children, size = "lg" }: DialogProps) {
  const titleId = useId();
  const panelRef = useRef<HTMLDivElement>(null);
  const previousFocus = useRef<HTMLElement | null>(null);

  useEffect(() => {
    previousFocus.current = document.activeElement as HTMLElement | null;
    // El contenedor recibe el foco al abrir: así el lector de pantalla
    // anuncia el diálogo y Tab arranca dentro de él.
    panelRef.current?.focus();
    return () => {
      previousFocus.current?.focus();
    };
  }, []);

  // Escape en captura: cubre también el foco dentro del iframe de vista
  // previa y corta antes de que el drawer que contiene al diálogo cierre
  // también (su handler escucha en burbuja en window).
  useEffect(() => {
    const onEscape = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.stopPropagation();
      onClose();
    };
    window.addEventListener("keydown", onEscape, true);
    return () => window.removeEventListener("keydown", onEscape, true);
  }, [onClose]);

  const handleKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    if (event.key !== "Tab") return;

    const panel = panelRef.current;
    if (!panel) return;
    const focusables = Array.from(
      panel.querySelectorAll<HTMLElement>(FOCUSABLE),
    );
    if (focusables.length === 0) {
      event.preventDefault();
      panel.focus();
      return;
    }

    const first = focusables[0];
    const last = focusables[focusables.length - 1];
    if (event.shiftKey && (document.activeElement === first || document.activeElement === panel)) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
        onKeyDown={handleKeyDown}
        className={`w-full outline-none ${size === "sm" ? "max-w-sm" : "max-w-3xl"} max-h-[90vh] overflow-y-auto rounded-xl border border-border bg-surface shadow-xl`}
      >
        <div className="flex items-center justify-between gap-3 border-b border-border p-4">
          <h2 id={titleId} className="text-sm font-bold text-text-primary">
            {title}
          </h2>
          <Button size="sm" variant="ghost" onClick={onClose}>
            Cerrar
          </Button>
        </div>
        <div className="p-4">{children}</div>
      </div>
    </div>
  );
}
