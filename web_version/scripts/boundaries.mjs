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
