import { ReactNode } from "react";

interface EmptyStateProps {
  title: string;
  hint?: string;
  /** `null` oculta el ícono (bloques de texto centrado); default: documento. */
  icon?: ReactNode;
  className?: string;
}

const DEFAULT_ICON_PATH =
  "M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z";

export function EmptyState({ title, hint, icon, className = "" }: EmptyStateProps) {
  return (
    <div
      className={`flex flex-col items-center justify-center py-12 text-text-muted ${className}`}
    >
      {icon !== null &&
        (icon ?? (
          <svg
            aria-hidden="true"
            className="w-11 h-11 mb-3 text-text-muted"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={1.5}
              d={DEFAULT_ICON_PATH}
            />
          </svg>
        ))}
      <p className="text-sm">{title}</p>
      {hint && <p className="text-xs text-text-muted mt-1">{hint}</p>}
    </div>
  );
}
