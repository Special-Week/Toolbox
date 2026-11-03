import { createHash, randomUUID } from "node:crypto";
import { mkdir, open, readFile, rename, stat, unlink } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";
import * as pdfjs from "./vendor/pdfjs/pdf.min.mjs";

pdfjs.GlobalWorkerOptions.workerSrc = new URL("./vendor/pdfjs/pdf.worker.min.mjs", import.meta.url).href;
export const DEFAULT_CACHE_DIRECTORY = fileURLToPath(new URL("../.pdf-cache/", import.meta.url));
const ORIGIN = "https://jjg.spc.org.cn";
const PREVIEW = ORIGIN + "/resmea/view/stdonline";
const MAX_PDF_BYTES = 50 * 1024 * 1024;
const ERROR_HASH = "f487c0adbe09950cd6effdbca2af0b9d59b4c53ed7adbf5e58cfff694b3e2ed8";
const AGENT = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36";

export class PdfError extends Error {
  constructor(message, status = 502) { super(message); this.name = "PdfError"; this.status = status; }
}

export function parseDetailLink(input) {
  let url;
  try {
    if (typeof input !== "string" || input.length > 2000 || /[\x00-\x1f]/.test(input)) throw new Error();
    url = new URL(input.trim());
  } catch { throw new PdfError("请输入有效的规范详情页链接。", 400); }
  if (url.origin !== ORIGIN || url.username || url.password || url.pathname !== "/resmea/standard/detail.html") {
    throw new PdfError("仅支持国家计量技术规范全文公开系统的 HTTPS 详情页链接。", 400);
  }
  const codes = url.searchParams.getAll("standno");
  if (codes.length !== 1 || !codes[0].trim() || !/^[A-Za-z0-9/ ._-]{1,100}$/.test(codes[0])) {
    throw new PdfError("链接中没有唯一有效的规范编号。", 400);
  }
  const code = codes[0].trim().replace(/ +/g, " ").toUpperCase();
  const detail = new URL("/resmea/standard/detail.html", ORIGIN);
  detail.searchParams.set("standno", code);
  return { code, url: detail.href, key: createHash("sha256").update(code).digest("hex") };
}

export function defaultFilename(code, name) {
  let label = (code + (name ? " " + name : "")).replace(/[\\/:*?"<>|\x00-\x1f]/g, "_")
    .slice(0, 180).replace(/[\uD800-\uDBFF]$/, "").replace(/[. ]+$/g, "");
  if (/^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\.|$)/i.test(label)) label = "_" + label;
  return label + ".pdf";
}

function standardName(page) {
  const match = page.match(/<title\b[^>]*>([\s\S]*?)<\/title\s*>/i);
  if (!match) return "";
  const entities = { amp: "&", lt: "<", gt: ">", quot: '"', apos: "'", nbsp: " " };
  const title = match[1].replace(/&(#x[0-9a-f]+|#\d+|amp|lt|gt|quot|apos|nbsp);/gi, (raw, entity) => {
    if (entity[0] !== "#") return entities[entity.toLowerCase()];
    const value = entity[1].toLowerCase() === "x" ? parseInt(entity.slice(2), 16) : Number(entity.slice(1));
    return value > 0 && value <= 0x10ffff && !(value >= 0xd800 && value <= 0xdfff) ? String.fromCodePoint(value) : raw;
  }).trim();
  const suffix = " - 国家计量技术规范全文公开系统";
  return title.endsWith(suffix) ? title.slice(0, -suffix.length).trim().slice(0, 500) : "";
}

function parameter(page, key) {
  const match = page.match(new RegExp('\\bvar\\s+' + key + '\\s*=\\s*("(?:\\\\.|[^"\\\\])*")'));
  let value;
  try { value = match && JSON.parse(match[1]); } catch { /* 参数校验统一返回格式错误。 */ }
  if (typeof value !== "string" || !value || value.length > 4096 || !/^[A-Za-z0-9+/=]+$/.test(value)) {
    throw new PdfError("预览页面缺少有效参数，网站可能已更新。");
  }
  return value;
}

async function readBounded(response, limit) {
  if (Number(response.headers.get("content-length")) > limit) {
    await response.body?.cancel();
    throw new PdfError("网站返回的数据超过大小限制（PDF 上限 50 MB）。");
  }
  const chunks = [];
  let size = 0;
  if (!response.body) return Buffer.alloc(0);
  const reader = response.body.getReader();
  for (;;) {
    const { value, done } = await reader.read();
    if (done) break;
    size += value.byteLength;
    if (size > limit) { await reader.cancel(); throw new PdfError("网站返回的数据超过大小限制（PDF 上限 50 MB）。"); }
    chunks.push(Buffer.from(value));
  }
  return Buffer.concat(chunks, size);
}

export async function inspectPdf(data) {
  if (!data.length || data.length > MAX_PDF_BYTES || data.subarray(0, 5).toString() !== "%PDF-") throw new PdfError("网站没有返回有效的 PDF。");
  if (createHash("sha256").update(data).digest("hex") === ERROR_HASH) throw new PdfError("网站返回了在线阅读加载失败的提示文件。");
  // PDF.js 会转移输入缓冲区，解析时复制，缓存和响应始终使用原始字节。
  const task = pdfjs.getDocument({ data: new Uint8Array(data), disableFontFace: true, isEvalSupported: false, stopAtErrors: true, verbosity: 0 });
  try {
    const document = await task.promise;
    if (!Number.isInteger(document.numPages) || document.numPages < 1) throw new PdfError("PDF 没有页面。");
    return document.numPages;
  } catch (error) {
    if (error instanceof PdfError) throw error;
    throw new PdfError(error?.name === "PasswordException" ? "PDF 需要密码，无法检查。" : "网站返回的 PDF 无法解析。");
  } finally { await task.destroy(); }
}

async function downloadStandard(standard, fetchImpl) {
  const cookies = new Map();
  async function request(url, options, limit) {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 30000);
    try {
      const headers = { "User-Agent": AGENT, ...options.headers };
      if (cookies.size) headers.Cookie = Array.from(cookies, ([key, value]) => key + "=" + value).join("; ");
      const response = await fetchImpl(url, { ...options, headers, redirect: "error", signal: controller.signal });
      if (response.status === 429) throw new PdfError("规范网站请求过于频繁（HTTP 429），请稍后再试。", 429);
      if (!response.ok) throw new PdfError("规范网站返回 HTTP " + response.status + "。");
      for (const cookie of response.headers.getSetCookie()) {
        const pair = cookie.split(";", 1)[0], index = pair.indexOf("=");
        if (index > 0) cookies.set(pair.slice(0, index), pair.slice(index + 1));
      }
      return await readBounded(response, limit);
    } catch (error) {
      if (error instanceof PdfError) throw error;
      throw new PdfError(error?.name === "AbortError" ? "获取规范超时，请稍后重试。" : "服务器无法连接规范网站，请稍后重试。");
    } finally { clearTimeout(timeout); }
  }
  let name = "";
  try { name = standardName((await request(standard.url, { headers: { Referer: ORIGIN + "/resmea/" } }, 1024 * 1024)).toString("utf8")); }
  catch (error) { if (error.status === 429) throw error; }
  const preview = (await request(PREVIEW, { method: "POST", headers: { Referer: standard.url }, body: new URLSearchParams({ a100: standard.code, ismobile: "1" }) }, 1024 * 1024)).toString("utf8");
  const url = new URL("/resmea/view/onlinereading", ORIGIN);
  url.searchParams.set("token", parameter(preview, "token"));
  const data = await request(url.href, { headers: { Referer: PREVIEW, myfoxit: parameter(preview, "enc"), "X-Requested-With": "XMLHttpRequest", Accept: "*/*" } }, MAX_PDF_BYTES);
  const pages = await inspectPdf(data);
  return { code: standard.code, name, pages, filename: defaultFilename(standard.code, name), data };
}

async function atomicWrite(target, data) {
  const temporary = target + "." + randomUUID() + ".tmp";
  let file;
  try {
    file = await open(temporary, "wx", 0o600);
    await file.writeFile(data); await file.sync(); await file.close(); file = null;
    await rename(temporary, target);
  } finally {
    if (file) await file.close();
    await unlink(temporary).catch(error => { if (error.code !== "ENOENT") throw error; });
  }
}

export function createPdfCache({ cacheDirectory = process.env.TOOLBOX_PDF_CACHE || DEFAULT_CACHE_DIRECTORY, fetchImpl = fetch } = {}) {
  const directory = path.resolve(cacheDirectory);
  const pending = new Map();
  let queue = Promise.resolve();
  async function readCache(standard) {
    const pdf = path.join(directory, standard.key + ".pdf");
    try {
      const info = await stat(pdf);
      if (!info.isFile() || info.size > MAX_PDF_BYTES) return null;
      const data = await readFile(pdf);
      let pages;
      try { pages = await inspectPdf(data); } catch (error) { if (error instanceof PdfError) return null; throw error; }
      let name = "";
      try {
        const metaPath = path.join(directory, standard.key + ".json");
        if ((await stat(metaPath)).size <= 65536) {
          const meta = JSON.parse(await readFile(metaPath, "utf8"));
          if (meta.code === standard.code && typeof meta.name === "string" && meta.name.length <= 500 && meta.name.isWellFormed()) name = meta.name;
        }
      } catch (error) { if (error.code && error.code !== "ENOENT") throw error; }
      return { code: standard.code, name, filename: defaultFilename(standard.code, name), pages, data, cache: "HIT" };
    } catch (error) { if (error.code === "ENOENT") return null; throw error; }
  }
  async function prepare(standard) {
    const cached = await readCache(standard);
    if (cached) return cached;
    // 全站顺序访问上游；相同规范的并发请求由 pending 合并。
    const job = queue.then(async () => {
      const existing = await readCache(standard);
      if (existing) return existing;
      const result = await downloadStandard(standard, fetchImpl);
      await mkdir(directory, { recursive: true });
      const { data, ...meta } = result;
      await atomicWrite(path.join(directory, standard.key + ".json"), JSON.stringify({ version: 1, ...meta, source: standard.url, downloadedAt: new Date().toISOString() }, null, 2));
      await atomicWrite(path.join(directory, standard.key + ".pdf"), data);
      return { ...result, cache: "MISS" };
    });
    queue = job.catch(() => {});
    return job;
  }
  return {
    directory,
    async get(input) {
      const standard = parseDetailLink(input);
      const running = pending.get(standard.key);
      if (running) return running;
      if (pending.size >= 16) throw new PdfError("服务器正在处理其他规范，请稍后重试。", 503);
      const job = prepare(standard);
      pending.set(standard.key, job);
      try { return await job; }
      finally { if (pending.get(standard.key) === job) pending.delete(standard.key); }
    }
  };
}
