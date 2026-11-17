import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

// Tauri dev server: fix port so tauri.conf.json devUrl stays valid
export default defineConfig({
  plugins: [solid()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "chrome110",
    minify: "esbuild",
    sourcemap: false,
  },
});
