import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { calculate, gauge_page, initSync } from "../pkg/toolbox_core.js";

initSync({ module: readFileSync(new URL("../pkg/toolbox_core_bg.wasm", import.meta.url)) });
const cases = JSON.parse(readFileSync(new URL("../../tests/boundaries.json", import.meta.url), "utf8"));
assert.ok(cases.length > 0);
for (const item of cases) {
  test(item.id, () => {
    assert.notEqual(Object.hasOwn(item, "error"), Object.hasOwn(item, "expected"));
    const run = () => JSON.parse(calculate(item.tool, JSON.stringify(item.input)));
    if (Object.hasOwn(item, "error")) {
      assert.equal(typeof item.error, "string");
      assert.ok(item.error.length > 0);
      assert.throws(run, (error) => String(error).includes(item.error));
      return;
    }
    const result = run();
    const actual = item.metric === "total" ? result.total : result.metrics.find((metric) => metric.label === item.metric)?.value;
    if (item.tolerance) {
      assert.ok(Number.isFinite(actual) && Math.abs(actual - item.expected) <= item.tolerance, `${actual} != ${item.expected}`);
    } else {
      assert.equal(actual, item.expected);
    }
    if (item.id === "division-decimal-endpoint") {
      assert.equal(result.rows.at(-1)[0], 22.1);
      assert.equal(result.rows.at(-2)[0], 20.4);
    } else if (item.id === "spline-finite-wide-preview") {
      assert.ok(result.curve.every(([x, y]) => Number.isFinite(x) && Number.isFinite(y)));
      assert.deepEqual(result.curve.at(-1), [1e306, 1]);
    } else if (item.id.startsWith("repeatability-scaled-residuals-")) {
      assert.equal(result.metrics.find((metric) => metric.label === "极差").value, null);
      assert.ok(result.warnings.includes("极差超出双精度范围。"));
    } else if (item.id === "repeatability-relative-deviation-overflow") {
      assert.equal(result.metrics.find((metric) => metric.label === "相对标准偏差 RSD").value, null);
      assert.ok(result.warnings.includes("相对标准偏差超出双精度范围。"));
    }
  });
}

test("量规报告记录参与计算的输入和软件版本", () => {
  const tool = "gauge_metric";
  const page = JSON.parse(gauge_page(tool, '{"diameter":"20.0000","pitch":"2.5"}', ""));
  const inputPrefix = "输入参数 (JSON)：";
  const inputLine = page.report.text.split("\n").find((line) => line.startsWith(inputPrefix));
  assert.deepEqual(JSON.parse(inputLine.slice(inputPrefix.length)), page.state);
  assert.equal(page.state.diameter, "20.0000");
  assert.ok(page.report.text.includes(`工具标识：${tool}`));
  assert.ok(page.report.text.includes("软件版本：Toolbox "));
});

test("UN 量规保留待处理的尺寸联动并在计算后清除标记", () => {
  const page = (input) => JSON.parse(gauge_page("gauge_unified", JSON.stringify(input), ""));
  const state = page({}).state;
  const resized = page({ ...state, size: "3/8", _threadChange: "size", "class.plug": "3B" });
  assert.equal(resized.state.tpi, "16");
  assert.equal(resized.state.series, "UNC");
  assert.equal(resized.state["class.plug"], "3B");
  assert.equal(resized.state._threadChange, undefined);
  const edited = page({ ...state, size: "3/8", tpi: "24", _threadChange: "tpi" });
  assert.equal(edited.state.series, "UNF");
});

test("UN 细牙规格跨过不完整的分数输入后保留同系列", () => {
  const page = (input) => JSON.parse(gauge_page("gauge_unified", JSON.stringify(input), ""));
  const fine = page({ size: "1/4", tpi: "28" }).state;
  const whole = page({ ...fine, size: "3", _threadChange: "size" });
  assert.equal(whole.state.series, "UNC");
  const partial = page({ ...whole.state, size: "3/", _threadChange: "size" });
  assert.ok(partial.error);
  const finished = page({ ...partial.state, size: "3/8", _threadChange: "size" });
  assert.equal(finished.state.series, "UNF");
  assert.equal(finished.state.tpi, "24");
});

test("UN 输入在去抖期间先修改 TPI 再修改直径仍保留新系列", () => {
  const page = (input) => JSON.parse(gauge_page("gauge_unified", JSON.stringify(input), ""));
  const initial = page({}).state;
  const updated = page({
    ...initial, size: "3/8", tpi: "28", _threadChange: "size", _tpiAtSize: ["1/4", "28"],
  });
  assert.equal(updated.state.series, "UNF");
  assert.equal(updated.state.tpi, "24");
  assert.equal(updated.state._tpiAtSize, undefined);
});

test("β 热敏电阻校准温区两端电阻相同时报告反算歧义", () => {
  const input = { family: 1, beta: 1e-14, r0: 1, t0: 25, min: 20, max: 30, direction: 1, value: 1 };
  assert.throws(() => calculate("thermistor", JSON.stringify(input)), /不能唯一反算温度/);
});

test("不确定度预算只在最大贡献子组实际影响合成时要求手动 k", () => {
  const leaf = { kind: 1, halfWidth: 1, distribution: 4 };
  const group = { kind: 2, combine: 1, children: [leaf] };
  const input = { components: [leaf, group], coverageMode: 1 };
  const run = (state) => JSON.parse(calculate("uncertainty_calculation", JSON.stringify(state)));
  assert.throws(() => run(input), /不适用/);
  const fallback = run({ ...input, allowManualCoverageFallback: true, coverage: "" });
  assert.equal(fallback.details.manualCoverageRequired, true);
  const manual = run({ ...input, coverageMode: 0, coverage: 2.5 });
  assert.equal(manual.details.automaticCoverage, false);
  assert.equal(manual.metrics.find(({ label }) => label === "包含因子 k").value, 2.5);
  const automatic = run({ ...input, components: [leaf, { ...group, sensitivity: 0 }] });
  assert.equal(automatic.details.automaticCoverage, true);
  const invalidK = run({
    ...input, components: [leaf, { ...group, sensitivity: 0 }],
    coverageMode: 0, coverage: 0, allowManualCoverageFallback: true,
  });
  assert.equal(invalidK.details.automaticCoverage, true);
  assert.match(invalidK.details.coverageError, /包含因子须大于 0/);
});

test("负温水面露点可回填而不被解释为霜点", () => {
  const run = (input) => JSON.parse(calculate("temperature_humidity_conversion", JSON.stringify(input)));
  const expected = run({ temperature: 20, known: 0, value: 10 });
  const dew = expected.metrics.find(({ label }) => label === "露点").value;
  const restored = run({ temperature: 20, known: 4, value: dew });
  const humidity = restored.metrics.find(({ label }) => label === "相对湿度").value;
  assert.ok(Math.abs(humidity - 10) < 1e-10);
  assert.ok(restored.process.some(line => line.includes("水面露点")));
});

test("极干空气的水面露点往返保留可表示的次正规分压", () => {
  const run = (input) => JSON.parse(calculate("temperature_humidity_conversion", JSON.stringify(input)));
  const forward = run({ temperature: 20, known: 0, value: 1e-323 });
  const value = forward.metrics.find(({ label }) => label === "露点").value;
  const reverse = run({ temperature: 20, known: 4, value });
  const vapor = (result) => result.metrics.find(({ label }) => label === "水气分压").value;
  assert.ok(vapor(forward) > 0);
  assert.equal(vapor(reverse), vapor(forward));
  assert.ok(reverse.process.some(line => line.includes("ln(e)")));
});

test("标准大气正算的端点显示气压可回填，其他越界气压仍拒绝", () => {
  const run = (direction, value) => JSON.parse(calculate("altitude_pressure_converter", JSON.stringify({ direction, value })));
  for (const [, height, outside] of [["high", 84852, -1], ["low", -1000, 1]]) {
    const exact = run(0, height).metrics.find(m => m.label === "绝对气压" && m.unit === "Pa").value;
    const displayed = Number(exact.toPrecision(11));
    const inverse = run(1, displayed);
    assert.equal(inverse.metrics[0].value, height);
    assert.equal(inverse.metrics[1].value, exact);
    assert.throws(() => run(1, exact + outside * exact * 1e-8), /绝对气压超出/);
  }
});
