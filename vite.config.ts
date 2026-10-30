import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
// @ts-expect-error type error without @types/node package
import process from "node:process";
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [
    react(),
    tailwindcss(),
    {
      name: "serve-package-files",
      configureServer(server) {
        server.middlewares.use((req, res, next) => {
          if (req.url && req.url.startsWith("/packages/")) {
            const fs = require("node:fs");
            const path = require("node:path");
            const cleanUrl = req.url.split("?")[0].replace(/^\/packages\//, "");
            const localRepoPath = path.resolve(process.cwd(), "repositories/community/packages", cleanUrl);
            const directPath = path.resolve(process.cwd(), "packages", cleanUrl);
            const target = fs.existsSync(localRepoPath) ? localRepoPath : (fs.existsSync(directPath) ? directPath : null);
            if (target && fs.statSync(target).isFile()) {
              const mime = target.endsWith(".png") ? "image/png" : target.endsWith(".jpg") || target.endsWith(".jpeg") ? "image/jpeg" : target.endsWith(".svg") ? "image/svg+xml" : "application/octet-stream";
              res.setHeader("Content-Type", mime);
              return fs.createReadStream(target).pipe(res);
            }
          }
          next();
        });
      },
    },
  ],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/target/**", "**/build/**"],
    },
  },
}));
