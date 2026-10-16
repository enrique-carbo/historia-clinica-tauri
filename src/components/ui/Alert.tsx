import { ReactNode } from "react";

interface AlertProps {
  variant?: "error" | "success" | "info";
  className?: string;
  children: ReactNode;
}

export function Alert({ variant = "info", className = "", children }: AlertProps) {
  const variants = {
    error: "bg-danger-tint border-danger-border text-danger-text",
    success: "bg-success-tint border-success-border text-success-text",
    info: "bg-info-tint border-info-border text-info-text",
  };

  return (
    <div className={`border p-3 rounded text-xs ${variants[variant]} ${className}`}>
      {children}
    </div>
  );
}
