import { useThemeStore } from "../../stores/useThemeStore";

interface ThemeToggleProps {
  className?: string;
}

export function ThemeToggle({ className = "" }: ThemeToggleProps) {
  const { theme, toggleTheme } = useThemeStore();
  const isDark = theme === "dark";

  return (
    <button
      onClick={toggleTheme}
      aria-label={isDark ? "Cambiar a tema claro" : "Cambiar a tema oscuro"}
      title={isDark ? "Tema oscuro" : "Tema claro"}
      className={`shrink-0 rounded-lg p-2 text-sm text-text-secondary hover:text-text-primary hover:bg-surface-raised transition-colors cursor-pointer ${className}`}
    >
      <span aria-hidden="true">{isDark ? "🌙" : "☀️"}</span>
    </button>
  );
}
