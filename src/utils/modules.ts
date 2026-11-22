// Cliente mínimo del contrato de módulos (`docs/module-contract.md` §5–§6).
//
// El frontend NUNCA usa commands tipados por módulo: todo pasa por
// `module_execute`. `module_id` viaja como `moduleId` (Tauri camelCase); los
// campos de `ModuleRequest` viajan en snake_case porque los deserializa
// serde en el struct Rust.

import { invoke } from "@tauri-apps/api/core";

export interface ModuleRequestPayload {
  protocol_version: number;
  action: string;
  entity_id: number | null;
  payload: unknown;
}

export interface ModuleStatusDto {
  id: string;
  name: string;
  version: string;
  scope: "Installation" | "InstallationAndProfessional";
  allowed_roles: string[];
  state: "active" | "inactive" | "uninstalled" | string;
}

export async function isModuleAvailable(moduleId: string): Promise<boolean> {
  return invoke<boolean>("is_module_available", { moduleId });
}

export async function listModules(): Promise<ModuleStatusDto[]> {
  return invoke<ModuleStatusDto[]>("list_modules");
}

export async function activateModule(moduleId: string): Promise<void> {
  await invoke("module_activate", { moduleId });
}

export async function deactivateModule(moduleId: string): Promise<void> {
  await invoke("module_deactivate", { moduleId });
}

export async function moduleExecute<T>(
  moduleId: string,
  action: string,
  payload: unknown = {},
  entityId?: number,
): Promise<T> {
  const req: ModuleRequestPayload = {
    protocol_version: 1,
    action,
    entity_id: entityId ?? null,
    payload,
  };
  const response = await invoke<{ payload: T }>("module_execute", {
    moduleId,
    req,
  });
  return response.payload;
}
