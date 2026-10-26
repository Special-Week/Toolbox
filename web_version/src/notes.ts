/** 说明随当前模式切换，仅组织文案，不参与数值计算。 */
import type { State, Tool } from "./types";

export function calculationNote(tool: Tool, state: State) {
  const keys: Record<string, string> = {
    standard_couple: "source",
    sprt: "model",
    thermistor: "family",
    pressure_reference: "referenceType",
    ear: "model",
    blackbody: "model",
    radiation: "model",
    resultant: "mode",
    uncertainty: "mode",
  };
  const key = keys[tool.kind];
  let mode = key ? Number(state[key] ?? 0) : 0;
  if (tool.kind === "sprt" && mode === 0 && Number(state.deviation) === 1)
    mode = 2;
  // Web 快速页逐项设置 B 类分布和灵敏系数，与桌面共用设置不同。
  if (tool.kind === "uncertainty" && mode === 1) mode = 2;
  return tool.noteModes.find((item) => item.mode === mode) ?? {
    note: tool.note,
    formula: tool.formula,
  };
}
