import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { offlinePlugin } from "./scripts/offline-plugin";

// 相对资源路径同时适用于域名根目录和 GitHub Pages 项目子目录。
export default defineConfig({
  plugins: [vue(), offlinePlugin()],
  base: "./",
  worker: { format: "es" },
  build: { target: "es2022", assetsInlineLimit: 0 },
});
