import { ReactNode } from "react";

interface CardProps {
  children: ReactNode;
  className?: string;
  hover?: boolean; // Si es true, cambia el fondo al pasar el ratón (ideal para listas)
}

export function Card({ children, className = "", hover = false }: CardProps) {
  return (
    <div
      className={`bg-surface p-5 rounded-lg border border-border ${hover ? "hover:bg-surface-raised hover:border-border-strong cursor-pointer transition-colors" : ""} ${className}`}
    >
      {children}
    </div>
  );
}
