import { useId, TextareaHTMLAttributes } from "react";

interface TextareaProps extends TextareaHTMLAttributes<HTMLTextAreaElement> {
  label?: string;
}

export function Textarea({
  label,
  className = "",
  id,
  ...props
}: TextareaProps) {
  const autoId = useId();
  const textareaId = id ?? autoId;

  if (label) {
    return (
      <div>
        <label
          htmlFor={textareaId}
          className="block text-xs text-text-muted mb-1 font-semibold"
        >
          {label}
        </label>
        <textarea
          id={textareaId}
          className={`w-full bg-field text-text-primary border border-border rounded px-3 py-2 text-sm font-[inherit] resize-y focus:outline-none focus:ring-1 focus:ring-focus placeholder:text-text-placeholder ${className}`}
          {...props}
        />
      </div>
    );
  }

  return (
    <textarea
      id={textareaId}
      className={`w-full bg-field text-text-primary border border-border rounded px-3 py-2 text-sm font-[inherit] resize-y focus:outline-none focus:ring-1 focus:ring-focus placeholder:text-text-placeholder ${className}`}
      {...props}
    />
  );
}
