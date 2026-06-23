import { ReactNode } from "react";

interface AlertProps {
  variant?: "error" | "success" | "info";
  children: ReactNode;
}

export function Alert({ variant = "info", children }: AlertProps) {
  const variants = {
    error: "bg-red-950 border-red-800 text-red-400",
    success: "bg-green-950 border-green-800 text-green-400",
    info: "bg-blue-950 border-blue-800 text-blue-400",
  };

  return (
    <div className={`border p-3 m-3 rounded text-xs ${variants[variant]}`}>
      {children}
    </div>
  );
}
