// src/components/ui/Button.test.tsx
import type { FormEvent } from "react";
import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Button } from "./Button";

describe("Button", () => {
  it("no envía el form por defecto", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn((event: FormEvent) => event.preventDefault());

    render(
      <form onSubmit={onSubmit}>
        <Button onClick={() => undefined}>Volver</Button>
      </form>,
    );

    await user.click(screen.getByRole("button", { name: "Volver" }));
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it("sí envía el form con type=\"submit\" explícito", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn((event: FormEvent) => event.preventDefault());

    render(
      <form onSubmit={onSubmit}>
        <Button type="submit">Guardar</Button>
      </form>,
    );

    await user.click(screen.getByRole("button", { name: "Guardar" }));
    expect(onSubmit).toHaveBeenCalledTimes(1);
  });
});
