import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

/**
 * Base path is configurable so the same build works at `/` locally and at
 * `/aether-os/` on GitHub Pages.
 */
export default defineConfig({
  base: process.env.BASE_PATH ?? "/",
  plugins: [react()],
  server: { port: 4321, strictPort: true },
  preview: { port: 4321, strictPort: true },
  build: {
    target: "es2022",
    cssCodeSplit: false,
    sourcemap: false,
  },
});
