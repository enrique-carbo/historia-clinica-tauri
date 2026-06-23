import { InputHTMLAttributes } from "react";

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
}

export function Input({ label, className = "", ...props }: InputProps) {
  // Si nos pasan un label, lo renderizamos; si no, solo el input
  if (label) {
    return (
      <div>
        <label className="block text-xs text-zinc-500 mb-1 font-semibold">
          {label}
        </label>
        <input
          className={`w-full bg-zinc-950 text-zinc-200 border border-zinc-800 rounded px-3 py-2.5 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 placeholder:text-zinc-700 ${className}`}
          {...props}
        />
      </div>
    );
  }

  return (
    <input
      className={`w-full bg-zinc-950 text-zinc-200 border border-zinc-800 rounded px-3 py-2.5 text-sm focus:outline-none focus:ring-1 focus:ring-blue-500 placeholder:text-zinc-700 ${className}`}
      {...props}
    />
  );
}
