import assert from "node:assert/strict";
import { setImmediate } from "node:timers/promises";
import { test } from "node:test";
import ts from "typescript";
import { readFileSync } from "node:fs";

async function load(name) {
  const file = new URL(`../src/${name}.ts`, import.meta.url);
  const { outputText } = ts.transpileModule(readFileSync(file, "utf8"), {
    compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  });
  return import("data:text/javascript;base64," + Buffer.from(outputText).toString("base64"));
}

function memoryStorage(disk = new Map()) {
  return {
    get length() { return disk.size; },
    key: (index) => [...disk.keys()][index] ?? null,
    getItem: (key) => disk.get(key) ?? null,
    setItem: (key, value) => disk.set(key, value),
    removeItem: (key) => disk.delete(key),
  };
}

test("压力单位换算重读编辑过的字段，忽略被替代的请求", async () => {
  const { changePressureUnit } = await load("engine");
  const state = { pressureUnit: "1", low: "0", high: "100" };
  const pending = [];
  const convert = (input) => new Promise((resolve) => pending.push({ input, resolve }));
  let live = true;
  const request = changePressureUnit(state, "2", false, () => live, convert);
  state.high = "250";
  pending.shift().resolve(["0", "0.1"]);
  await setImmediate();
  const latest = pending.shift();
  assert.deepEqual(latest.input, { source: 1, target: 2, values: ["0", "250"] });
  latest.resolve(["0", "0.25"]);
  await request;
  assert.deepEqual(state, { pressureUnit: "2", low: "0", high: "0.25" });

  const stale = changePressureUnit(state, "1", false, () => live, convert);
  live = false;
  pending.shift().resolve(["0", "250"]);
  await stale;
  assert.equal(state.pressureUnit, "2");
  assert.equal(state.high, "0.25");
});

test("最近输入兼容旧档，两个标签页分别保存时保留双方的修改", async () => {
  const { INPUT_STORAGE_KEY, readRememberedInputs, saveChangedInputs } = await load("input-history");
  const tools = [{ id: "pressure", kind: "pressure_reference", rememberInputs: true }, { id: "fit", rememberInputs: true }];
  const disk = new Map([[INPUT_STORAGE_KEY, '{"pressure":{"high":"100"},"fit":{"model":"0"}}']]);
  const storage = memoryStorage(disk);
  const tab = () => {
    const versions = new Map();
    const state = readRememberedInputs(tools, storage, versions);
    const edited = new Map();
    return { state, edited, save: () => saveChangedInputs(tools, state, edited, storage, versions) };
  };
  const first = tab(), second = tab();
  first.state.pressure.high = "250";
  first.edited.set("pressure", { fields: new Map([["high", "250"]]) });
  first.save();
  // 初始化旧标签页时补上的示例字段不会写回输入历史。
  second.state.pressure.low = "0";
  second.state.fit.model = "2";
  second.edited.set("fit", { fields: new Map([["model", "2"]]) });
  second.save();
  assert.deepEqual(readRememberedInputs(tools, storage), {
    pressure: { high: "250" },
    fit: { model: "2" },
  });
  second.edited.set("pressure", { fields: new Map([["low", "-1"]]) });
  second.save();
  assert.deepEqual(readRememberedInputs(tools, storage).pressure, { high: "250", low: "-1" });
  first.state.pressure = { high: 1, low: 0 };
  first.edited.set("pressure", { fields: new Map(), reset: true });
  first.save();
  assert.deepEqual(readRememberedInputs(tools, storage), {
    pressure: { high: 1, low: 0 },
    fit: { model: "2" },
  });

  first.state.pressure = { pressureUnit: "1", high: "1000", low: "0" };
  first.edited.set("pressure", { fields: new Map(Object.entries(first.state.pressure)) });
  first.save();
  second.state.pressure = { pressureUnit: "2", high: "1", low: "-0.1" };
  second.edited.set("pressure", { fields: new Map([["low", "-0.1"]]) });
  assert.deepEqual([...second.save()], ["pressure"]);
  assert.deepEqual(readRememberedInputs(tools, storage).pressure, {
    pressureUnit: "1", high: "1000", low: "0",
  });
});

test("压力单位换算和恢复示例拒绝过期的组、普通字段和行编辑", async () => {
  const { INPUT_STORAGE_KEY, readRememberedInputs, recordInputChanges, saveChangedInputs } = await load("input-history");
  const tools = [{ id: "pressure", kind: "pressure_reference", rememberInputs: true }];
  const defaults = { pressureUnit: "2", low: "0", high: "1", refHigh: "1.6", ratio: "3", rows: ["1", "2"] };
  const storage = memoryStorage(new Map([[INPUT_STORAGE_KEY, JSON.stringify({ pressure: defaults })]]));
  const open = () => {
    const versions = new Map();
    const states = readRememberedInputs(tools, storage, versions);
    const pending = new Map();
    return {
      edit(patch) {
        const after = { ...states.pressure, ...patch };
        const edit = pending.get("pressure") ?? { fields: new Map() };
        recordInputChanges(edit, states.pressure, after, versions.get("pressure"));
        states.pressure = after;
        pending.set("pressure", edit);
      },
      reset() { states.pressure = defaults; pending.set("pressure", { fields: new Map(), reset: true }); },
      save: () => saveChangedInputs(tools, states, pending, storage, versions),
      pending,
    };
  };
  const changed = open(), stale = open(), olderConversion = open();
  changed.edit({ high: "2" });
  assert.equal(changed.save().size, 0);
  olderConversion.edit({ pressureUnit: "1", high: "1000", refHigh: "1600" });
  assert.deepEqual([...olderConversion.save()], ["pressure"]);
  changed.edit({ pressureUnit: "1", high: "2000", refHigh: "1600" });
  assert.equal(changed.save().size, 0);
  stale.edit({ refHigh: "3.2", ratio: "4" });
  assert.deepEqual([...stale.save()], ["pressure"]);
  assert.equal(stale.pending.get("pressure").fields.get("refHigh"), "3.2");
  assert.deepEqual(readRememberedInputs(tools, storage).pressure, {
    ...defaults, pressureUnit: "1", high: "2000", refHigh: "1600", ratio: "4",
  });

  const delayed = open(), reset = open();
  delayed.edit({ ratio: "5", high: "2500", rows: ["1", "changed"] });
  reset.reset();
  assert.equal(reset.save().size, 0);
  assert.deepEqual([...delayed.save()], ["pressure"]);
  assert.deepEqual(readRememberedInputs(tools, storage).pressure, defaults);
  assert.ok(delayed.pending.has("pressure"));
});

test("跨标签页合并同表的不同行和列，整表替换后保留新结构", async () => {
  const { INPUT_STORAGE_KEY, readRememberedInputs, recordInputChanges, saveChangedInputs } = await load("input-history");
  const tools = [{ id: "fit", rememberInputs: true }];
  const defaults = [["0", "0.197"], ["1", "0.208"]];
  const storage = memoryStorage(new Map([
    [`${INPUT_STORAGE_KEY}:fit:field:0:points`, JSON.stringify([defaults])],
  ]));
  const tab = () => {
    const versions = new Map();
    const states = readRememberedInputs(tools, storage, versions);
    const pending = new Map();
    return {
      update(points, replace) {
        const edit = pending.get("fit") ?? { fields: new Map() };
        const next = { ...states.fit, points };
        recordInputChanges(edit, states.fit, next, versions.get("fit"), replace);
        states.fit = next;
        pending.set("fit", edit);
      },
      save: () => saveChangedInputs(tools, states, pending, storage, versions),
    };
  };
  const first = tab(), second = tab(), third = tab();
  first.update([["9", "0.197"], ["1", "0.208"]]);
  second.update([["0", "1.31"], ["1", "0.208"]]);
  third.update([["0", "0.197"], ["1", "2.51"]]);
  first.save();
  second.save();
  third.save();
  assert.deepEqual(readRememberedInputs(tools, storage).fit.points, [["9", "1.31"], ["1", "2.51"]]);

  const replacement = tab(), stale = tab();
  stale.update([["9", "9.99"], ["1", "2.51"]]);
  const pasted = [["8", "8"], ["7", "7"]];
  replacement.update(pasted, "points");
  assert.equal(replacement.save().size, 0);
  assert.deepEqual([...stale.save()], ["fit"]);
  assert.deepEqual(readRememberedInputs(tools, storage).fit.points, pasted);
  replacement.update([["8", "8"], ["7", "7.1"]]);
  replacement.save();
  assert.deepEqual(readRememberedInputs(tools, storage).fit.points, [["8", "8"], ["7", "7.1"]]);

  const editing = tab(), concurrent = tab();
  editing.update([["8", "8"], ["7", "9"]]);
  const newTable = [["4", "4"], ["3", "3"]];
  const setItem = storage.setItem;
  let interrupted = false;
  storage.setItem = (key, value) => {
    if (!interrupted && key.includes(":cell:")) {
      interrupted = true;
      concurrent.update(newTable, "points");
      concurrent.save();
    }
    setItem(key, value);
  };
  assert.deepEqual([...editing.save()], ["fit"]);
  assert.deepEqual(readRememberedInputs(tools, storage).fit.points, newTable);
});

test("跨标签页追加和删除指定行，保留其他窗口已保存的单元格", async () => {
  const { readRememberedInputs, recordInputChanges, saveChangedInputs } = await load("input-history");
  const tools = [{ id: "fit", rememberInputs: true }];
  const example = [["0", "0.197"], ["1", "0.208"]];
  const storage = memoryStorage();
  const open = () => {
    const versions = new Map();
    const states = readRememberedInputs(tools, storage, versions);
    states.fit ??= { points: example };
    const pending = new Map();
    return {
      update(next) {
        const edit = pending.get("fit") ?? { fields: new Map() };
        recordInputChanges(edit, states.fit, { points: next }, versions.get("fit"));
        states.fit = { points: next };
        pending.set("fit", edit);
      },
      save: () => saveChangedInputs(tools, states, pending, storage, versions),
    };
  };
  const first = open(), second = open(), third = open(), remove = open();
  first.update([example[0], ["1", "2.51"]]);
  first.save();
  second.update([...example, ["2", "second"]]);
  third.update([...example, ["3", "third"]]);
  second.save();
  third.save();
  assert.deepEqual(readRememberedInputs(tools, storage).fit.points.slice(0, 2), [example[0], ["1", "2.51"]]);

  third.update([...example, ["3", "edited third"]]);
  third.save();
  remove.update([example[0]]);
  remove.save();
  const rows = readRememberedInputs(tools, storage).fit.points;
  assert.deepEqual(rows[0], example[0]);
  assert.deepEqual(new Map(rows.slice(1)), new Map([["2", "second"], ["3", "edited third"]]));
  first.update([example[0], ["1", "edited removed row"]]);
  assert.deepEqual([...first.save()], ["fit"]);
  assert.deepEqual(readRememberedInputs(tools, storage).fit.points, rows);

  const editing = open(), deleting = open();
  const tail = rows.at(-1);
  editing.update([...rows.slice(0, -1), [tail[0], "changed during deletion"]]);
  deleting.update(rows.slice(0, -1));
  const original = storage.setItem;
  let raced = false;
  storage.setItem = (key, value) => {
    if (!raced && key.includes(":cell:")) {
      raced = true;
      deleting.save();
    }
    original(key, value);
  };
  assert.deepEqual([...editing.save()], ["fit"]);
  assert.deepEqual(readRememberedInputs(tools, storage).fit.points, rows.slice(0, -1));
});

test("跨标签页删除非末行和增删子分量，按行身份保留已保存的编辑", async () => {
  const { readRememberedInputs, recordInputChanges, saveChangedInputs } = await load("input-history");
  const tools = [{ id: "budget", rememberInputs: true }];
  const sample = { name: "duplicate", children: [] };
  const example = [sample, { ...sample }, { name: "group", children: [sample, { ...sample }] }];
  const storage = memoryStorage();
  const read = () => readRememberedInputs(tools, storage).budget.components;
  const tab = () => {
    const versions = new Map();
    const states = readRememberedInputs(tools, storage, versions);
    states.budget ??= { components: example };
    const pending = new Map();
    return {
      update(change) {
        const before = states.budget;
        const after = { components: change(before.components) };
        const edit = pending.get("budget") ?? { fields: new Map() };
        recordInputChanges(edit, before, after, versions.get("budget"));
        states.budget = after;
        pending.set("budget", edit);
      },
      save: () => saveChangedInputs(tools, states, pending, storage, versions),
    };
  };
  const editor = tab(), remover = tab(), appender = tab();
  editor.update(([a, b, group]) => [a, { ...b, name: "kept" }, {
    ...group, children: [group.children[0], { ...group.children[1], name: "edited child" }],
  }]);
  remover.update(rows => rows.slice(1));
  appender.update(([a, b, group]) => [a, b, { ...group, children: [...group.children, { ...sample, name: "new" }] }]);
  for (const current of [editor, remover, appender]) assert.equal(current.save().size, 0);
  assert.deepEqual(read().map(row => row.name), ["kept", "group"]);
  assert.deepEqual(read()[1].children.map(row => row.name), ["duplicate", "edited child", "new"]);

  const deletingChild = tab(), editingChild = tab();
  deletingChild.update(([a, group]) => [a, { ...group, children: group.children.slice(1) }]);
  editingChild.update(([a, group]) => [a, {
    ...group, children: group.children.map((row, i) => i === 1 ? { ...row, name: "edited after deletion" } : row),
  }]);
  assert.equal(deletingChild.save().size, 0);
  assert.equal(editingChild.save().size, 0);
  assert.deepEqual(read()[1].children.map(row => row.name), ["edited after deletion", "new"]);

  const removeGroup = tab(), addToRemovedGroup = tab();
  addToRemovedGroup.update(([a, group]) => [a, {
    ...group, children: [...group.children, { ...sample, name: "pending child" }],
  }]);
  removeGroup.update(rows => rows.slice(0, 1));
  assert.equal(removeGroup.save().size, 0);
  assert.deepEqual([...addToRemovedGroup.save()], ["budget"]);
  assert.deepEqual(read().map(row => row.name), ["kept"]);
});

test("按保存批次校验预算行数上下限以及子组最少行数", async () => {
  const { INPUT_STORAGE_KEY, readRememberedInputs, recordInputChanges, saveChangedInputs } = await load("input-history");
  const tools = [{ id: "budget", kind: "uncertainty", rememberInputs: true }];
  const leaf = name => ({ name, kind: "1", children: [] });
  const group = { name: "group", kind: "2", children: [leaf("first"), leaf("second")] };
  const initial = [leaf("root"), group];
  const storage = memoryStorage(new Map([[INPUT_STORAGE_KEY, JSON.stringify({ budget: { components: initial } })]]));
  const field = { key: "components", type: "budget", min: 1, max: 2 };
  const current = () => readRememberedInputs(tools, storage).budget.components;
  const open = () => {
    const versions = new Map();
    const state = readRememberedInputs(tools, storage, versions);
    const edit = { fields: new Map() };
    const pending = new Map([["budget", edit]]);
    return {
      change(apply) {
        const updated = { components: apply(state.budget.components) };
        recordInputChanges(edit, state.budget, updated, versions.get("budget"), undefined, field);
        state.budget = updated;
      },
      save: () => saveChangedInputs(tools, state, pending, storage, versions),
      edit,
    };
  };
  const removeFirst = open(), removeSecond = open();
  removeFirst.change(rows => rows.slice(1));
  removeSecond.change(rows => rows.slice(0, 1));
  assert.equal(removeFirst.save().size, 0);
  assert.deepEqual([...removeSecond.save()], ["budget"]);
  assert.deepEqual(current(), [group]);
  assert.equal(removeSecond.edit.cells.get("components").rows.size, 1);

  const appendFirst = open(), appendSecond = open();
  appendFirst.change(rows => [...rows, leaf("new A")]);
  appendSecond.change(rows => [...rows, leaf("new B")]);
  assert.equal(appendFirst.save().size, 0);
  assert.deepEqual([...appendSecond.save()], ["budget"]);
  assert.deepEqual(current().map(row => row.name), ["group", "new A"]);

  const removeChildFirst = open(), removeChildSecond = open();
  removeChildFirst.change(([row, other]) => [{ ...row, children: row.children.slice(1) }, other]);
  removeChildSecond.change(([row, other]) => [{ ...row, children: row.children.slice(0, 1) }, other]);
  assert.equal(removeChildFirst.save().size, 0);
  assert.deepEqual([...removeChildSecond.save()], ["budget"]);
  assert.deepEqual(current()[0].children, [leaf("second")]);
});

test("极差法切换和测量值变更一起校验，保留旧版批次中已保存的数据", async () => {
  const { INPUT_STORAGE_KEY, readRememberedInputs, recordInputChanges, saveChangedInputs } = await load("input-history");
  const tools = [{ id: "budget", kind: "uncertainty", rememberInputs: true }];
  const samples = Array.from({ length: 9 }, (_, i) => String(i + 1));
  for (const methodFirst of [true, false]) {
    const storage = memoryStorage(new Map([[INPUT_STORAGE_KEY, JSON.stringify({
      budget: { components: [{ kind: "0", method: "0", values: samples }] },
    })]]));
    const current = () => readRememberedInputs(tools, storage).budget.components[0];
    const open = () => {
      const versions = new Map();
      const states = readRememberedInputs(tools, storage, versions);
      const pending = new Map();
      return {
        update(change) {
          const before = states.budget;
          const after = { components: [change(before.components[0])] };
          const edit = { fields: new Map() };
          recordInputChanges(edit, before, after, versions.get("budget"));
          states.budget = after;
          pending.set("budget", edit);
        },
        save: () => saveChangedInputs(tools, states, pending, storage, versions),
      };
    };
    const switchMethod = row => ({ ...row, method: "1", values: row.values.slice(0, 9) });
    const switching = open(), appending = open();
    switching.update(switchMethod);
    appending.update(row => ({ ...row, values: [...row.values, "10"] }));
    const first = methodFirst ? switching : appending;
    const second = methodFirst ? appending : switching;
    assert.equal(first.save().size, 0);
    assert.deepEqual([...second.save()], ["budget"]);
    assert.deepEqual(current(), { kind: "0", method: methodFirst ? "1" : "0", values: methodFirst ? samples : [...samples, "10"] });
    if (!methodFirst) {
      const fresh = open();
      fresh.update(switchMethod);
      assert.equal(fresh.save().size, 0);
      assert.deepEqual(current().values, samples);
      assert.equal(current().method, "1");
    }
  }

  const prefix = `${INPUT_STORAGE_KEY}:budget`;
  const legacy = memoryStorage(new Map([
    [INPUT_STORAGE_KEY, JSON.stringify({ budget: { components: [{ kind: "0", method: "0", values: samples.slice(0, 5) }] } })],
    [`${prefix}:row-batch:0:components:0:early`, JSON.stringify(samples.slice(5).concat("10").map((value, i) => ({ path: ["0", "values", `r${i}`], value, limits: [2, 50] })))],
    [`${prefix}:row-batch:0:components:0:late`, JSON.stringify([{ path: ["0", "values", "r4"], limits: [2, 9] }])],
    [`${prefix}:cell:0:components:0:${encodeURIComponent(JSON.stringify(["0", "method"]))}`, '["1"]'],
  ]));
  assert.deepEqual(readRememberedInputs(tools, legacy).budget.components[0], { kind: "0", method: "1", values: samples });
});
