import test from "node:test";
import assert from "node:assert/strict";
import { readFile, readdir, writeFile, unlink } from "node:fs/promises";
import path from "node:path";
import { createPdfCache, inspectPdf, parseDetailLink, defaultFilename } from "./pdf-cache.mjs";
import { LINK, NAME, pdf, upstream, temporary } from "./test-helpers.mjs";

test("编号规范化作为缓存键，附加参数和空格编码不影响匹配", () => {
  const standard = parseDetailLink(LINK);
  assert.equal(standard.code, "JJF 1059.1-2012");
  assert.equal(parseDetailLink(LINK.replace("JJF+", "jjf%20") + "&pageindex=2&rows=10").key, standard.key);
  assert.notEqual(parseDetailLink(LINK.replace("2012", "2024")).key, standard.key);
  assert.match(standard.key, /^[a-f0-9]{64}$/);
  assert.equal(new URL(standard.url).searchParams.size, 1);
});

test("拒绝外部站点、非详情页、重复编号和控制字符，不发起任意网络请求", async t => {
  const { fetchImpl, calls } = upstream();
  const cache = createPdfCache({ cacheDirectory: await temporary(t), fetchImpl });
  for (const input of ["", LINK.replace("https:", "http:"), LINK.replace("jjg.spc.org.cn", "127.0.0.1"), LINK.replace("jjg.spc.org.cn", "jjg.spc.org.cn.evil.test"), LINK.replace("https://", "https://user:pass@"), LINK.replace("detail.html", "other.html"), LINK + "&standno=JJF+1", LINK + "%0a", LINK.replace("JJF+1059.1-2012", "%E4%B8%AD%E6%96%87"), LINK + "\n"]) {
    await assert.rejects(cache.get(input), error => error.status === 400, input);
  }
  assert.equal(calls.length, 0);
});

test("首次下载完整保存，等价链接及重启后的请求只读本地 PDF", async t => {
  const directory = await temporary(t), data = pdf(3);
  const { fetchImpl, calls } = upstream(data);
  const cache = createPdfCache({ cacheDirectory: directory, fetchImpl });
  const first = await cache.get(LINK);
  assert.equal(first.cache, "MISS"); assert.equal(first.pages, 3); assert.equal(first.name, NAME);
  assert.deepEqual(first.data, data); assert.equal(calls.length, 3);
  assert.equal(calls[1].method, "POST");
  assert.equal(calls[1].body.get("a100"), first.code); assert.equal(calls[1].body.get("ismobile"), "1");
  assert.equal(calls[2].url.searchParams.get("token"), "dG9rZW4rLw==");
  assert.equal(calls[2].headers.myfoxit, "ZW5jLys=");
  assert.equal(calls[1].headers.Cookie, "JSESSIONID=test-session");
  assert.equal(calls[2].headers.Cookie, "JSESSIONID=test-session");
  assert(calls.every(call => call.redirect === "error"));
  const key = parseDetailLink(LINK).key;
  assert.deepEqual((await readdir(directory)).sort(), [key + ".json", key + ".pdf"]);
  assert.deepEqual(await readFile(path.join(directory, key + ".pdf")), data);
  const meta = JSON.parse(await readFile(path.join(directory, key + ".json"), "utf8"));
  assert.equal(meta.code, first.code); assert.equal(meta.pages, 3); assert.equal(meta.name, NAME);
  assert.equal((await cache.get(LINK + "&pageindex=2")).cache, "HIT"); assert.equal(calls.length, 3);
  const restarted = createPdfCache({ cacheDirectory: directory, fetchImpl() { assert.fail("缓存命中不应访问网络"); } });
  const second = await restarted.get(LINK.replace("+", "%20"));
  assert.equal(second.cache, "HIT"); assert.deepEqual(second.data, data); assert.equal(second.name, NAME);
});

test("并发请求同一编号只获取一次，返回的原始 PDF 字节一致", async t => {
  const data = pdf(), { fetchImpl, calls } = upstream(data);
  const cache = createPdfCache({ cacheDirectory: await temporary(t), fetchImpl });
  const results = await Promise.all(Array.from({ length: 8 }, (_, i) => cache.get(LINK + "&pageindex=" + i)));
  assert.equal(calls.length, 3);
  assert(results.every(result => result.cache === "MISS" && result.data.equals(data)));
});

test("损坏的缓存重新下载；缺少或损坏的元数据不影响使用有效 PDF", async t => {
  const directory = await temporary(t), key = parseDetailLink(LINK).key;
  const { fetchImpl, calls } = upstream();
  const cache = createPdfCache({ cacheDirectory: directory, fetchImpl });
  await writeFile(path.join(directory, key + ".pdf"), "%PDF-1.4\nbroken");
  assert.equal((await cache.get(LINK)).cache, "MISS"); assert.equal(calls.length, 3);
  await writeFile(path.join(directory, key + ".json"), "{broken");
  assert.equal((await cache.get(LINK)).cache, "HIT");
  await writeFile(path.join(directory, key + ".json"), JSON.stringify({ code: "JJF 1059.1-2012", name: "\ud800" }));
  assert.equal((await cache.get(LINK)).name, "");
  await unlink(path.join(directory, key + ".json"));
  const result = await cache.get(LINK);
  assert.equal(result.cache, "HIT"); assert.equal(result.name, ""); assert.equal(calls.length, 3);
});

test("HTML、损坏 PDF、超限响应和 HTTP 429 不进入缓存，失败后可重试", async t => {
  const directory = await temporary(t);
  for (const kind of ["html", "broken", "oversize", "429", "redirect"]) {
    const normal = upstream();
    const cache = createPdfCache({ cacheDirectory: directory, fetchImpl: async (url, options) => {
      if (kind === "429") return new Response("limit", { status: 429 });
      if (kind === "redirect") return new Response("", { status: 302, headers: { location: "http://127.0.0.1/private" } });
      if (new URL(url).pathname.endsWith("onlinereading")) {
        if (kind === "oversize") return new Response(pdf(), { headers: { "content-length": String(101 * 1024 * 1024) } });
        return new Response(kind === "html" ? "<html>Error</html>" : "%PDF-1.4\ninvalid");
      }
      return normal.fetchImpl(url, options);
    } });
    await assert.rejects(cache.get(LINK), error => error.status === (kind === "429" ? 429 : 502));
    assert.deepEqual(await readdir(directory), []);
  }
  let fail = true;
  const normal = upstream();
  const cache = createPdfCache({ cacheDirectory: directory, fetchImpl: (...args) => fail ? Promise.reject(new Error("offline")) : normal.fetchImpl(...args) });
  await assert.rejects(cache.get(LINK));
  fail = false;
  assert.equal((await cache.get(LINK)).cache, "MISS");
});

test("超过原 50 MB 上限的 PDF 可下载、解析及缓存复用", async t => {
  const data = pdf(2, 51 * 1024 * 1024), remote = upstream(data);
  const cache = createPdfCache({ cacheDirectory: await temporary(t), fetchImpl: remote.fetchImpl });
  const first = await cache.get(LINK);
  assert.equal(first.cache, "MISS"); assert.equal(first.pages, 2); assert.deepEqual(first.data, data);
  const second = await cache.get(LINK);
  assert.equal(second.cache, "HIT"); assert.equal(second.pages, 2); assert.deepEqual(second.data, data);
  assert.equal(remote.calls.length, 3);
});

test("缓存写入失败时不宣称下载成功，不留下临时文件", async t => {
  const directory = await temporary(t), blocked = path.join(directory, "file-instead-of-directory");
  await writeFile(blocked, "keep");
  const cache = createPdfCache({ cacheDirectory: blocked, fetchImpl: upstream().fetchImpl });
  await assert.rejects(cache.get(LINK));
  assert.deepEqual(await readdir(directory), ["file-instead-of-directory"]);
  assert.equal(await readFile(blocked, "utf8"), "keep");
});

test("真实解析页数并保留原始字节，文件名过滤 Windows 非法字符", async () => {
  const data = pdf(4), copy = Buffer.from(data);
  assert.equal(await inspectPdf(data), 4); assert.deepEqual(data, copy);
  await assert.rejects(inspectPdf(Buffer.from("%PDF-no-objects")));
  assert.equal(defaultFilename("CON", ""), "_CON.pdf");
  assert.equal(defaultFilename("JJF 1/2", "a:b?"), "JJF 1_2 a_b_.pdf");
});
