import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { offlinePlugin } from "./scripts/offline-plugin";
import { createPdfMiddleware } from "./server/http.mjs";

// 相对资源路径同时适用于域名根目录和 GitHub Pages 项目子目录。
export default defineConfig({
  plugins: [vue(), offlinePlugin(), {
    name: "toolbox-pdf-api",
    configureServer(server) { server.middlewares.use(createPdfMiddleware()); },
    configurePreviewServer(server) { server.middlewares.use(createPdfMiddleware()); },
  }],
  base: "./",
  worker: { format: "es" },
  // clean-dist.mjs 只清理构建产物，保留 serve.py 同级的 PDF 缓存。
  build: { target: "es2022", assetsInlineLimit: 0, emptyOutDir: false },
});
