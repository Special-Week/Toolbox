import { createServer } from "node:http";
import { readFile, stat } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { gzip } from "node:zlib";
import { promisify } from "node:util";
import { createPdfCache, PdfError } from "./pdf-cache.mjs";

function json(response, status, message) {
  response.writeHead(status, { "Content-Type": "application/json; charset=utf-8", "Cache-Control": "no-store", "X-Content-Type-Options": "nosniff" });
  response.end(JSON.stringify({ error: message }));
}

export function createPdfMiddleware(options = {}) {
  const cache = createPdfCache(options);
  return (request, response, next = () => json(response, 404, "接口不存在。")) => {
    let url;
    try { url = new URL(request.url, "http://localhost"); }
    catch { json(response, 400, "请求地址无效。"); return; }
    if (url.pathname !== "/api/pdf") { next(); return; }
    if (request.method !== "GET") { response.setHeader("Allow", "GET"); json(response, 405, "请使用 GET 请求下载 PDF。"); return; }
    const inputs = url.searchParams.getAll("url");
    if (inputs.length !== 1) { json(response, 400, "请提供唯一的规范详情页 url。"); return; }
    cache.get(inputs[0]).then(result => {
      if (response.destroyed) return;
      response.writeHead(200, {
        "Content-Type": "application/pdf", "Content-Length": result.data.length,
        "Content-Disposition": "attachment; filename=standard.pdf; filename*=UTF-8''" + encodeURIComponent(result.filename),
        "Cache-Control": "no-store", "X-Content-Type-Options": "nosniff",
        "X-PDF-Code": encodeURIComponent(result.code), "X-PDF-Name": encodeURIComponent(result.name),
        "X-PDF-Filename": encodeURIComponent(result.filename), "X-PDF-Pages": String(result.pages), "X-PDF-Cache": result.cache
      });
      response.end(result.data);
    }).catch(error => {
      if (response.destroyed) return;
      if (error instanceof PdfError) json(response, error.status, error.message);
      else { console.error("PDF 缓存读写失败：", error); json(response, 500, "服务器无法读取或保存 PDF，请检查缓存目录和磁盘空间。"); }
    });
  };
}

const MIME = { ".html": "text/html; charset=utf-8", ".js": "text/javascript; charset=utf-8", ".mjs": "text/javascript; charset=utf-8", ".css": "text/css; charset=utf-8", ".wasm": "application/wasm", ".json": "application/json", ".webmanifest": "application/manifest+json", ".png": "image/png", ".ico": "image/x-icon", ".svg": "image/svg+xml", ".txt": "text/plain; charset=utf-8" };
const COMPRESSIBLE = new Set([".html", ".js", ".mjs", ".css", ".wasm", ".json", ".webmanifest", ".svg", ".txt", ".map"]);
const compress = promisify(gzip);
function acceptsGzip(header = "") {
  const qualities = new Map();
  for (const item of header.toLowerCase().split(",")) {
    const [encoding, ...parameters] = item.trim().split(";");
    const parameter = parameters.map(value => value.trim()).find(value => value.startsWith("q="));
    const quality = parameter === undefined ? 1 : Number(parameter.slice(2));
    qualities.set(encoding, Number.isFinite(quality) && quality >= 0 && quality <= 1 ? quality : 0);
  }
  return (qualities.get("gzip") ?? qualities.get("*") ?? 0) > 0;
}
export function createToolboxServer({ staticDirectory = fileURLToPath(new URL("../dist/", import.meta.url)), ...options } = {}) {
  const root = path.resolve(staticDirectory);
  const api = createPdfMiddleware(options);
  const compressed = new Map();
  return createServer((request, response) => api(request, response, async () => {
    if (!["GET", "HEAD"].includes(request.method)) { json(response, 405, "请求方法不支持。"); return; }
    try {
      const pathname = decodeURIComponent(new URL(request.url, "http://localhost").pathname);
      if (pathname.startsWith("/api/")) { json(response, 404, "接口不存在。"); return; }
      if (pathname === "/server" || pathname.startsWith("/server/") || pathname.split("/").some(segment => segment.startsWith(".")) || ["/serve.py", "/serve.mjs"].includes(pathname)) { json(response, 403, "服务端文件不允许通过网页访问。"); return; }
      const target = path.resolve(root, "." + pathname);
      if (target !== root && !target.startsWith(root + path.sep) || pathname.includes("\\") || pathname.includes("\0")) { json(response, 403, "路径不允许。"); return; }
      let file = target;
      try { if ((await stat(file)).isDirectory()) file = path.join(file, "index.html"); }
      catch (error) { if (error.code !== "ENOENT") throw error; if (!path.extname(pathname)) file = path.join(root, "index.html"); }
      const info = await stat(file);
      const headers = { "Content-Type": MIME[path.extname(file)] || "application/octet-stream", "Cache-Control": "no-cache", "X-Content-Type-Options": "nosniff", "Vary": "Accept-Encoding", "Last-Modified": info.mtime.toUTCString() };
      const since = Date.parse(request.headers["if-modified-since"]);
      if (!request.headers["if-none-match"] && Number.isFinite(since) && Math.floor(info.mtimeMs / 1000) * 1000 <= since) { response.writeHead(304, headers); response.end(); return; }
      let data;
      if (!request.headers.range && acceptsGzip(request.headers["accept-encoding"]) && COMPRESSIBLE.has(path.extname(file)) && info.size >= 256 && info.size <= 8 * 1024 * 1024) {
        let cached = compressed.get(file);
        if (!cached || cached.modified !== info.mtimeMs || cached.size !== info.size) {
          const raw = await readFile(file), zipped = await compress(raw, { level: 6 });
          cached = { modified: info.mtimeMs, size: info.size, data: zipped.length < raw.length ? zipped : null };
        }
        compressed.delete(file); compressed.set(file, cached);
        if (compressed.size > 32) compressed.delete(compressed.keys().next().value);
        if (cached.data) { data = cached.data; headers["Content-Encoding"] = "gzip"; }
      }
      data ??= await readFile(file);
      response.writeHead(200, { ...headers, "Content-Length": data.length });
      response.end(request.method === "HEAD" ? undefined : data);
    } catch (error) { json(response, error.code === "ENOENT" ? 404 : 400, "页面不存在或路径无效。"); }
  }));
}
