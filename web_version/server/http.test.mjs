import test from "node:test";
import assert from "node:assert/strict";
import { once } from "node:events";
import { writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { createToolboxServer } from "./http.mjs";
import { createPdfCache } from "./pdf-cache.mjs";
import { LINK, pdf, upstream, temporary } from "./test-helpers.mjs";

async function listen(t, server) {
  server.listen(0, "127.0.0.1"); await once(server, "listening");
  t.after(() => new Promise(resolve => { server.close(resolve); server.closeAllConnections(); }));
  return "http://127.0.0.1:" + server.address().port;
}
const query = link => "/api/pdf?" + new URLSearchParams({ url: link });

test("HTTP 下载返回 PDF 和缓存状态，重启后仍从磁盘发送相同文件", async t => {
  const cacheDirectory = await temporary(t), staticDirectory = await temporary(t);
  const data = pdf(3), remote = upstream(data);
  const origin = await listen(t, createToolboxServer({ cacheDirectory, staticDirectory, fetchImpl: remote.fetchImpl }));
  const first = await fetch(origin + query(LINK));
  assert.equal(first.status, 200); assert.equal(first.headers.get("content-type"), "application/pdf");
  assert.equal(first.headers.get("x-pdf-cache"), "MISS"); assert.equal(first.headers.get("x-pdf-pages"), "3");
  assert.equal(decodeURIComponent(first.headers.get("x-pdf-code")), "JJF 1059.1-2012");
  assert.match(first.headers.get("content-disposition"), /^attachment;.*filename\*=UTF-8''/);
  assert.deepEqual(Buffer.from(await first.arrayBuffer()), data);
  const second = await fetch(origin + query(LINK.replace("+", "%20") + "&rows=20"));
  assert.equal(second.headers.get("x-pdf-cache"), "HIT"); assert.equal(remote.calls.length, 3);
  assert.deepEqual(Buffer.from(await second.arrayBuffer()), data);
  const restart = await listen(t, createToolboxServer({ cacheDirectory, staticDirectory, fetchImpl() { assert.fail("重启后命中本地缓存"); } }));
  const saved = await fetch(restart + query(LINK));
  assert.equal(saved.headers.get("x-pdf-cache"), "HIT"); assert.deepEqual(Buffer.from(await saved.arrayBuffer()), data);
});

test("无效 API 请求不访问上游，错误使用 JSON，静态资源正常提供", async t => {
  const cacheDirectory = await temporary(t), staticDirectory = await temporary(t), remote = upstream();
  await writeFile(path.join(staticDirectory, "index.html"), "<html>Toolbox</html>");
  await writeFile(path.join(staticDirectory, "engine.wasm"), "wasm-data");
  const origin = await listen(t, createToolboxServer({ cacheDirectory, staticDirectory, fetchImpl: remote.fetchImpl }));
  for (const [url, options, status] of [["/api/pdf", {}, 400], [query("http://127.0.0.1"), {}, 400], [query(LINK) + "&url=other", {}, 400], [query(LINK), { method: "POST" }, 405], ["/api/unknown", {}, 404]]) {
    const response = await fetch(origin + url, options);
    assert.equal(response.status, status); assert.match(response.headers.get("content-type"), /application\/json/);
    assert.equal(typeof (await response.json()).error, "string");
  }
  assert.equal(remote.calls.length, 0);
  const home = await fetch(origin + "/"); assert.equal(await home.text(), "<html>Toolbox</html>");
  const wasm = await fetch(origin + "/engine.wasm", { method: "HEAD" });
  assert.equal(wasm.status, 200); assert.equal(wasm.headers.get("content-type"), "application/wasm"); assert.equal(await wasm.text(), "");
  assert.equal((await fetch(origin + "/missing.js")).status, 404);
  assert.equal((await fetch(origin + "/%2e%2e%2fserver%2findex.mjs")).status, 403);
  assert.equal((await fetch(origin + "/%5c..%5cserver%5cindex.mjs")).status, 403);
});

test("Vite 开发及预览服务也使用同一磁盘缓存接口", async t => {
  const cacheDirectory = await temporary(t), data = pdf(2);
  await createPdfCache({ cacheDirectory, fetchImpl: upstream(data).fetchImpl }).get(LINK);
  const previous = process.env.TOOLBOX_PDF_CACHE;
  process.env.TOOLBOX_PDF_CACHE = cacheDirectory;
  t.after(() => { if (previous === undefined) delete process.env.TOOLBOX_PDF_CACHE; else process.env.TOOLBOX_PDF_CACHE = previous; });
  const { createServer, preview } = await import("vite");
  const root = fileURLToPath(new URL("..", import.meta.url));
  const dev = await createServer({ root, server: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  t.after(() => dev.close()); await dev.listen();
  const devOrigin = "http://127.0.0.1:" + dev.httpServer.address().port;
  const searchModule = await fetch(devOrigin + "/src/standard-catalog.ts");
  assert.equal(searchModule.status, 200);
  await searchModule.text();
  const catalogPath = path.resolve(root, "../data/standards/catalog.json").replaceAll("\\", "/");
  const catalogData = await fetch(devOrigin + "/@fs/" + catalogPath + "?import");
  assert.equal(catalogData.status, 200);
  assert((await catalogData.text()).includes("JJF 1059.1-2012"));
  const response = await fetch("http://127.0.0.1:" + dev.httpServer.address().port + query(LINK));
  assert.equal(response.status, 200); assert.equal(response.headers.get("x-pdf-cache"), "HIT");
  assert.deepEqual(Buffer.from(await response.arrayBuffer()), data);
  const built = await preview({ root, preview: { host: "127.0.0.1", port: 0 }, logLevel: "silent" });
  t.after(() => new Promise(resolve => { built.httpServer.close(resolve); built.httpServer.closeAllConnections(); }));
  const cached = await fetch("http://127.0.0.1:" + built.httpServer.address().port + query(LINK));
  assert.equal(cached.status, 200); assert.equal(cached.headers.get("x-pdf-cache"), "HIT");
  assert.deepEqual(Buffer.from(await cached.arrayBuffer()), data);
});

test("保留静态资源 gzip、HEAD 和条件请求，并禁止读取服务端文件", async t => {
  const cacheDirectory = await temporary(t), staticDirectory = await temporary(t);
  const text = "const toolbox = 'static gzip content';\n".repeat(100);
  await writeFile(path.join(staticDirectory, "app.js"), text);
  await writeFile(path.join(staticDirectory, "serve.py"), "private launcher");
  const origin = await listen(t, createToolboxServer({ cacheDirectory, staticDirectory, fetchImpl: upstream().fetchImpl }));
  const zipped = await fetch(origin + "/app.js", { headers: { "accept-encoding": "gzip" } });
  assert.equal(zipped.headers.get("content-encoding"), "gzip"); assert.equal(await zipped.text(), text);
  assert(Number(zipped.headers.get("content-length")) < Buffer.byteLength(text));
  assert.equal(zipped.headers.get("vary"), "Accept-Encoding");
  const raw = await fetch(origin + "/app.js", { headers: { "accept-encoding": "gzip;q=0, *;q=1" } });
  assert.equal(raw.headers.get("content-encoding"), null); assert.equal(await raw.text(), text);
  const head = await fetch(origin + "/app.js", { method: "HEAD", headers: { "accept-encoding": "gzip" } });
  assert.equal(head.headers.get("content-length"), zipped.headers.get("content-length")); assert.equal(await head.text(), "");
  const cached = await fetch(origin + "/app.js", { headers: { "if-modified-since": zipped.headers.get("last-modified") } });
  assert.equal(cached.status, 304); assert.equal(await cached.text(), "");
  for (const file of ["/server/index.mjs", "/server/vendor/pdfjs/pdf.min.mjs", "/.pdf-cache/test.pdf", "/serve.py", "/serve.mjs"]) {
    assert.equal((await fetch(origin + file)).status, 403, file);
  }
});
