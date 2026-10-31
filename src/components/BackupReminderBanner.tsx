// src/components/BackupReminderBanner.tsx
//
// Aviso persistente para administradores cuando el último respaldo es viejo
// (o no existe). El banner vive en el layout, así que se ve en cualquier
// pestaña — el momento en que el admin "se acuerda" de respaldar no es
// necesariamente cuando está en Respaldos.

import { useEffect } from "react";
import { useAuthStore } from "../stores/useAuthStore";
import { useBackupStore } from "../stores/useBackupStore";
import { useNavigationStore, AdminTab } from "../stores/useNavigationStore";
import { backupReminder, backupReminderMessage } from "../utils/backupReminder";
import { Alert } from "./ui/Alert";
import { Button } from "./ui/Button";

export function BackupReminderBanner() {
  const { activeUser } = useAuthStore();
  const { status, load } = useBackupStore();
  const setActiveAdminTab = useNavigationStore((s) => s.setActiveAdminTab);

  const isAdmin = activeUser?.role === "administrador";

  useEffect(() => {
    if (isAdmin) void load();
  }, [isAdmin, load]);

  if (!isAdmin || !status) return null;

  const reminder = backupReminder(status.last_backup_at);
  if (reminder.level === "ok") return null;

  return (
    <Alert variant="warning" className="mx-4 mt-3 sm:mx-10">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <span>
          <span aria-hidden="true">💾</span> {backupReminderMessage(reminder)}
        </span>
        <Button
          size="sm"
          variant="secondary"
          onClick={() => setActiveAdminTab("respaldos" as AdminTab)}
        >
          Ir a Respaldos
        </Button>
      </div>
    </Alert>
  );
}
