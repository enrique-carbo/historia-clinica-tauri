import { ReactNode } from "react";

interface CardProps {
  children: ReactNode;
  className?: string;
  hover?: boolean; // Si es true, cambia el fondo al pasar el ratón (ideal para listas)
}

export function Card({ children, className = "", hover = false }: CardProps) {
  return (
    <div
      className={`bg-zinc-900 p-5 rounded-lg border border-zinc-800 ${hover ? "hover:bg-zinc-800 hover:border-zinc-700 cursor-pointer transition-colors" : ""} ${className}`}
    >
      {children}
    </div>
  );
}
