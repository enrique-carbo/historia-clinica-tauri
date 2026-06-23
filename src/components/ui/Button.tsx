import { ButtonHTMLAttributes, ReactNode } from "react";

// Permitimos que reciba cualquier prop nativa de un botón HTML (como disabled, type, onClick)
interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "ghost" | "danger";
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
    primary: "bg-blue-600 hover:bg-blue-700 text-white",
    secondary:
      "bg-zinc-800 hover:bg-zinc-700 text-zinc-200 border border-zinc-700",
    ghost: "bg-transparent hover:bg-zinc-800 text-zinc-400",
    danger: "bg-red-600 hover:bg-red-700 text-white",
  };

  return (
    <button
      className={`px-4 py-2 text-sm font-semibold rounded transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed ${variants[variant]} ${className}`}
      disabled={isLoading || props.disabled}
      {...props}
    >
      {isLoading ? "Procesando..." : children}
    </button>
  );
}
