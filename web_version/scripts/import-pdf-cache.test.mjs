import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { importPdfCache } from "./import-pdf-cache.mjs";
import { parseDetailLink, createPdfCache } from "../server/pdf-cache.mjs";
import { pdf } from "../server/test-helpers.mjs";

test("imports verified PDFs for the existing download API and leaves invalid files untouched", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "toolbox-import-"));
  const input = path.join(root, "input"), output = path.join(root, "cache");
  await mkdir(input);
  try {
    const source = "https://jjg.spc.org.cn/resmea/standard/detail.html?standno=JJF+123-2024+";
    const data = pdf(3), filename = "JJF 123-2024 测试.pdf";
    const row = { "文件名": filename, "规范编号": "JJF 123-2024 ", "规范名称": "测试", url: source, "文件hash": createHash("sha256").update(data).digest("hex") };
    await writeFile(path.join(input, filename), data);
    await writeFile(path.join(input, "bad.pdf"), data);
    await writeFile(path.join(input, "index.json"), JSON.stringify([row, { ...row, "文件名": "bad.pdf", "文件hash": "0".repeat(64), "规范编号": "JJF 124-2024", url: source.replace("123", "124") }]));
    const before = Date.now();
    const result = await importPdfCache(input, output);
    assert.equal(result.imported, 1);
    assert.equal(result.failures.length, 1);
    assert.match(result.failures[0].error, /SHA-256/);
    const standard = parseDetailLink(source);
    const meta = JSON.parse(await readFile(path.join(output, standard.key + ".json"), "utf8"));
    assert.equal(meta.pages, 3); assert.equal(meta.source, source);
    assert(Date.parse(meta.downloadedAt) >= before);
    const cached = await createPdfCache({ cacheDirectory: output, fetchImpl: () => { throw new Error("unexpected network request"); } }).get(source);
    assert.equal(cached.cache, "HIT"); assert.equal(cached.pages, 3); assert.deepEqual(cached.data, data);
  } finally { await rm(root, { recursive: true, force: true }); }
});
