const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const esbuild = require('esbuild');
const { parse, compileScript } = require('@vue/compiler-sfc');
const vue = require('vue');

test('budget and quick uncertainty cards edit, add and remove without losing settings', async () => {
  class Element {
    constructor(type, text = '') { this.type = type; this.text = text; this.props = {}; this.children = []; this.parent = null; this.style = {}; this.clientWidth = 960; }
    get offsetHeight() { return content(this).includes('A 类评定数据') ? 650 : 200; }
    addEventListener() {} removeEventListener() {} setAttribute() {} removeAttribute() {} getAttribute() { return null; }
  }
  function content(node) { return node.text + node.children.map(content).join(''); }
  global.HTMLElement = Element;
  global.document = { activeElement: null };
  global.window = { listeners: {}, matchMedia: () => ({ matches: false }),
    addEventListener(name, fn) { this.listeners[name] = fn; }, removeEventListener(name) { delete this.listeners[name]; } };
  global.ResizeObserver = class { observe() {} unobserve() {} disconnect() {} };
  let queued = [];
  global.requestAnimationFrame = fn => { queued.push(fn); return queued.length; };
  global.cancelAnimationFrame = () => {};
  const build = await esbuild.build({ entryPoints: [path.resolve('src/components/QuickUncertainty.vue')], bundle: true,
    write: false, platform: 'node', format: 'cjs', external: ['vue'],
    plugins: [{ name: 'vue-sfc', setup(build) { build.onLoad({ filter: /\.vue$/ }, args => {
      const descriptor = parse(fs.readFileSync(args.path, 'utf8'), { filename: args.path }).descriptor;
      return { contents: compileScript(descriptor, { id: args.path, inlineTemplate: true }).content,
        loader: 'ts', resolveDir: path.dirname(args.path) };
    }); } }],
  });
  const compiled = { exports: {} };
  new Function('require', 'module', 'exports', build.outputFiles[0].text)(require, compiled, compiled.exports);
  const Component = compiled.exports.default;
  const schemaBuild = await esbuild.build({ entryPoints: ['src/schema.ts'], bundle: true, write: false, platform: 'node', format: 'cjs' });
  const schemaModule = { exports: {} };
  new Function('require', 'module', 'exports', schemaBuild.outputFiles[0].text)(require, schemaModule, schemaModule.exports);
  const { initialize, schema } = schemaModule.exports;
  const renderer = vue.createRenderer({
    createElement: type => new Element(type), createText: text => new Element('#text', text), createComment: text => new Element('#comment', text),
    setText: (el, text) => { el.text = text; }, setElementText: (el, text) => { el.text = text; el.children = []; },
    parentNode: el => el.parent, nextSibling: el => el.parent?.children[el.parent.children.indexOf(el) + 1] ?? null,
    insert(el, parent, anchor) { if (el.parent) { const siblings = el.parent.children; const old = siblings.indexOf(el); if (old >= 0) siblings.splice(old, 1); } el.parent = parent; const i = anchor ? parent.children.indexOf(anchor) : -1; if (i < 0) parent.children.push(el); else parent.children.splice(i, 0, el); },
    remove(el) { if (el.parent) { const i = el.parent.children.indexOf(el); if (i >= 0) el.parent.children.splice(i, 1); } el.parent = null; },
    patchProp(el, key, _, value) { el.props[key] = value; if (key === 'value') el.value = value; },
  });
  const state = vue.reactive({ mode: '1', aValues: ['1', '2', '3'] });
  const tool = { kind: 'uncertainty' }, catalog = {};
  initialize(tool, state, catalog);
  const root = new Element('root');
  const app = renderer.createApp({ render() { return vue.h(Component, { state, fields: schema(tool, state, catalog)[0].fields,
    catalog, details: [], onChange: (key, value) => { state[key] = value; initialize(tool, state, catalog); } }); } });
  app.mount(root);
  async function flush() { await vue.nextTick(); while (queued.length) { const work = queued; queued = []; for (const fn of work) fn(); await vue.nextTick(); } }
  function all(node) { return [node, ...node.children.flatMap(all)]; }
  function button(text) { const found = all(root).find(node => node.type === 'button' && content(node).includes(text)); assert.ok(found, text); return found; }
  await flush();
  assert.ok(content(root).includes('A 类评定数据'));
  assert.equal(state.bComponents.length, 2);
  const firstCard = all(root).find(node => node.props.class === 'budget-root' && content(node).includes('B 类分量 1'));
  assert.ok(firstCard);
  const digitsLabel = all(firstCard).find(node => node.type === 'label' && content(node).includes('有效数字'));
  const digitsInput = all(digitsLabel).find(node => node.type === 'input');
  digitsInput.props.onInput({ target: { value: '3' } });
  await flush();
  assert.equal(state.bComponents[0].digits, '3');
  button('添加分量').props.onClick();
  await flush();
  assert.equal(state.bComponents.length, 3);
  assert.equal(state.bComponents[0].digits, '3');
  button('删除此分量').props.onClick();
  await flush();
  assert.equal(state.bComponents.length, 2);
  assert.equal(state.bComponents[0].name, 'B 类分量 2');
  const containers = all(root).filter(node => node.props.class === 'masonry-cards');
  assert.equal(containers.length, 1);
  assert.ok(Number.parseFloat(containers[0].style.height) > 0);
  const cards = all(containers[0]).filter(node => node.props.class === 'masonry-card');
  assert.deepEqual(cards.map(card => card.style.left), ['0px', '486px', '486px']);
  assert.ok(Number.parseFloat(cards[2].style.top) >= 200);
  containers[0].clientWidth = 620;
  window.listeners.resize();
  await flush();
  assert.deepEqual(cards.map(card => card.style.left), ['0px', '0px', '0px']);
  assert.ok(Number.parseFloat(cards[1].style.top) >= 650);
  containers[0].clientWidth = 960;
  window.listeners.resize();
  await flush();
  assert.deepEqual(cards.map(card => card.style.left), ['0px', '486px', '486px']);
  app.unmount();

  const fieldBuild = await esbuild.build({ entryPoints: [path.resolve('src/components/FieldEditor.vue')], bundle: true,
    write: false, platform: 'node', format: 'cjs', external: ['vue'],
    plugins: [{ name: 'vue-sfc', setup(build) { build.onLoad({ filter: /\.vue$/ }, args => {
      const descriptor = parse(fs.readFileSync(args.path, 'utf8'), { filename: args.path }).descriptor;
      return { contents: compileScript(descriptor, { id: args.path, inlineTemplate: true }).content,
        loader: 'ts', resolveDir: path.dirname(args.path) };
    }); } }],
  });
  const fieldModule = { exports: {} };
  new Function('require', 'module', 'exports', fieldBuild.outputFiles[0].text)(require, fieldModule, fieldModule.exports);
  const budgetState = vue.reactive({ mode: '0' });
  initialize(tool, budgetState, catalog);
  const budgetRoot = new Element('root');
  const budgetApp = renderer.createApp({ render() {
    const field = schema(tool, budgetState, catalog)[0].fields.find(item => item.key === 'components');
    return vue.h(fieldModule.exports.default, { field, modelValue: budgetState.components, catalog,
      'onUpdate:modelValue': value => { budgetState.components = value; } });
  } });
  budgetApp.mount(budgetRoot);
  await flush();
  function budgetNodes() { return all(budgetRoot); }
  const standardCard = budgetNodes().find(node => node.props.class === 'budget-root' && content(node).includes('标准器'));
  const standardDigits = all(standardCard).find(node => node.type === 'label' && content(node).includes('有效数字'));
  all(standardDigits).find(node => node.type === 'input').props.onInput({ target: { value: '4' } });
  await flush();
  assert.equal(budgetState.components[1].digits, '4');
  budgetNodes().find(node => node.type === 'button' && content(node).includes('添加分量')).props.onClick();
  await flush();
  assert.equal(budgetState.components.length, 3);
  budgetNodes().find(node => node.type === 'button' && content(node).includes('删除此分量')).props.onClick();
  await flush();
  assert.equal(budgetState.components.length, 2);
  assert.equal(budgetState.components[0].name, '标准器');
  assert.equal(budgetState.components[0].digits, '4');
  assert.ok(Number.parseFloat(budgetNodes().find(node => node.props.class === 'masonry-cards').style.height) > 0);
  budgetApp.unmount();
});
