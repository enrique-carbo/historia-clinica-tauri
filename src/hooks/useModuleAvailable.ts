// Disponibilidad de módulos con revalidación en caliente.
//
// Tres consumidores (MedicoView, NavigationDrawer y SolicitudesPanel) hacían
// la misma llamada `is_module_available` al montar y se quedaban con el
// resultado para siempre. Si el admin activaba/desactivaba un módulo con la
// app abierta, la UI no se enteraba hasta reiniciar.
//
// Este hook centraliza la consulta y la repite en dos situaciones:
//   1. `fichio:modules-changed`, que emite el panel de admin tras activar o
//      desactivar un módulo.
//   2. `focus` de la ventana, para cubrir cambios hechos en otra ventana.

import { useCallback, useEffect, useRef, useState } from "react";
import { isModuleAvailable } from "../utils/modules";

export const MODULES_CHANGED_EVENT = "fichio:modules-changed";

export function notifyModulesChanged(): void {
  window.dispatchEvent(new Event(MODULES_CHANGED_EVENT));
}

/** `null` = todavía no se consultó. Evita parpadeos y redirecciones prematuras. */
export function useModuleAvailable(moduleId: string): boolean | null {
  const [available, setAvailable] = useState<boolean | null>(null);
  // Descarta respuestas de consultas anteriores que lleguen después de una
  // más nueva (por ejemplo, activate + focus casi simultáneos).
  const generation = useRef(0);

  const refresh = useCallback(() => {
    const current = ++generation.current;
    isModuleAvailable(moduleId)
      .then((enabled) => {
        if (current === generation.current) setAvailable(enabled);
      })
      .catch(() => {
        if (current === generation.current) setAvailable(false);
      });
  }, [moduleId]);

  useEffect(() => {
    refresh();
    window.addEventListener(MODULES_CHANGED_EVENT, refresh);
    window.addEventListener("focus", refresh);
    return () => {
      generation.current += 1;
      window.removeEventListener(MODULES_CHANGED_EVENT, refresh);
      window.removeEventListener("focus", refresh);
    };
  }, [refresh]);

  return available;
}
