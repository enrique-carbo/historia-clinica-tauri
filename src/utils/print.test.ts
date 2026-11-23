// src/utils/print.test.ts
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { downloadText } from "./print";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const invokeMock = vi.mocked(invoke);

beforeEach(() => {
  vi.clearAllMocks();
});

describe("downloadText", () => {
  it("guarda con el diálogo nativo y devuelve true", async () => {
    invokeMock.mockResolvedValue("/tmp/solicitud.md");

    await expect(downloadText("solicitud.md", "# Hola")).resolves.toBe(true);
    expect(invokeMock).toHaveBeenCalledWith("save_text_file", {
      defaultFilename: "solicitud.md",
      content: "# Hola",
    });
  });

  it("devuelve false si el usuario cancela el diálogo", async () => {
    invokeMock.mockResolvedValue(null);

    await expect(downloadText("solicitud.md", "# Hola")).resolves.toBe(false);
  });
});
