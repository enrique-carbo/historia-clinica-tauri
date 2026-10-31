// Traducción de los errores del backend a texto para el usuario.
//
// Contrato con Rust (`src-tauri/src/error.rs`): el `invoke` rechaza con
// `{ code, params? }`; el texto nunca viaja desde Rust. Este módulo es el
// único lugar del frontend que conoce los códigos y sus mensajes — cuando
// llegue i18n (Fase 1, ver `docs/i18n.md`) este diccionario se reemplaza
// por uno por locale, sin tocar los call sites.
//
// Errores internos (`ERR_INTERNAL`, DB/IO/cripto): el detalle técnico viene
// en `params.detail` y se usa como fallback, pero no se traduce.
//
// Los rechazos que no son del backend (strings legacy, `Error` de JS) se
// muestran tal cual: los tests y los helpers del frontend todavía los usan.

export type CommandErrorPayload = {
  code: string;
  params?: Record<string, string>;
};

type Params = Record<string, string>;

const MESSAGES: Record<string, string | ((params: Params) => string)> = {
  // --- Infraestructura ---
  ERR_INTERNAL: "Ocurrió un error inesperado.",
  ERR_STORAGE_PATH: "No se pudo determinar la ruta de almacenamiento.",
  ERR_DB_NOT_INITIALIZED:
    "La base de datos no está inicializada. Reiniciá la aplicación.",

  // --- Sesión y roles ---
  ERR_SESSION_REQUIRED: "Sesión no iniciada. Ingresá de nuevo.",
  ERR_ADMIN_REQUIRED: "Esta acción requiere rol administrador.",
  ERR_ADMIN_OR_ASSISTANT_REQUIRED:
    "Esta acción requiere rol administrador o asistente.",

  // --- Autenticación ---
  ERR_REGISTRATION_CLOSED:
    "El registro está cerrado. Un administrador debe darte de alta desde el panel.",
  ERR_USERNAME_TAKEN: "Ya existe un usuario con ese nombre.",
  ERR_USER_DISABLED: "Usuario desactivado. Contactá al administrador.",
  ERR_WRONG_PASSWORD: "Contraseña incorrecta.",
  ERR_USER_NOT_FOUND: "El usuario no existe.",
  ERR_PASSWORD_TOO_SHORT: (p) =>
    `La contraseña debe tener al menos ${p.min ?? "8"} caracteres.`,
  ERR_PASSWORD_SAME_AS_CURRENT:
    "La nueva contraseña debe ser distinta a la actual.",

  // --- Administración de usuarios ---
  ERR_USERNAME_EMPTY: "El nombre de usuario no puede estar vacío.",
  ERR_INVALID_ROLE: (p) => `Rol inválido: ${p.role ?? "desconocido"}.`,
  ERR_SELF_DEACTIVATE: "No podés desactivar tu propia cuenta.",
  ERR_LAST_ADMIN:
    "No se puede desactivar: quedaría la instalación sin administradores activos.",
  ERR_OWN_PASSWORD_RESET:
    "Para tu propia cuenta usá «Cambiar contraseña» desde tu perfil.",

  // --- Bóveda y semilla ---
  ERR_VAULT_LOCKED:
    "La bóveda está cerrada. Volvé a iniciar sesión para desbloquearla.",
  ERR_SIGNING_KEY_MISSING:
    "No hay llave de firma en memoria. ¿La bóveda está cerrada?",
  ERR_SEED_EMPTY: "Ingresá la frase semilla.",
  ERR_SEED_NOT_CONFIGURED:
    "Esta instalación no tiene una frase semilla configurada. Recuperá con un respaldo o con una contraseña válida.",
  ERR_SEED_INVALID: "Frase semilla incorrecta.",
  ERR_SEED_MALFORMED: "La frase semilla no es válida.",
  ERR_NO_ACTIVE_ADMIN:
    "No hay usuarios administradores activos en esta instalación.",
  SEED_REQUIRED: "Se requiere la frase semilla de recuperación.",

  // --- Fichas y accesos ---
  ERR_ENTITY_NOT_FOUND: "Ficha no encontrada.",
  ERR_NOT_PATIENT_ENTITY:
    "El acceso de paciente solo aplica a fichas de tipo paciente.",
  ERR_PATIENT_ACCESS_EXISTS: (p) =>
    `La ficha ya tiene un acceso activo: @${p.username ?? ""}.`,
  ERR_BLIND_INDEX_FIELD_MISSING: (p) =>
    `Falta el campo de identificación: ${p.field ?? ""}.`,

  // --- Entries ---
  ERR_INVALID_CATEGORY: (p) => `Categoría inválida: ${p.category ?? ""}.`,
  ERR_INVALID_STATUS: (p) => `Estado inválido: ${p.status ?? ""}.`,
  ERR_PROFILE_INCOMPLETE:
    "Perfil profesional incompleto: completá nombre, profesión y matrícula antes de crear entries.",

  // --- Perfil profesional ---
  ERR_FOREIGN_PROFILE: "No podés editar el perfil de otro usuario.",

  // --- Export y respaldos ---
  ERR_EXPORT_EMPTY: "El paciente no tiene entries para exportar.",
  ERR_BACKUP_SIGNATURE_INVALID: (p) =>
    `Firma del manifest inválida: ${p.reason ?? ""}. El respaldo fue alterado o no corresponde a esta instalación; no se puede restaurar.`,
  ERR_BACKUP_UNVERIFIED: (p) =>
    `El respaldo no tiene una firma verificable por esta instalación (${p.reason ?? "firma no verificada"}). Verificá el origen y confirmá la restauración de todos modos.`,
};

/** Extrae `{ code, params }` de un rechazo del backend, si lo es. */
export function parseCommandError(err: unknown): CommandErrorPayload | null {
  if (typeof err !== "object" || err === null) return null;
  const candidate = err as { code?: unknown; params?: unknown };
  if (typeof candidate.code !== "string") return null;
  const params =
    typeof candidate.params === "object" && candidate.params !== null
      ? (candidate.params as Params)
      : {};
  return { code: candidate.code, params };
}

/** Código estable del error (`ERR_...`), o `null` si no vino del backend. */
export function errorCode(err: unknown): string | null {
  return parseCommandError(err)?.code ?? null;
}

/** `true` si el rechazo es del backend con ese código. */
export function isErrorCode(err: unknown, code: string): boolean {
  return errorCode(err) === code;
}

/**
 * Texto para el usuario a partir de cualquier rechazo:
 * - `{ code, params }` del backend → mensaje del diccionario;
 * - `ERR_INTERNAL` → detalle técnico si lo hay (no se traduce);
 * - string/Error legacy → tal cual (compatibilidad y tests).
 */
export function formatError(err: unknown): string {
  const payload = parseCommandError(err);
  if (payload) {
    const { code, params = {} } = payload;
    const entry = MESSAGES[code];
    if (entry !== undefined) {
      const message = typeof entry === "function" ? entry(params) : entry;
      if (code === "ERR_INTERNAL" && params.detail) {
        return `${message} (${params.detail})`;
      }
      return message;
    }
    // Código desconocido (backend más nuevo que el frontend): el detalle
    // técnico, si existe, es mejor que "[object Object]".
    return params.detail ?? code;
  }

  if (err instanceof Error) return err.message;
  if (typeof err === "object" && err !== null) {
    // Objeto sin `code`: no es del backend; nunca mostrar "[object Object]".
    return "Ocurrió un error inesperado.";
  }
  return String(err);
}
