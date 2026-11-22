import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import { createServer } from "vite";

async function loadResultCsv(t) {
  const server = await createServer({
    root: fileURLToPath(new URL("..", import.meta.url)),
    configFile: false,
    optimizeDeps: { noDiscovery: true },
    server: { middlewareMode: true, ws: false },
  });
  t.after(() => server.close());
  return (await server.ssrLoadModule("/src/export.ts")).resultCsv;
}

test("结果 CSV 保留计算输入、软件版本、取值说明和未作显示修约的结果", async (t) => {
  const resultCsv = await loadResultCsv(t);
  const { version } = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));
  const tool = { id: "uncertainty", name: "不确定度计算", kind: "uncertainty" };
  const input = { digits: "3", roundingV2: "1", values: ["0.143200", "0.271"], unit: 'mV "A"' };
  const result = {
    metrics: [{ label: "扩展不确定度", value: 0.11412345678912, unit: "mV" }],
    coefficients: [], tolerances: [], rows: [], warnings: [],
  };
  const csv = resultCsv(tool, result, input, { note: "按所选规则修约", formula: "U = k × uc" });
  const inputPrefix = "输入参数 (JSON),";
  const inputLine = csv.split("\r\n").find((line) => line.startsWith(inputPrefix));
  assert.deepEqual(JSON.parse(inputLine.slice(inputPrefix.length + 1, -1).replaceAll('""', '"')), input);
  assert.ok(csv.includes(`软件版本,Toolbox Web ${version}`));
  assert.ok(csv.includes("扩展不确定度,0.11412345678912,mV"));
  assert.ok(csv.includes("U = k × uc"));
});

test("大基准二次拟合可通过导出的归一化预测式复算", async (t) => {
  const resultCsv = await loadResultCsv(t);
  const { calculate, initSync } = await import("../pkg/toolbox_core.js");
  initSync({ module: readFileSync(new URL("../pkg/toolbox_core_bg.wasm", import.meta.url)) });
  const tool = { id: "calibration_curve_fit", name: "校准曲线拟合", kind: "fit" };
  const base = 1e9;
  const input = { model: 2, points: [0, 1, 2, 3].map((d) => [base + d, d * d]) };
  const result = JSON.parse(calculate(tool.id, JSON.stringify(input)));
  const csv = resultCsv(tool, result, input);
  const block = csv.match(/归一化预测式（用于复算）,"([^"]+)"/u)[1];
  const [, x0, sx, y0, sy, coefficients] = block.match(
    /X = \(x − \(([^)]+)\)\) \/ ([^\n]+)\nŷ = \(([^)]+)\) \+ ([^ ]+) × P\(X\)\nP 的升幂系数 = (\[[^\]]+\])/u,
  );
  const x = (base + 2 - Number(x0)) / Number(sx);
  const predicted = Number(y0) + Number(sy) * JSON.parse(coefficients).reduceRight((acc, c) => acc * x + c, 0);
  assert.ok(Math.abs(predicted - result.rows[2][3]) <= 1e-12);
  assert.equal(result.rows[2][3], 4);
  assert.ok(csv.includes("原坐标展开式（供参考）"));
});
