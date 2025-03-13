import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Vite configuration for Tauri development
export default defineConfig({
  plugins: [react()],
  clearScreen: false, // Prevent Vite from clearing the screen so that Rust errors remain visible
  server: {
    port: 5173, // This must match the devPath URL in tauri.conf.json
    strictPort: true // Fail if the port is not available
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: ["es2021", "chrome100", "safari13"],
    minify: process.env.TAURI_DEBUG ? false : "esbuild",
    sourcemap: !!process.env.TAURI_DEBUG
  }
});
