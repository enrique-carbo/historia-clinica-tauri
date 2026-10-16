import { ButtonHTMLAttributes, ReactNode } from "react";

// Permitimos que reciba cualquier prop nativa de un botón HTML (como disabled, type, onClick)
interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "ghost" | "success" | "danger";
  isLoading?: boolean;
  children: ReactNode;
}

export function Button({
  variant = "primary",
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
  };

  return (
    <button
      className={`px-4 py-2 text-sm font-semibold rounded transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed ${variants[variant]} ${className}`}
      {...props}
      disabled={isLoading || props.disabled}
    >
      {isLoading ? "Procesando..." : children}
    </button>
  );
}
