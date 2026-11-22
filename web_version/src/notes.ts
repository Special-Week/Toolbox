/** 说明随当前模式切换，仅组织文案，不参与数值计算。 */
import type { State, Tool } from "./types";

export function calculationNote(tool: Tool, state: State) {
  const keys: Record<string, string> = {
    standard_couple: "source",
    sprt: "model",
    thermistor: "family",
    pressure_reference: "referenceType",
    transmitter_reference: "assessment",
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
  // Web 快速页逐项设置 B 类分布、自由度与灵敏系数，A 类固定采用贝塞尔法。
  if (tool.kind === "uncertainty" && mode === 1)
    return {
      note: "A 类按重复测量（贝塞尔法）评定；B 类各项分别设置分布、自由度与灵敏系数。可按平方和根或最大绝对贡献合成，U=k·uc；平方和根模式支持 Welch–Satterthwaite 有效自由度与 95%/99% t 分布包含因子。",
      formula:
        "使用 A 类时，uA=s/√n（自由度 ν=m−1），s 由全部重复测量值计算；n 为取测量次数，留空按 1 次，不使用 A 类时取 0。B 类各项的标准不确定度 u=输入值/d，正态项填证书扩展不确定度，其余填区间半宽；自由度留空按 ∞ 处理。各项先按设置修约，再乘各自灵敏系数得到贡献；按选定方法取平方和根或最大绝对贡献。\nd：均匀 √3、三角 √6、正态证书包含因子 2 或 3、两点 1、反正弦 √2；证书包含因子与合成结果的包含因子 k 分别设置。\n平方和根模式按 Welch–Satterthwaite 公式计算有效自由度 νeff=uc⁴/Σ(qᵢ⁴/νᵢ)，可选手动 k，或用双侧 t 分布分位数 t_p(νeff) 估计 95%/99% 覆盖区间。适用于统计独立的分量且 t 分布近似合理的情况。非正态分量占主导时，需另行评估覆盖区间。\n先用原始 uc 计算 U=k×uc，再分别对 uc 和 U 按所选有效数字修约。四舍六入按五成双处理。",
    };
  const note = tool.noteModes.find((item) => item.mode === mode) ?? {
    note: tool.note,
    formula: tool.formula,
  };
  if (tool.kind === "uncertainty")
    return {
      ...note,
      formula: `${note.formula}\n分量填写有效数字后先修约，再乘灵敏系数参与合成；留空保留原始计算精度。四舍六入按五成双处理。`,
    };
  if (tool.kind === "thermocouple" || tool.kind === "standard_couple")
    return {
      ...note,
      formula: note.formula
        .replaceAll("E测=E(t热端)−E(t参考端)", "E测=E(t热端)−E(t参考端)−热电势修正量")
        .replaceAll("反算先加回参考端电势", "反算先加回参考端电势和热电势修正量"),
    };
  if (tool.kind === "resultant" && mode === 1)
    return {
      ...note,
      formula: note.formula.replace(
        "此模式只显示平方和根结果。",
        "同时显示最坏情况界限 Σ|e[i]| 和最大单项贡献。",
      ),
    };
  return note;
}
