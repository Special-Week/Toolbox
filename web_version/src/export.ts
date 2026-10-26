/** CSV 同时包含摘要与明细，避免有表格时漏掉方程系数和相关性等结果。 */
import type { Result, Tool } from "./types";
import { csvCell, decimal } from "./format";

export function resultCsv(tool: Tool, result: Result, note = { note: tool.note, formula: tool.formula }): string {
  const rows: unknown[][] = [
    ["工具", tool.name],
    [],
    ["计算结果"],
    ["参数", "结果", "单位"],
    ...result.metrics.map((m) => [m.label, m.value ?? "未定义", m.unit]),
  ];
  if (result.equation) rows.push([], ["拟合方程", "y = " + result.equation]);
  if (result.coefficients.length)
    rows.push(
      [],
      ["方程系数（升幂）"],
      ...result.coefficients.map((v, j) => [
        tool.kind === "standard_couple"
          ? "C" + j
          : tool.kind === "sprt"
            ? ["A", "B", "C"][j]
            : "K" + j,
        v,
      ]),
    );
  if (result.tolerances.length) {
    rows.push(
      [],
      ["等级允差"],
      [
        "等级",
        "允差（±）",
        "温度单位",
        "温度下限",
        "温度上限",
        "信号下限",
        "信号上限",
        "信号单位",
        "公式",
        "适用下限 (°C)",
        "适用上限 (°C)",
      ],
      ...result.tolerances.map((t) => [
        t.name,
        t.delta ?? "超出适用范围",
        t.unit,
        t.temperature?.[0],
        t.temperature?.[1],
        t.signal?.[0],
        t.signal?.[1],
        t.signalUnit,
        t.formula,
        t.min,
        t.max,
      ]),
    );
  }
  if (result.details) {
    // 详细结果保留原始精度，包括分量树、传感器系数和校准点残差。
    rows.push(
      [],
      [
        "详细结果",
        JSON.stringify(
          result.details,
          (_key, value) => (typeof value === "number" ? decimal(value) : value),
          2,
        ),
      ],
    );
  }
  if (result.rows.length)
    rows.push([], ["结果明细"], result.headings, ...result.rows);
  if (result.warnings.length)
    rows.push([], ["说明"], ...result.warnings.map((w) => [w]));
  if (note.note) rows.push([], ["计算说明"], [note.note]);
  if (note.formula) rows.push([], ["详细公式与取值"], [note.formula]);
  return "\uFEFF" + rows.map((row) => row.map(csvCell).join(",")).join("\r\n");
}
