// src/components/views/AdminUsersPanel.test.tsx

import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { AdminUsersPanel } from "./AdminUsersPanel";
import { useAuthStore } from "../../stores/useAuthStore";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const invokeMock = vi.mocked(invoke);

const USERS = [
  {
    id: "u-admin",
    username: "admin",
    role: "administrador",
    is_active: true,
    created_at: "2026-01-01T00:00:00Z",
    last_login_at: null,
  },
  {
    id: "u-dra",
    username: "dra.perez",
    role: "profesional",
    is_active: true,
    created_at: "2026-01-02T00:00:00Z",
    last_login_at: "2026-01-03T00:00:00Z",
  },
];

beforeEach(() => {
  vi.clearAllMocks();
  useAuthStore.setState({
    isVaultUnlocked: true,
    activeUser: { user_id: "u-admin", username: "admin", role: "administrador" },
  });
});

describe("AdminUsersPanel", () => {
  it("lista los usuarios que devuelve el backend", async () => {
    invokeMock.mockResolvedValueOnce(USERS);

    render(<AdminUsersPanel />);

    expect(await screen.findByText("admin")).toBeInTheDocument();
    expect(screen.getByText("dra.perez")).toBeInTheDocument();
    expect(screen.getAllByText("Administrador").length).toBeGreaterThan(0);
  });

  it("muestra el error del backend en un Alert descartable", async () => {
    invokeMock.mockRejectedValueOnce("sin permisos");

    render(<AdminUsersPanel />);

    expect(await screen.findByText("sin permisos")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Cerrar" }));
    expect(screen.queryByText("sin permisos")).not.toBeInTheDocument();
  });

  it("desactivar invoca set_user_active y muestra el aviso", async () => {
    invokeMock.mockResolvedValueOnce(USERS);
    const user = userEvent.setup();
    render(<AdminUsersPanel />);

    await screen.findByText("dra.perez");
    // Solo la fila de dra.perez es desactivable (u-admin es la cuenta propia).
    const enabled = screen
      .getAllByRole("button", { name: "Desactivar" })
      .find((b) => !b.hasAttribute("disabled"))!;

    invokeMock.mockResolvedValueOnce(undefined);
    invokeMock.mockResolvedValueOnce(USERS.map((u) =>
      u.id === "u-dra" ? { ...u, is_active: false } : u,
    ));
    await user.click(enabled);

    await waitFor(() =>
      expect(
        screen.getByText(/Usuario «dra.perez» desactivado/),
      ).toBeInTheDocument(),
    );
    expect(invokeMock).toHaveBeenCalledWith("set_user_active", {
      userId: "u-dra",
      active: false,
    });
  });

  it("no permite desactivar la propia cuenta", async () => {
    invokeMock.mockResolvedValueOnce(USERS);

    render(<AdminUsersPanel />);

    const rows = await screen.findAllByText(/admin/);
    expect(rows.length).toBeGreaterThan(0);
    const selfButtons = screen
      .getAllByRole("button", { name: "Desactivar" })
      .filter((b) => b.hasAttribute("disabled"));
    expect(selfButtons).toHaveLength(1);
  });
});
