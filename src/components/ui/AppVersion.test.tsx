// src/components/ui/AppVersion.test.tsx
//
// La versión sale de `getVersion()` (tauri.conf.json en build), no de una
// constante: el test mockea el módulo de Tauri. Si el runtime no responde,
// el label cae al nombre del producto sin romper la UI.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { AppVersion } from "./AppVersion";
import { APP_NAME } from "../../utils/branding";

vi.mock("@tauri-apps/api/app", () => ({ getVersion: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));

const getVersionMock = vi.mocked(getVersion);
const openUrlMock = vi.mocked(openUrl);

beforeEach(() => {
  vi.clearAllMocks();
  getVersionMock.mockResolvedValue("0.1.2");
  openUrlMock.mockResolvedValue(undefined);
});

describe("AppVersion", () => {
  it("muestra el nombre del producto y la versión del build", async () => {
    render(<AppVersion />);
    expect(
      await screen.findByRole("button", { name: `Acerca de ${APP_NAME}` }),
    ).toHaveTextContent(`${APP_NAME} v0.1.2`);
  });

  it("abre el modal con versión y licencia al hacer clic", async () => {
    const user = userEvent.setup();
    render(<AppVersion />);

    await user.click(
      await screen.findByRole("button", { name: `Acerca de ${APP_NAME}` }),
    );

    const dialog = screen.getByRole("dialog");
    expect(dialog).toHaveTextContent(`Acerca de ${APP_NAME}`);
    expect(dialog).toHaveTextContent("0.1.2");
    expect(dialog).toHaveTextContent("AGPL-3.0-or-later");
  });

  it("cierra el modal con Escape", async () => {
    const user = userEvent.setup();
    render(<AppVersion />);

    await user.click(
      await screen.findByRole("button", { name: `Acerca de ${APP_NAME}` }),
    );
    expect(screen.getByRole("dialog")).toBeInTheDocument();

    await user.keyboard("{Escape}");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("abre la página de releases con el opener (no navega el webview)", async () => {
    const user = userEvent.setup();
    render(<AppVersion />);

    await user.click(
      await screen.findByRole("button", { name: `Acerca de ${APP_NAME}` }),
    );
    await user.click(
      screen.getByRole("button", { name: "Ver versiones y descargas" }),
    );

    expect(openUrlMock).toHaveBeenCalledWith(
      "https://github.com/enrique-carbo/historia-clinica-tauri/releases",
    );
  });

  it("sin runtime de Tauri muestra solo el producto y 'no disponible'", async () => {
    getVersionMock.mockRejectedValue("sin runtime");
    const user = userEvent.setup();
    render(<AppVersion />);

    const label = await screen.findByRole("button", {
      name: `Acerca de ${APP_NAME}`,
    });
    await waitFor(() => expect(label).toHaveTextContent(APP_NAME));
    expect(label).not.toHaveTextContent("v0.1.2");

    await user.click(label);
    expect(screen.getByRole("dialog")).toHaveTextContent("no disponible");
  });
});
