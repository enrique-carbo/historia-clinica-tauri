// Botón de cierre compartido para alerts y paneles: misma forma, mismo
// aria-label y el mismo icono X que ya usan el Drawer y el Dialog.
import type { ComponentProps } from "react";
import { X } from "lucide-react";
import { Button } from "./Button";

type CloseButtonProps = Omit<
  ComponentProps<typeof Button>,
  "children" | "size" | "variant" | "aria-label"
>;

export function CloseButton(props: CloseButtonProps) {
  return (
    <Button size="icon" variant="ghost" aria-label="Cerrar" {...props}>
      <X size={18} aria-hidden="true" />
    </Button>
  );
}
