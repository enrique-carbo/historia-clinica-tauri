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
 * Formatea una fecha ISO a un string legible (ej: "15 Jul 2026")
 */
export const formatDate = (dateStr?: string): string => {
  if (!dateStr) return "—";
  try {
    return new Date(dateStr).toLocaleDateString('es-ES', {
      year: 'numeric',
      month: 'short',
      day: 'numeric'
    });
  } catch {
    return "—";
  }
};
