import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  // Relative asset paths so the build works at any mount point
  // (GitHub Pages serves this under /ledger/).
  base: "./",
  plugins: [react()],
  server: {
    port: 5180,
    strictPort: true,
  },
  build: {
    target: "es2022",
  },
});
