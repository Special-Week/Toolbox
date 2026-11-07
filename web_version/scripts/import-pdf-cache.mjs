import { createHash, randomUUID } from "node:crypto";
import { mkdir, readFile, readdir, rename, unlink, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { DEFAULT_CACHE_DIRECTORY, defaultFilename, inspectPdf, parseDetailLink } from "../server/pdf-cache.mjs";

export async function importPdfCache(input, output) {
  const files = await readdir(input);
  const records = new Map();
  for (const file of files.filter(file => file.toLowerCase().endsWith(".json"))) {
    const rows = JSON.parse(await readFile(path.join(input, file), "utf8"));
    if (!Array.isArray(rows)) throw new Error(`${file}: 索引不是数组`);
    for (const row of rows) {
      const name = row["文件名"];
      if (typeof name !== "string" || !name) continue;
      if (records.has(name)) throw new Error(`索引文件名重复: ${name}`);
      records.set(name, row);
    }
  }
  await mkdir(output, { recursive: true });
  const failures = [], keys = new Set();
  let imported = 0;
  for (const file of files.filter(file => file.toLowerCase().endsWith(".pdf"))) {
    let temporaryPdf, temporaryJson;
    try {
      const row = records.get(file);
      if (!row) throw new Error("索引中没有对应的文件名");
      const standard = parseDetailLink(row.url);
      if (typeof row["规范编号"] !== "string" || standard.code !== row["规范编号"].trim().replace(/ +/g, " ").toUpperCase() || typeof row["规范名称"] !== "string") throw new Error("索引中的编号或名称无效");
      if (keys.has(standard.key)) throw new Error("规范编号重复");
      keys.add(standard.key);
      const data = await readFile(path.join(input, file));
      if (!/^[a-f0-9]{64}$/i.test(row["文件hash"]) || createHash("sha256").update(data).digest("hex") !== row["文件hash"].toLowerCase()) throw new Error("文件 SHA-256 与索引不一致");
      const pages = await inspectPdf(data);
      const meta = { version: 1, code: standard.code, name: row["规范名称"], pages, filename: defaultFilename(standard.code, row["规范名称"]), source: row.url, downloadedAt: new Date().toISOString() };
      const target = path.join(output, standard.key);
      const suffix = `.${randomUUID()}.tmp`;
      temporaryPdf = target + ".pdf" + suffix;
      temporaryJson = target + ".json" + suffix;
      await writeFile(temporaryPdf, data, { flag: "wx" });
      await writeFile(temporaryJson, JSON.stringify(meta, null, 2), { flag: "wx" });
      await rename(temporaryPdf, target + ".pdf");
      await rename(temporaryJson, target + ".json");
      imported++;
      if (imported % 200 === 0) console.log(`已导入 ${imported} 份 PDF`);
    } catch (error) {
      failures.push({ file, error: error.message });
    } finally {
      for (const temporary of [temporaryPdf, temporaryJson]) if (temporary) await unlink(temporary).catch(error => { if (error.code !== "ENOENT") throw error; });
    }
  }
  return { imported, failures };
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  const root = fileURLToPath(new URL("..", import.meta.url));
  const result = await importPdfCache(path.join(root, "待处理"), process.env.TOOLBOX_PDF_CACHE || DEFAULT_CACHE_DIRECTORY);
  console.log(JSON.stringify(result, null, 2));
  if (result.failures.length) process.exitCode = 1;
}
