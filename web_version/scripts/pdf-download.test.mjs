import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { parse, compileScript } from "@vue/compiler-sfc";
import ts from "typescript";
import * as Vue from "vue";
import { pdf } from "../server/test-helpers.mjs";

const filename = new URL("../src/components/PdfDownload.vue", import.meta.url);
const source = readFileSync(filename, "utf8");
const catalogSource = readFileSync(new URL("../src/standard-catalog.ts", import.meta.url), "utf8")
  .replace(/import catalog from ["'][^"']+["'];?/, "const catalog = " + readFileSync(new URL("../../data/standards/catalog.json", import.meta.url), "utf8") + ";");
const catalogJs = ts.transpileModule(catalogSource, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext } }).outputText;
const catalogModuleUrl = "data:text/javascript;base64," + Buffer.from(catalogJs).toString("base64");
const catalogModule = await import(catalogModuleUrl);
const { descriptor } = parse(source, { filename: filename.pathname });
const script = compileScript(descriptor, { id: "pdf-test", genDefaultAs: "PdfComponent" });
let js = ts.transpileModule(script.content, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext } }).outputText;
js = js.replace(/import \{ download \} from ["']\.\.\/format["'];?/, "const download = (...args) => globalThis.pdfDownloads.push(args);");
js = js.replace(/from ["']\.\.\/standard-catalog["']/g, "from " + JSON.stringify(catalogModuleUrl));
js = js.replace(/from ["']vue["']/g, "from " + JSON.stringify(import.meta.resolve("vue")));
const component = (await import("data:text/javascript;base64," + Buffer.from(js + "\nPdfComponent.render = () => null; export default PdfComponent;").toString("base64"))).default;
const renderer = Vue.createRenderer({
  createElement: tag => ({ tag, children: [] }), createText: text => ({ text }), createComment: text => ({ text }),
  setText(node, text) { node.text = text; }, setElementText(node, text) { node.text = text; },
  parentNode: node => node.parent, nextSibling: () => null, patchProp() {},
  insert(node, parent) { node.parent = parent; parent.children.push(node); }, remove() {}
});
function dialog() { return { open: false, opened: 0, showModal() { this.open = true; this.opened++; }, close() { this.open = false; } }; }
const link = "https://jjg.spc.org.cn/resmea/standard/detail.html?standno=JJF+1059.1-2012";
const result = { code: "JJF 1059.1-2012", name: "测量不确定度评定与表示", pages: 63, filename: "JJF 1059.1-2012 测量不确定度评定与表示.pdf", cache: "HIT" };
function response(cache = "HIT") {
  return new Response("%PDF-1.4\nfixture", { headers: { "content-type": "application/pdf", ...Object.fromEntries(Object.entries({ ...result, cache }).map(([key, value]) => ["x-pdf-" + key, encodeURIComponent(value)])) } });
}
async function mount(fetchImpl = async () => response()) {
  const calls = [], oldFetch = globalThis.fetch;
  globalThis.pdfDownloads = [];
  globalThis.window = {};
  globalThis.document = { activeElement: { focus() {} } };
  globalThis.fetch = (...args) => { calls.push(args); return fetchImpl(...args); };
  const active = Vue.ref(true);
  const app = renderer.createApp({ setup: () => () => Vue.h(component, { active: active.value }) });
  app.mount({ children: [] });
  const page = app._instance.subTree.component.setupState;
  page.warningDialog = dialog(); page.successDialog = dialog();
  await Vue.nextTick();
  return { page, app, active, calls, cleanup() { app.unmount(); globalThis.fetch = oldFetch; } };
}

test("按名称和编号筛选、分页，选择结果后确认才发起下载", async () => {
  const env = await mount();
  try {
    assert.equal(catalogModule.standards.length, 2882); assert.equal(env.page.matches.length, 0);
    env.page.query = "压力 表"; await Vue.nextTick();
    assert(env.page.matches.length > 0); assert(env.page.matches.every(item => item.name.includes("压力表")));
    env.page.query = "JJF"; await Vue.nextTick();
    assert.equal(env.page.visibleStandards.length, 10); assert(env.page.pageCount > 1);
    env.page.page = 1; assert.notEqual(env.page.visibleStandards[0].code, env.page.matches[0].code);
    env.page.query = " jjf1059.1－2012 "; await Vue.nextTick();
    assert.equal(env.page.page, 0); assert.equal(env.page.matches.length, 1);
    assert.equal(env.page.matches[0].name, result.name);
    env.page.chooseStandard(env.page.matches[0]); await Vue.nextTick();
    assert(env.page.warningOpen); assert.equal(env.calls.length, 0); assert.equal(env.page.url, link);
    assert.equal(env.page.selectedStandard.code, result.code);
    env.page.closeWarning(); assert.equal(env.calls.length, 0);
    env.page.query = "no-such-standard-xyz"; await Vue.nextTick(); assert.equal(env.page.matches.length, 0);
    env.page.reset(); assert.equal(env.page.query, ""); assert.equal(env.page.matches.length, 0);
  } finally { env.cleanup(); }
});

test("无效链接、警告取消不发请求，输入为空且无预设地址", async () => {
  const env = await mount();
  try {
    assert.equal(env.page.url, ""); assert(!descriptor.template.content.includes("placeholder="));
    for (const url of ["", "https://example.com", link + "&standno=other", link + "%0a"]) {
      env.page.url = url; await env.page.requestDownload(); assert(env.page.error); assert(!env.page.warningOpen);
    }
    env.page.url = link; await env.page.requestDownload(); assert(env.page.warningOpen);
    env.page.closeWarning(); assert(!env.page.warningOpen); assert.equal(env.calls.length, 0);
    assert(!source.includes("postMessage")); assert(!source.includes("javascript:"));
  } finally { env.cleanup(); }
});

test("确认后请求同源服务器，收到 PDF 后交给浏览器下载并显示完成提示", async () => {
  for (const cache of ["HIT", "MISS"]) {
    const env = await mount(async () => response(cache));
    try {
      env.page.url = link; await env.page.requestDownload(); await env.page.start(); await Vue.nextTick();
      assert.equal(env.calls.length, 1);
      const request = new URL(env.calls[0][0], "http://localhost");
      assert.equal(request.pathname, "/api/pdf"); assert.equal(request.searchParams.get("url"), link);
      assert.equal(env.page.result.cache, cache); assert.equal(env.page.result.pages, 63);
      assert.equal(pdfDownloads.length, 1); assert.equal(pdfDownloads[0][1], result.filename);
      assert.equal(await pdfDownloads[0][0].text(), "%PDF-1.4\nfixture");
      assert(!env.page.busy); assert.equal(env.page.successDialog.opened, 1);
      assert.equal(env.page.successTitle, "PDF 下载已发起"); assert(env.page.message.includes("下载列表"));
      env.page.closeSuccess(); await env.page.saveAgain();
      assert.equal(pdfDownloads.length, 2); assert.equal(env.calls.length, 1);
      env.page.reset(); assert.equal(env.page.url, ""); assert.equal(env.page.result, null);
    } finally { env.cleanup(); }
  }
});

test("前端可接收并保存超过原 50 MB 上限的 PDF", async () => {
  const data = pdf(2, 51 * 1024 * 1024);
  const env = await mount(async () => new Response(data, { headers: response().headers }));
  try {
    env.page.url = link; await env.page.start();
    assert.equal(env.page.error, ""); assert.equal(pdfDownloads.length, 1);
    assert.equal(pdfDownloads[0][0].size, data.length); assert(env.page.successPending);
  } finally { env.cleanup(); }
});

test("服务器错误、静态托管返回 HTML、无效 PDF 均不显示成功", async () => {
  for (const reply of [() => new Response(JSON.stringify({ error: "上游请求过于频繁" }), { status: 429 }), () => new Response("<html>static host</html>", { headers: { "content-type": "text/html" } }), () => new Response("<html>not pdf</html>", { headers: response().headers }), () => { throw new TypeError("offline"); }]) {
    const env = await mount(reply);
    try {
      env.page.url = link; await env.page.requestDownload(); await env.page.start();
      assert(env.page.error); assert(!env.page.busy); assert(!env.page.successPending);
      assert.equal(pdfDownloads.length, 0); assert.equal(env.page.successDialog.opened, 0);
    } finally { env.cleanup(); }
  }
});

test("静态服务器的 404 提示使用完整服务启动入口", async () => {
  const env = await mount(async () => new Response("<html>Not Found</html>", { status: 404 }));
  try {
    env.page.url = link; await env.page.start();
    assert(env.page.error.includes("python serve.py")); assert(env.page.error.includes("/api/pdf"));
    assert.equal(pdfDownloads.length, 0);
  } finally { env.cleanup(); }
});

test("再次保存使用浏览器内已有文件，仅写入完成才提示保存成功", async () => {
  const env = await mount();
  try {
    env.page.url = link; await env.page.start(); env.page.closeSuccess();
    const events = [];
    window.showSaveFilePicker = async () => ({ name: "chosen.pdf", createWritable: async () => ({
      async write(blob) { events.push("write"); assert.equal(await blob.slice(0, 5).text(), "%PDF-"); },
      async close() { assert(!env.page.successPending); events.push("close"); }, async abort() { events.push("abort"); }
    }) });
    await env.page.saveAgain(); await Vue.nextTick();
    assert.deepEqual(events, ["write", "close"]); assert.equal(env.page.successTitle, "保存成功");
    assert(env.page.message.includes("chosen.pdf")); assert.equal(env.calls.length, 1);
    env.page.closeSuccess();
    window.showSaveFilePicker = async () => { throw new DOMException("cancelled", "AbortError"); };
    await env.page.saveAgain(); assert(!env.page.successPending); assert(env.page.message.includes("取消"));
    assert.equal(env.calls.length, 1);
  } finally { env.cleanup(); }
});

test("切换工具不打断下载，回到页面显示完成弹窗，卸载时终止浏览器请求", async () => {
  let resolveRequest;
  const env = await mount(() => new Promise(resolve => { resolveRequest = resolve; }));
  try {
    env.page.url = link; const work = env.page.start();
    assert(env.page.busy); env.active.value = false; await Vue.nextTick();
    resolveRequest(response()); await work; await Vue.nextTick();
    assert(env.page.successPending); assert.equal(env.page.successDialog.opened, 0);
    env.active.value = true; await Vue.nextTick(); await Vue.nextTick();
    assert.equal(env.page.successDialog.opened, 1);
  } finally { env.cleanup(); }
  const pending = await mount((url, options) => new Promise((resolve, reject) => options.signal.addEventListener("abort", () => reject(new DOMException("abort", "AbortError")))));
  pending.page.url = link; const work = pending.page.start();
  pending.cleanup(); await work;
  assert(pending.calls[0][1].signal.aborted);
});
