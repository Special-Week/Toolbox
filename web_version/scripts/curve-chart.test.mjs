import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { parse, compileScript } from "@vue/compiler-sfc";
import ts from "typescript";
import * as Vue from "vue";

const options = { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext } };
const format = ts.transpileModule(readFileSync(new URL("../src/format.ts", import.meta.url), "utf8"), options).outputText;
const formatUrl = "data:text/javascript;base64," + Buffer.from(format).toString("base64");
const filename = new URL("../src/components/CurveChart.vue", import.meta.url);
const { descriptor } = parse(readFileSync(filename, "utf8"), { filename: filename.pathname });
const script = compileScript(descriptor, { id: "curve-test", genDefaultAs: "CurveComponent" });
let js = ts.transpileModule(script.content, options).outputText;
js = js.replace(/from ["']\.\.\/format["']/g, "from " + JSON.stringify(formatUrl));
js = js.replace(/from ["']vue["']/g, "from " + JSON.stringify(import.meta.resolve("vue")));
const component = (await import("data:text/javascript;base64," + Buffer.from(js + "\nCurveComponent.render = () => null; export default CurveComponent;").toString("base64"))).default;
const renderer = Vue.createRenderer({
  createElement: tag => ({ tag, children: [] }), createText: text => ({ text }), createComment: text => ({ text }),
  setText(node, text) { node.text = text; }, setElementText(node, text) { node.text = text; },
  parentNode: node => node.parent, nextSibling: () => null, patchProp() {},
  insert(node, parent) { node.parent = parent; parent.children.push(node); }, remove() {},
});
globalThis.ResizeObserver = class { observe() {} disconnect() {} };

function mount(evaluate) {
  const app = renderer.createApp({ setup: () => () => Vue.h(component, { curve: [[0, 0], [10, 100]], evaluate }) });
  app.mount({ children: [] });
  return { app, page: app._instance.subTree.component.setupState };
}
function pointer(x, y) {
  return { clientX: x, clientY: y, currentTarget: { getBoundingClientRect: () => ({ left: 0, top: 0, width: 800, height: 300 }) } };
}
const flush = () => new Promise(resolve => setTimeout(resolve, 0));

test("悬停显示模型曲线坐标，不使用鼠标纵坐标或最近采样点", async () => {
  const env = mount(async x => x * x);
  try {
    env.page.inspect(pointer(400, 80)); await flush();
    const first = [...env.page.hover];
    env.page.inspect(pointer(400, 180)); await flush();
    assert.deepEqual(env.page.hover, first);
    assert(Math.abs(first[1] - first[0] * first[0]) < 1e-10);
    assert.notEqual(first[1], 0);
    assert.notEqual(first[1], 100);
  } finally { env.app.unmount(); }
});

test("快速移动只保留最新读数，离开后旧请求不能恢复提示", async () => {
  const waiting = [];
  const env = mount(x => new Promise(resolve => waiting.push({ x, resolve })));
  try {
    env.page.inspect(pointer(350, 100));
    env.page.inspect(pointer(400, 100));
    env.page.inspect(pointer(450, 100));
    assert.equal(waiting.length, 1);
    waiting[0].resolve(-1); await flush();
    assert.equal(waiting.length, 2);
    assert.equal(env.page.hover, null);
    env.page.clearHover();
    waiting[1].resolve(123); await flush();
    assert.equal(env.page.hover, null);
  } finally { env.app.unmount(); }
});
