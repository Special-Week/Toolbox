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
  const note = tool.noteModes.find((item) => item.mode === mode) ?? {
    note: tool.note,
    formula: tool.formula,
  };
  if (tool.kind === "uncertainty")
    return {
      ...note,
      formula: `${note.formula.replace("最低为整数位", "可修约至十位、百位等")}\n贝塞尔法先用全部测量值计算样本标准差 s，再按 u=s/√n 计算；取测量次数 n 留空时默认为 1。\n分量填写有效数字后先修约，再乘灵敏系数参与合成；留空保留原始计算精度。四舍六入按五成双处理。${mode === 2 ? "\n合成结果按 uc 和 U 较大值确定修约位数；修约后 U=k×修约后的 uc。" : ""}`,
    };
  return note;
}
