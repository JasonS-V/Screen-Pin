import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
// @ts-expect-error type error without @types/node package
import process from "node:process";
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [
    {
      name: "tauri-index-html-rewrite",
      configureServer(server) {
        server.middlewares.use(
          /** @param {any} req @param {any} _res @param {any} next */
          (req, _res, next) => {
            if (req.url && (req.url === "/index.html" || req.url.startsWith("/index.html?"))) {
              req.url = req.url.replace(/^\/index\.html/, "") || "/";
              if (!req.url.startsWith("/")) req.url = "/" + req.url;
              req.originalUrl = req.url;
            }
            next();
          }
        );
      },
    },
    sveltekit(),
  ],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || "127.0.0.1",
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
