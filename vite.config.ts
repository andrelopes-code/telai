import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  build: {
    target: "chrome120",
    rolldownOptions: {
      input: {
        app: "index.html",
        player: "player.html",
      },
    },
  },
  test: {
    include: ["src/**/*.test.ts"],
  },
});
