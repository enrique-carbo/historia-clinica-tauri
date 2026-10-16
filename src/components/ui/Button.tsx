import { ButtonHTMLAttributes, ReactNode } from "react";

// Permitimos que reciba cualquier prop nativa de un botón HTML (como disabled, type, onClick)
interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "ghost" | "success" | "danger" | "ghost-danger";
  size?: "sm" | "md" | "icon";
  isLoading?: boolean;
  children: ReactNode;
}

export function Button({
  variant = "primary",
  size = "md",
  isLoading,
  children,
  className = "",
  ...props
}: ButtonProps) {
  // Definimos los estilos base para cada variante
  const variants = {
    primary: "bg-action-primary hover:bg-action-primary-hover text-text-inverse",
    secondary:
      "bg-surface-raised hover:bg-border-strong text-text-primary border border-border-strong",
    ghost: "bg-transparent hover:bg-surface-raised text-text-secondary",
    success: "bg-emerald-700 hover:bg-emerald-600 text-text-inverse",
    danger: "bg-red-600 hover:bg-red-700 text-text-inverse",
    "ghost-danger":
      "bg-transparent hover:bg-red-950/40 text-red-400 hover:text-red-300",
  };

  const sizes = {
    sm: "px-3 py-1.5 text-xs",
    md: "px-4 py-2 text-sm",
    // Controles solo-ícono: cuadrado con target mínimo razonable (§13).
    icon: "p-1.5 min-h-10 min-w-10 inline-flex items-center justify-center",
  };

  return (
    <button
      className={`rounded font-semibold transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed ${sizes[size]} ${variants[variant]} ${className}`}
      {...props}
      disabled={isLoading || props.disabled}
    >
      {isLoading ? "Procesando..." : children}
    </button>
  );
}
