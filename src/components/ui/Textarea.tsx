import { TextareaHTMLAttributes } from "react";

interface TextareaProps extends TextareaHTMLAttributes<HTMLTextAreaElement> {
  label?: string;
}

export function Textarea({ label, className = "", ...props }: TextareaProps) {
  if (label) {
    return (
      <div>
        <label className="block text-xs text-zinc-500 mb-1 font-semibold">
          {label}
        </label>
        <textarea
          className={`w-full bg-zinc-950 text-zinc-200 border border-zinc-800 rounded px-3 py-2 text-sm font-[inherit] resize-y focus:outline-none focus:ring-1 focus:ring-blue-500 placeholder:text-zinc-700 ${className}`}
          {...props}
        />
      </div>
    );
  }

  return (
    <textarea
      className={`w-full bg-zinc-950 text-zinc-200 border border-zinc-800 rounded px-3 py-2 text-sm font-[inherit] resize-y focus:outline-none focus:ring-1 focus:ring-blue-500 placeholder:text-zinc-700 ${className}`}
      {...props}
    />
  );
}
