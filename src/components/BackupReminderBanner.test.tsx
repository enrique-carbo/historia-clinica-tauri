// src/components/BackupReminderBanner.test.tsx
//
// El banner solo aparece para administradores y solo cuando el respaldo
// está vencido (o nunca existió). `backup_status` se mockea: el contrato
// del backend ya tiene sus tests en Rust.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { BackupReminderBanner } from "./BackupReminderBanner";
import { useAuthStore } from "../stores/useAuthStore";
import { useBackupStore } from "../stores/useBackupStore";
import { useNavigationStore } from "../stores/useNavigationStore";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const invokeMock = vi.mocked(invoke);

function daysAgo(days: number): string {
  return new Date(Date.now() - days * 86_400_000).toISOString();
}

function loginAs(role: string) {
  useAuthStore.setState({
    isVaultUnlocked: true,
    activeUser: { user_id: "u-1", username: "admin", role },
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  useBackupStore.setState({ status: null, isLoading: false });
  useNavigationStore.setState({ activeAdminTab: "usuarios" });
});

describe("BackupReminderBanner", () => {
  it("sin respaldos avisa y lleva a la pestaña Respaldos", async () => {
    loginAs("administrador");
    invokeMock.mockResolvedValue({ last_backup_at: null, backup_count: 0 });
    const user = userEvent.setup();

    render(<BackupReminderBanner />);

    expect(
      await screen.findByText(/Nunca se generó un respaldo/),
    ).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Ir a Respaldos" }));
    expect(useNavigationStore.getState().activeAdminTab).toBe("respaldos");
  });

  it("respaldo viejo → avisa cuántos días pasaron", async () => {
    loginAs("administrador");
    invokeMock.mockResolvedValue({
      last_backup_at: daysAgo(9),
      backup_count: 2,
    });

    render(<BackupReminderBanner />);

    expect(await screen.findByText(/Pasaron 9 días/)).toBeInTheDocument();
  });

  it("respaldo reciente → no muestra nada", async () => {
    loginAs("administrador");
    invokeMock.mockResolvedValue({
      last_backup_at: daysAgo(1),
      backup_count: 3,
    });

    render(<BackupReminderBanner />);

    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("backup_status"));
    expect(screen.queryByText(/respaldo/i)).not.toBeInTheDocument();
  });

  it("no consulta ni muestra nada si el usuario no es administrador", async () => {
    loginAs("profesional");

    render(<BackupReminderBanner />);

    await waitFor(() => expect(invokeMock).not.toHaveBeenCalled());
    expect(screen.queryByText(/respaldo/i)).not.toBeInTheDocument();
  });

  it("si el backend falla, no rompe la UI", async () => {
    loginAs("administrador");
    invokeMock.mockRejectedValue({ code: "ERR_ADMIN_REQUIRED" });

    render(<BackupReminderBanner />);

    await waitFor(() => expect(invokeMock).toHaveBeenCalled());
    expect(screen.queryByText(/respaldo/i)).not.toBeInTheDocument();
  });
});
