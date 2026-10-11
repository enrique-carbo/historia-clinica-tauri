import { defineConfig } from "vitest/config";

// Config dedicada de Vitest: tiene prioridad sobre vite.config.ts y deja
// fuera plugins (react/tailwind) que los tests unit no necesitan.
// Requiere Node >= 22.12 (vitest 5 engines).
export default defineConfig({
  test: {
    environment: "node",
    include: ["src/**/*.{test,spec}.{ts,tsx}"],
  },
});
