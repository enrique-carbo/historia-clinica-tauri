import { useId, InputHTMLAttributes } from "react";

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
}

export function Input({ label, className = "", id, ...props }: InputProps) {
  const autoId = useId();
  const inputId = id ?? autoId;

  // Si nos pasan un label, lo renderizamos asociado al input; si no, solo el input
  if (label) {
    return (
      <div>
        <label
          htmlFor={inputId}
          className="block text-xs text-text-muted mb-1 font-semibold"
        >
          {label}
        </label>
        <input
          id={inputId}
          className={`w-full bg-field text-text-primary border border-border rounded px-3 py-2.5 text-sm focus:outline-none focus:ring-1 focus:ring-focus placeholder:text-text-placeholder ${className}`}
          {...props}
        />
      </div>
    );
  }

  return (
    <input
      id={inputId}
      className={`w-full bg-field text-text-primary border border-border rounded px-3 py-2.5 text-sm focus:outline-none focus:ring-1 focus:ring-focus placeholder:text-text-placeholder ${className}`}
      {...props}
    />
  );
}
