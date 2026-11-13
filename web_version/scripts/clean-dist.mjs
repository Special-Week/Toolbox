// 清理构建输出时保留 PDF 缓存，并限制删除范围在输出目录内。
import { mkdir, readdir, rm } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

const directory = path.resolve(
  fileURLToPath(new URL("../dist/", import.meta.url)),
);
await mkdir(directory, { recursive: true });
for (const name of await readdir(directory)) {
  if (name === ".pdf-cache") continue;
  const target = path.resolve(directory, name);
  if (path.dirname(target) !== directory)
    throw new Error("拒绝清理构建目录以外的文件。");
  await rm(target, { recursive: true, force: true });
}
