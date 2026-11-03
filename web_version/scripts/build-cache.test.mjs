import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, copyFile, readFile, writeFile, rm, access } from "node:fs/promises";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";
import path from "node:path";
import os from "node:os";
import { build, loadConfigFromFile } from "vite";

test("重新构建保留 PDF 缓存，清理旧产物，缓存不进入离线资源清单", async t => {
  const prefix = path.join(os.tmpdir(), "toolbox-build-cache-");
  const root = await mkdtemp(prefix);
  t.after(async () => {
    if (!path.resolve(root).startsWith(prefix)) throw new Error("Invalid test cleanup path");
    await rm(root, { recursive: true, force: true });
  });
  const source = fileURLToPath(new URL("..", import.meta.url));
  const scripts = path.join(root, "scripts"), cache = path.join(root, "dist/.pdf-cache");
  await mkdir(scripts); await mkdir(cache, { recursive: true });
  await mkdir(path.join(root, "dist/assets"));
  await copyFile(path.join(source, "scripts/clean-dist.mjs"), path.join(scripts, "clean-dist.mjs"));
  await copyFile(path.join(source, "scripts/service-worker.js"), path.join(scripts, "service-worker.js"));
  await writeFile(path.join(root, "index.html"), "<html><body>Toolbox</body></html>");
  await writeFile(path.join(root, "dist/assets/old.js"), "old asset");
  const pdf = Buffer.from("%PDF-cached-by-server"), meta = '{"code":"JJF 1059.1-2012"}';
  await writeFile(path.join(cache, "cached.pdf"), pdf); await writeFile(path.join(cache, "cached.json"), meta);
  const loaded = await loadConfigFromFile({ command: "build", mode: "production" }, path.join(source, "vite.config.ts"));
  assert(loaded);
  for (let i = 0; i < 2; i++) {
    await promisify(execFile)(process.execPath, [path.join(scripts, "clean-dist.mjs")]);
    await assert.rejects(access(path.join(root, "dist/assets/old.js")));
    await build({ ...loaded.config, root, configFile: false, logLevel: "silent" });
    assert.deepEqual(await readFile(path.join(cache, "cached.pdf")), pdf);
    assert.equal(await readFile(path.join(cache, "cached.json"), "utf8"), meta);
    const worker = await readFile(path.join(root, "dist/sw.js"), "utf8");
    assert(!worker.includes(".pdf-cache")); assert(!worker.includes("cached.pdf"));
    assert((await readFile(path.join(root, "dist/index.html"), "utf8")).includes("Toolbox"));
  }
});
