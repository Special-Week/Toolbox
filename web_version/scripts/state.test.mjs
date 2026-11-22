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
  const tools = [{ id: "pressure", rememberInputs: true }, { id: "fit", rememberInputs: true }];
  const disk = new Map([[INPUT_STORAGE_KEY, '{"pressure":{"high":"100"},"fit":{"model":"0"}}']]);
  const storage = { getItem: (key) => disk.get(key) ?? null, setItem: (key, value) => disk.set(key, value) };
  const tab = () => {
    const state = readRememberedInputs(tools, storage);
    const saved = new Map(Object.entries(state).map(([id, value]) => [id, JSON.stringify(value)]));
    return { state, save: () => saveChangedInputs(tools, state, saved, storage) };
  };
  const first = tab(), second = tab();
  first.state.pressure.high = "250";
  first.save();
  second.state.fit.model = "2";
  second.save();
  assert.deepEqual(readRememberedInputs(tools, storage), {
    pressure: { high: "250" },
    fit: { model: "2" },
  });
});
