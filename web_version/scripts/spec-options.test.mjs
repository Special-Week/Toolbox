import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { parse, compileScript } from "@vue/compiler-sfc";
import ts from "typescript";
import * as Vue from "vue";

const file = new URL("../src/components/SpecOptions.vue", import.meta.url);
const { descriptor } = parse(readFileSync(file, "utf8"), { filename: file.pathname });
const script = compileScript(descriptor, { id: "spec-test", inlineTemplate: true, genDefaultAs: "SpecComponent" });
let source = ts.transpileModule(script.content, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext } }).outputText;
source = source.replace(/from ["']vue["']/g, "from " + JSON.stringify(import.meta.resolve("vue")));
const component = (await import("data:text/javascript;base64," + Buffer.from(source + "\nexport default SpecComponent;").toString("base64"))).default;

globalThis.Node = class {};
const listeners = new Map();
globalThis.document = {
  activeElement: null,
  addEventListener(name, callback) { listeners.set(name, callback); },
  removeEventListener(name) { listeners.delete(name); },
};
function descendants(node) {
  return [node, ...(node.children ?? []).flatMap(descendants)];
}
class Element extends Node {
  constructor(tag) { super(); this.tag = tag; this.children = []; this.props = {}; this.open = false; }
  contains(node) { return descendants(this).includes(node); }
  querySelector(selector) { return selector === "summary" ? descendants(this).find(node => node.tag === "summary") : null; }
  querySelectorAll(selector) { return selector === '[role="option"]' ? descendants(this).filter(node => node.props?.role === "option") : []; }
  focus() { document.activeElement = this; }
  scrollIntoView() {}
}
const renderer = Vue.createRenderer({
  createElement: tag => Vue.markRaw(new Element(tag)), createText: text => ({ text }), createComment: text => ({ text }),
  setText(node, text) { node.text = text; }, setElementText(node, text) { node.text = text; },
  parentNode: node => node.parent, nextSibling: () => null,
  patchProp(node, key, previous, value) { node.props[key] = value; },
  insert(node, parent) { node.parent = parent; parent.children.push(node); }, remove() {},
});

test("三列规格可用键盘选择，Escape 和外部点击关闭菜单", async () => {
  let selected;
  const options = [
    { value: "No.0", label: "No.0 (0.06 inch / 1.524 mm)", columns: ["No.0", "0.06 inch", "1.524 mm"] },
    { value: "1/4", label: "1/4 (0.25 inch / 6.35 mm)", columns: ["1/4", "0.25 inch", "6.35 mm"] },
  ];
  const root = new Element("root");
  const app = renderer.createApp({ render: () => Vue.h(component, { options, selected: "1/4", onSelect: value => selected = value }) });
  app.mount(root);
  try {
    const menu = descendants(root).find(node => node.tag === "details");
    const buttons = menu.querySelectorAll('[role="option"]');
    assert.equal(buttons.length, 2);
    assert.equal(buttons[1].props["aria-selected"], true);
    assert.deepEqual(buttons[0].children.filter(node => node.tag === "span").map(node => node.text), options[0].columns);
    menu.open = true;
    menu.props.onKeydown({ key: "ArrowDown", preventDefault() {} });
    await Vue.nextTick();
    assert.equal(document.activeElement, buttons[0]);
    buttons[0].props.onClick();
    assert.equal(selected, "No.0");
    assert.equal(menu.open, false);
    assert.equal(document.activeElement.tag, "summary");
    menu.open = true;
    menu.props.onKeydown({ key: "Escape", preventDefault() {} });
    assert.equal(menu.open, false);
    menu.open = true;
    listeners.get("pointerdown")({ target: new Element("outside") });
    assert.equal(menu.open, false);
  } finally { app.unmount(); }
});
