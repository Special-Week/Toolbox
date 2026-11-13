// 发布目录默认同时提供静态资源与本地 PDF 缓存，可用环境变量覆盖。
import { fileURLToPath } from "node:url";

process.env.TOOLBOX_STATIC_DIRECTORY ||= fileURLToPath(
  new URL("./", import.meta.url),
);
process.env.TOOLBOX_PDF_CACHE ||= fileURLToPath(
  new URL("./.pdf-cache/", import.meta.url),
);
await import("./server/index.mjs");
