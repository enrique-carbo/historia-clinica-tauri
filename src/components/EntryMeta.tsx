// Piezas visuales compartidas de las cards de entries (EntryCard/SoapCard):
// badge de categoría, badge de estado, fila de firma y footer.
import { EntryCategory } from "../stores/useEntryStore";
import { formatDateTime } from "../utils/dateUtils";

const CATEGORY_CONFIG: Record<
  EntryCategory,
  { label: string; icon: string; color: string }
> = {
  SOAP_NOTE: {
    label: "Nota SOAP",
    icon: "📋",
    color: "bg-info-tint/30 text-info-text border-info-border/50",
  },
  MEDICATION: {
    label: "Medicación",
    icon: "💊",
    color: "bg-success-tint/30 text-success-text border-success-border/50",
  },
  ALLERGY: {
    label: "Alergia",
    icon: "⚠️",
    color: "bg-danger-tint/30 text-danger-text border-danger-border/50",
  },
  CONDITION: {
    label: "Diagnóstico",
    icon: "🏥",
    color: "bg-violet-tint/30 text-violet-text border-violet-border/50",
  },
};

const STATUS_CONFIG: Record<string, { label: string; color: string }> = {
  ACTIVE: { label: "Activo", color: "bg-warning-tint/30 text-warning-text" },
  RESOLVED: { label: "Resuelto", color: "bg-success-tint/30 text-success-text" },
  COMPLETED: { label: "Completado", color: "bg-border-strong text-text-primary" },
};

export function CategoryBadge({ category }: { category: EntryCategory }) {
  const config = CATEGORY_CONFIG[category] || CATEGORY_CONFIG.SOAP_NOTE;
  return (
    <span
      className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded text-xs font-medium border ${config.color}`}
    >
      <span aria-hidden="true">{config.icon}</span> {config.label}
    </span>
  );
}

export function StatusBadge({ status }: { status: string }) {
  const config = STATUS_CONFIG[status] || STATUS_CONFIG.ACTIVE;
  return (
    <span
      className={`inline-flex px-1.5 py-0.5 rounded text-[10px] font-medium ${config.color}`}
    >
      {config.label}
    </span>
  );
}

export function SignedBy({
  name,
  profession,
}: {
  name: string;
  profession?: string | null;
}) {
  return (
    <div className="flex items-center gap-1.5 min-w-0">
      <span className="text-text-muted shrink-0">Firmado:</span>
      <span className="text-text-primary font-medium truncate">{name}</span>
      {profession && (
        <span className="text-text-muted shrink-0 hidden sm:inline">
          · {profession}
        </span>
      )}
    </div>
  );
}

export function SignatureStatus({
  isVerified,
  signature,
}: {
  isVerified: boolean;
  signature: string;
}) {
  if (isVerified) {
    return (
      <span
        className="flex items-center gap-1 text-success-text"
        title={`Firma verificada: ${signature.slice(0, 16)}...`}
      >
        <svg
          aria-hidden="true"
          className="w-3.5 h-3.5"
          fill="currentColor"
          viewBox="0 0 20 20"
        >
          <path
            fillRule="evenodd"
            d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
            clipRule="evenodd"
          />
        </svg>
        Verificada
      </span>
    );
  }
  return (
    <span className="flex items-center gap-1 text-warning-text" title="Firma no verificada">
      <svg
        aria-hidden="true"
        className="w-3.5 h-3.5"
        fill="currentColor"
        viewBox="0 0 20 20"
      >
        <path
          fillRule="evenodd"
          d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z"
          clipRule="evenodd"
        />
      </svg>
      Sin verificar
    </span>
  );
}

export function EntryFooter({
  timestamp,
  id,
  className = "",
}: {
  timestamp: string;
  id: string;
  className?: string;
}) {
  return (
    <div
      className={`flex items-center justify-between text-[10px] text-text-muted ${className}`}
    >
      <span>{formatDateTime(timestamp)}</span>
      <span>ID: {id.slice(0, 8)}...</span>
    </div>
  );
}
