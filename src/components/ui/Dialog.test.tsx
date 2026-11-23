// src/components/ui/Dialog.test.tsx
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Dialog } from "./Dialog";

function Harness({ onOuterKeyDown }: { onOuterKeyDown?: () => void }) {
  const [open, setOpen] = useState(false);
  return (
    <div onKeyDown={onOuterKeyDown}>
      <button onClick={() => setOpen(true)}>Abrir</button>
      {open && (
        <Dialog title="Título" onClose={() => setOpen(false)}>
          <button>Uno</button>
          <button>Dos</button>
        </Dialog>
      )}
    </div>
  );
}

describe("Dialog", () => {
  it("asocia el título y mueve el foco al abrir", async () => {
    const user = userEvent.setup();
    render(<Harness />);

    await user.click(screen.getByRole("button", { name: "Abrir" }));

    const dialog = screen.getByRole("dialog");
    expect(dialog).toHaveAttribute("aria-modal", "true");
    expect(dialog).toHaveAccessibleName("Título");
    expect(dialog).toHaveFocus();
  });

  it("devuelve el foco al elemento que lo abrió", async () => {
    const user = userEvent.setup();
    render(<Harness />);

    await user.click(screen.getByRole("button", { name: "Abrir" }));
    await user.click(screen.getByRole("button", { name: "Cerrar" }));

    expect(screen.getByRole("button", { name: "Abrir" })).toHaveFocus();
  });

  it("cierra con Escape sin propagar al contenedor", async () => {
    const onOuterKeyDown = vi.fn();
    const user = userEvent.setup();
    render(<Harness onOuterKeyDown={onOuterKeyDown} />);

    await user.click(screen.getByRole("button", { name: "Abrir" }));
    await user.keyboard("{Escape}");

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(onOuterKeyDown).not.toHaveBeenCalled();
  });

  it("mantiene Tab dentro del diálogo", async () => {
    const user = userEvent.setup();
    render(<Harness />);

    await user.click(screen.getByRole("button", { name: "Abrir" }));
    // Orden: Cerrar (header), Uno, Dos.
    await user.keyboard("{Tab}");
    expect(screen.getByRole("button", { name: "Cerrar" })).toHaveFocus();

    await user.keyboard("{Tab}{Tab}{Tab}");
    expect(screen.getByRole("button", { name: "Cerrar" })).toHaveFocus();

    await user.keyboard("{Shift>}{Tab}{/Shift}");
    expect(screen.getByRole("button", { name: "Dos" })).toHaveFocus();
  });
});
