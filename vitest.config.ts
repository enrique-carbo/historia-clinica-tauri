import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// Config dedicada de Vitest: tiene prioridad sobre vite.config.ts.
// El plugin react aporta el transform JSX que los tests de componentes
// necesitan; jsdom es el environment de DOM. Los tests node de
// stores/utils corren igual bajo jsdom. Requiere Node >= 22.12 (vitest 5).
export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    include: ["src/**/*.{test,spec}.{ts,tsx}"],
    setupFiles: ["./src/test/setup.ts"],
  },
});
