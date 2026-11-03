import { copyFile, cp, mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = fileURLToPath(new URL("..", import.meta.url));
const output = path.join(root, "dist");
const runtime = path.join(output, "server");
await mkdir(runtime, { recursive: true });
for (const file of ["index.mjs", "http.mjs", "pdf-cache.mjs"]) {
  await copyFile(path.join(root, "server", file), path.join(runtime, file));
}
await cp(path.join(root, "server/vendor"), path.join(runtime, "vendor"), { recursive: true });
for (const file of ["serve.py", "serve.mjs"]) await copyFile(path.join(root, "deploy", file), path.join(output, file));
console.log("已打包完整服务：dist/serve.py、serve.mjs 及 server（运行需 Node.js 22.12+）。");
