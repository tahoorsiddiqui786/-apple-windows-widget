import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri expects a fixed dev-server port and must not have Vite clear its output.
// The dev server binds to localhost only: nothing is exposed to the network.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  envPrefix: ["VITE_"],
  build: {
    // WebView2 on Windows 11 is evergreen Chromium.
    target: "es2022",
    sourcemap: false,
    // Keep the dock a single small bundle; no remote chunks or CDNs.
    assetsInlineLimit: 0,
  },
});
