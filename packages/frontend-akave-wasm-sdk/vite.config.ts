import { defineConfig } from "vite";
import react from "@vitejs/plugin-react-swc";

// https://vite.dev/config/
export default defineConfig({
  server: {
    fs: {
      allow: ["..", "../akave-wasm-sdk/"],
    },
  },
  plugins: [react()],
});
