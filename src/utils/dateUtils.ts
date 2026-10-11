// src/utils/dateUtils.ts

/**
 * Calcula la edad aproximada basada en una fecha de nacimiento.
 * @param birthDateStr - Fecha en formato ISO string (YYYY-MM-DD)
 * @returns Edad en años o null si la fecha es inválida/vacía
 */
export const calculateAge = (birthDateStr?: string): number | null => {
  if (!birthDateStr) return null;

  const today = new Date();
  const birthDate = new Date(birthDateStr);

  // Validación básica de fecha
  if (isNaN(birthDate.getTime())) return null;

  let age = today.getFullYear() - birthDate.getFullYear();
  const m = today.getMonth() - birthDate.getMonth();

  // Ajuste si aún no ha cumplido años este año
  if (m < 0 || (m === 0 && today.getDate() < birthDate.getDate())) {
    age--;
  }

  return age;
};

/**
 * Formatea un timestamp ISO a "dd/mm/yyyy, hh:mm a. m./p. m." (es-AR,
 * 12 horas) — el formato histórico de los entry cards. Fecha ausente,
 * vacía o inválida → "—".
 */
export const formatDateTime = (iso?: string | null): string => {
  if (!iso) return "—";
  const date = new Date(iso);
  if (isNaN(date.getTime())) return "—";
  return date.toLocaleString("es-AR", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
};
