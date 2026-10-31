// src/utils/backupReminder.test.ts
//
// Regla pura del recordatorio: fechas fijas, sin depender del reloj real.

import { describe, expect, it } from "vitest";
import {
  BACKUP_REMINDER_DAYS,
  backupReminder,
  backupReminderMessage,
} from "./backupReminder";

const NOW = new Date("2026-10-31T12:00:00Z");

function daysAgo(days: number): string {
  return new Date(NOW.getTime() - days * 86_400_000).toISOString();
}

describe("backupReminder", () => {
  it("sin fecha o con fecha inválida → never", () => {
    expect(backupReminder(null, NOW).level).toBe("never");
    expect(backupReminder(undefined, NOW).level).toBe("never");
    expect(backupReminder("no-es-fecha", NOW).level).toBe("never");
  });

  it("respaldos recientes → ok con los días transcurridos", () => {
    expect(backupReminder(daysAgo(0), NOW)).toEqual({ level: "ok", days: 0 });
    expect(backupReminder(daysAgo(6), NOW)).toEqual({ level: "ok", days: 6 });
  });

  it("el umbral exacto ya vence", () => {
    expect(backupReminder(daysAgo(BACKUP_REMINDER_DAYS), NOW).level).toBe("due");
    expect(backupReminder(daysAgo(BACKUP_REMINDER_DAYS - 1), NOW).level).toBe("ok");
  });

  it("una fecha futura no da días negativos", () => {
    const tomorrow = new Date(NOW.getTime() + 86_400_000).toISOString();
    expect(backupReminder(tomorrow, NOW)).toEqual({ level: "ok", days: 0 });
  });

  it("el umbral es configurable", () => {
    expect(backupReminder(daysAgo(2), NOW, 2).level).toBe("due");
    expect(backupReminder(daysAgo(1), NOW, 2).level).toBe("ok");
  });
});

describe("backupReminderMessage", () => {
  it("nunca respaldó → explica el riesgo", () => {
    expect(backupReminderMessage({ level: "never", days: null })).toContain(
      "Nunca se generó un respaldo",
    );
  });

  it("vencido → dice cuántos días pasaron", () => {
    expect(backupReminderMessage({ level: "due", days: 9 })).toContain(
      "Pasaron 9 días",
    );
  });

  it("singular/plural del día", () => {
    expect(backupReminderMessage({ level: "ok", days: 1 })).toContain("1 día");
    expect(backupReminderMessage({ level: "ok", days: 3 })).toContain("3 días");
  });
});
