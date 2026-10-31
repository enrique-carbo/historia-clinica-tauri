// src/utils/backupReminder.ts
//
// Regla del recordatorio de respaldos. Puro (sin React ni Tauri) para poder
// testear los bordes: "nunca hubo", fecha inválida, exactamente el umbral.
//
// El umbral es una decisión de producto, no de seguridad: 7 días cubre una
// semana de trabajo de un consultorio sin volver el aviso molesto. Si algún
// día es configurable, este es el único lugar a tocar.

export const BACKUP_REMINDER_DAYS = 7;

const MS_PER_DAY = 86_400_000;

export type BackupReminderLevel = "ok" | "due" | "never";

export interface BackupReminder {
  level: BackupReminderLevel;
  /** Días completos desde el último respaldo (`null` si nunca hubo). */
  days: number | null;
}

export function backupReminder(
  lastBackupAt: string | null | undefined,
  now: Date = new Date(),
  thresholdDays: number = BACKUP_REMINDER_DAYS,
): BackupReminder {
  if (!lastBackupAt) return { level: "never", days: null };

  const last = new Date(lastBackupAt);
  if (Number.isNaN(last.getTime())) return { level: "never", days: null };

  // Fecha futura (reloj movido): se trata como "hoy", no como error.
  const days = Math.max(
    0,
    Math.floor((now.getTime() - last.getTime()) / MS_PER_DAY),
  );

  return { level: days >= thresholdDays ? "due" : "ok", days };
}

function daysLabel(days: number): string {
  return days === 1 ? "1 día" : `${days} días`;
}

/** Texto del recordatorio (banner) o del estado (panel de respaldos). */
export function backupReminderMessage(reminder: BackupReminder): string {
  switch (reminder.level) {
    case "never":
      return "Nunca se generó un respaldo. Si se pierde este equipo, se pierden las historias clínicas.";
    case "due":
      return `Pasaron ${daysLabel(reminder.days ?? 0)} desde el último respaldo. Conviene generar uno ahora.`;
    default:
      return `Último respaldo hace ${daysLabel(reminder.days ?? 0)}.`;
  }
}
