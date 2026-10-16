import { afterEach } from "vitest";
import { cleanup } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";

// RTL auto-cleanup solo se registra si vitest expone globals; con config
// por-import lo hacemos explícito para que cada test arranque con DOM limpio.
afterEach(cleanup);
