import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const Host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: Host || false,
    hmr: Host ? { protocol: "ws", host: Host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
