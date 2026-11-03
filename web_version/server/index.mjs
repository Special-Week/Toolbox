import { createToolboxServer } from "./http.mjs";
import { DEFAULT_CACHE_DIRECTORY } from "./pdf-cache.mjs";

const host = process.env.HOST || "0.0.0.0";
const port = Number(process.env.PORT || 4173);
if (!Number.isInteger(port) || port < 1 || port > 65535) throw new Error("PORT 必须是 1～65535 的整数。");
const server = createToolboxServer({ staticDirectory: process.env.TOOLBOX_STATIC_DIRECTORY });
server.on("error", error => { console.error("工具箱服务启动失败：", error.message); process.exitCode = 1; });
server.listen(port, host, () => {
  console.log(`工具箱：http://${host}:${port}/`);
  console.log("PDF 缓存：" + (process.env.TOOLBOX_PDF_CACHE || DEFAULT_CACHE_DIRECTORY));
});
