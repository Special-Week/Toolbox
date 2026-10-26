/** 页面字段与公式分离；所有数值结果仍由 Rust 返回。 */
import type { Catalog, Field, Section, State, Tool, Budget } from "./types";
import { decimal } from "./format";
const samples = ["10.02", "10.01", "10.03", "10.02", "10.01"];
export const fitPoints = [
  ["0", "0.197"],
  ["5", "0.208"],
  ["10", "0.231"],
  ["20", "0.263"],
  ["40", "0.338"],
  ["60", "0.439"],
];
export const splinePoints = [
  ["250", "2002"],
  ["550", "6531"],
  ["850", "12285"],
  ["1150", "18708"],
  ["1450", "25475"],
  ["1750", "32170"],
  ["2050", "38455"],
  ["2350", "43954"],
  ["2650", "48340"],
];
export function newBudget(name = "分量"): Budget {
  return {
    name,
    kind: "1",
    sensitivity: "1",
    values: [...samples],
    method: "0",
    halfWidth: "0.02",
    distribution: "0",
    combine: "0",
    children: [],
  };
}
const n = (
  key: string,
  label: string,
  value: unknown,
  hint?: string,
): Field => ({
  key,
  label,
  type: "number",
  default: typeof value === "number" ? decimal(value) : String(value),
  hint,
});
const text = (key: string, label: string, value: string): Field => ({
  key,
  label,
  type: "text",
  default: value,
});
const sel = (
  key: string,
  label: string,
  options: string[],
  value = 0,
): Field => ({
  key,
  label,
  type: "select",
  default: String(value),
  options: options.map((label, value) => ({ label, value: String(value) })),
});
const vector = (
  key: string,
  label: string,
  values: unknown[],
  min = 1,
  max = 50,
  labels?: string[],
): Field => ({
  key,
  label,
  type: "vector",
  default: values.map((v) => (typeof v === "number" ? decimal(v) : String(v))),
  min,
  max,
  labels,
});
const points = (
  label: string,
  values: string[][],
  min: number,
  max: number,
): Field => ({
  key: "points",
  label,
  type: "points",
  default: values,
  min,
  max,
});
const section = (title: string, ...fields: Field[]): Section => ({
  title,
  fields,
});
const idx = (s: State, key: string, d = 0) => Number(s[key] ?? d);
const units = ["°C", "K", "°F"];
const tempUnit = () => sel("unit", "温度单位", units);
const direction = () =>
  sel("direction", "计算方式", ["温度 → 电信号", "电信号 → 温度"]);
const temp = (c: number, s: State) =>
  idx(s, "unit") === 1 ? c + 273.15 : idx(s, "unit") === 2 ? c * 1.8 + 32 : c;
export function schema(t: Tool, s: State, c: Catalog): Section[] {
  const kind = t.kind;
  const u = units[idx(s, "unit")] ?? "°C";
  const dir = idx(s, "direction");
  if (kind === "unit")
    return [
      section(
        "换算参数",
        n("value", "输入数值", "1"),
        sel(
          "source",
          "输入单位",
          t.units!.map((x) => `${x.name} · ${x.code}`),
        ),
      ),
    ];
  if (kind === "thermocouple")
    return [
      section(
        "热电偶参数",
        sel(
          "type",
          "分度号",
          [
            "B",
            "E",
            "J",
            "K",
            "N",
            "R",
            "S",
            "T",
            "C",
            "D",
            "G",
            "L",
            "LR",
            "U",
            "A",
          ].map((x) => x + " 型"),
          3,
        ),
        tempUnit(),
        direction(),
        n(
          "value",
          dir ? "实测热电势 (mV)" : `热端温度 (${u})`,
          dir ? "4.096230218723254" : temp(100, s),
        ),
        n("cold", `参考端温度 (${u})`, temp(0, s)),
        n("correction", "热电势修正量 (mV)", 0),
      ),
    ];
  if (kind === "rtd") {
    const g = c.rtd[idx(s, "graduation")] ?? c.rtd[0];
    const f = [
      sel(
        "graduation",
        "分度号",
        c.rtd.map((g) => g.name),
      ),
      tempUnit(),
      direction(),
      n("r0", "零度电阻 R₀ (Ω)", g.r0),
    ];
    if (g.key === "cvd")
      f.push(
        n("a", "系数 A", 0.0039083),
        n("b", "系数 B", -5.775e-7),
        n("c", "系数 C", -4.183e-12),
      );
    f.push(
      n(
        "value",
        dir ? "电阻 (Ω)" : `温度 (${u})`,
        dir ? g.r0 * 1.385055 : temp(100, s),
      ),
      sel("structure", "元件结构", ["绕线式", "薄膜式"]),
    );
    return [section("热电阻参数", ...f)];
  }
  if (kind === "standard_couple") {
    const k = idx(s, "type");
    const zones = c.zones[k] ?? c.zones[0];
    const z = zones[idx(s, "zone")] ?? zones[0];
    const fields = [
      sel("type", "分度号", ["S 型", "B 型"]),
      sel("source", "校准方式", ["固定点实测热电势", "温区多项式系数"]),
    ];
    if (idx(s, "source") === 0)
      fields.push(
        vector(
          "fixed",
          "固定点实测热电势 (mV)",
          k
            ? [5.8029, 6.805, 7.8809, 8.9926, 10.1205]
            : [3.4472, 5.8605, 10.5822],
          k ? 5 : 3,
          k ? 5 : 3,
          (k
            ? [1100, 1200, 1300, 1400, 1500]
            : [419.527, 660.323, 1084.62]
          ).map((x) => x + " °C"),
        ),
      );
    else {
      fields.push(
        sel(
          "zone",
          "系数温区",
          zones.map((z) => `${z.min}～${z.max} °C`),
        ),
        sel("coefficientSource", "系数设置", ["手动输入系数", "校准点拟合"]),
      );
      if (idx(s, "coefficientSource") === 1)
        fields.push(
          points("校准点：温度 (°C) / 实测电势 (mV)", z.points, 2, 30),
        );
      else
        fields.push(
          vector(
            "coefficients",
            "多项式系数（升幂）",
            z.coefficients,
            z.coefficients.length,
            z.coefficients.length,
            z.coefficients.map((_, j) => `C${j}`),
          ),
        );
    }
    fields.push(
      tempUnit(),
      direction(),
      n(
        "value",
        dir ? "实测热电势 (mV)" : `温度 (${u})`,
        dir
          ? idx(s, "source") === 1
            ? z.points[1][1]
            : 9.587
          : temp(
              idx(s, "source") === 1
                ? Math.max(z.min, Math.min(1000, z.max))
                : 1000,
              s,
            ),
      ),
      n("cold", `参考端温度 (${u})`, temp(0, s)),
      n("correction", "热电势修正量 (mV)", 0),
    );
    return [section("标准热电偶参数", ...fields)];
  }
  if (kind === "sprt") {
    const fields = [sel("model", "计算模型", ["SPRT 换算", "CVD 系数拟合"])];
    if (idx(s, "model") === 1)
      return [
        section(
          "CVD 系数拟合",
          ...fields,
          n("r0", "R₀ (Ω)", 100),
          points(
            "校准点：温度 (°C) / 电阻 (Ω)",
            [
              ["-100", "60.25584"],
              ["100", "138.5055"],
              ["200", "175.856"],
            ],
            3,
            3,
          ),
        ),
      ];
    const hr = idx(s, "highRange", 1);
    const lr = idx(s, "lowRange");
    fields.push(
      sel(
        "highRange",
        "高温分区",
        [
          "A6/B6/C6：至 961.78 °C",
          "A7/B7/C7：至 660.323 °C",
          "A8/B8：至 419.527 °C",
          "A9/B9：至 231.928 °C",
          "A10：至 156.599 °C",
          "A11：至 29.765 °C",
          "不使用高温分区",
        ],
        1,
      ),
      sel(
        "lowRange",
        "低温分区",
        c.lowRanges.map((x) => x.label),
      ),
      sel("deviation", "偏差函数", [
        "ITS-90 子区间偏差函数",
        "三项高温 / 对数低温偏差",
      ]),
      sel("source", "系数设置", ["输入证书系数", "固定点电阻拟合"]),
      n("rtp", "水三相点电阻 Rtp (Ω)", 25.12461),
    );
    if (idx(s, "source") === 0) {
      if (hr !== 6) {
        fields.push(n("a", "a", -0.000272072));
        if (hr <= 3 || idx(s, "deviation") === 1)
          fields.push(n("b", "b", 0.0000178924));
        if (hr <= 1 || idx(s, "deviation") === 1)
          fields.push(n("c", "c", -0.0000112269));
      }
      if (lr !== 2)
        fields.push(
          n("a4", lr === 1 ? "a5" : "a4", 0),
          n("b4", lr === 1 ? "b5" : "b4", 0),
        );
      if (hr === 0 && idx(s, "deviation") === 0)
        fields.push(n("d", "d6", 0), n("w660", "W660", 3.376008599409339));
    } else {
      const highs = [
        [231.928, 419.527, 660.323],
        [231.928, 419.527, 660.323],
        [231.928, 419.527],
        [156.599, 231.928],
        [156.599],
        [29.765],
        [],
      ][hr];
      // 固定点默认电阻由 Rust 元数据按相同参考模型生成。
      if (highs.length)
        fields.push(
          vector(
            "highFixed",
            "高温固定点实测电阻 (Ω)",
            highs.map((t) => c.defaults.sprtFixed[String(t)]),
            highs.length,
            highs.length,
            highs.map((t) => t + " °C"),
          ),
        );
      const lows = c.lowRanges[lr].fixed;
      if (lows.length)
        fields.push(
          vector(
            "lowFixed",
            "低温固定点实测电阻 (Ω)",
            lows.map((t) => c.defaults.sprtFixed[String(t)]),
            lows.length,
            lows.length,
            lows.map((t) => t + " °C"),
          ),
        );
      if (hr === 0)
        fields.push(
          n("silver", "银点实测电阻 (Ω)", c.defaults.sprtFixed["961.78"]),
        );
    }
    fields.push(
      tempUnit(),
      direction(),
      n(
        "value",
        dir ? "电阻 (Ω)" : `温度 (${u})`,
        dir
          ? lr === 2
            ? Number(s.rtp ?? 25.12461) * 1.01
            : 25
          : temp(hr === 6 ? -20 : 20, s),
      ),
    );
    return [section("标准铂电阻参数", ...fields)];
  }
  if (kind === "thermistor") {
    const fields = [
      sel("family", "计算模型", ["多项式 / Steinhart–Hart", "β 模型"]),
    ];
    if (idx(s, "family") === 1)
      fields.push(
        tempUnit(),
        n("r0", "参考电阻 R₀ (Ω)", 10000),
        n("t0", `参考温度 (${u})`, temp(25, s)),
        n("beta", "β (K)", 3950),
      );
    else {
      fields.push(
        sel("form", "多项式形式", [
          "完整多项式",
          "Steinhart–Hart（去除二次项）",
        ]),
        sel("order", "多项式阶数", ["3 阶", "4 阶", "5 阶", "6 阶"]),
        sel("source", "系数设置", ["手动输入系数", "校准点最小二乘拟合"]),
      );
      const degree = idx(s, "order") + 3;
      const a = Array(degree + 1).fill("0");
      const b = [...a];
      [a[0], a[1]] = c.defaults.ntcA;
      [b[0], b[1]] = c.defaults.ntcB;
      if (idx(s, "source") === 0) {
        for (const [key, label, v] of [
          ["aCoefficients", "系数 A：1/T = ΣA[i]·ln(R)^i", a],
          ["bCoefficients", "系数 B：ln(R) = ΣB[i]·(1/T)^i", b],
        ] as const) {
          const f = vector(
            key,
            label,
            [...v],
            degree + 1,
            degree + 1,
            v.map((_, j) => `${key === "aCoefficients" ? "A" : "B"}${j}`),
          );
          if (idx(s, "form") === 1) f.locked = [2];
          fields.push(f);
        }
      } else
        fields.push(
          points(
            "校准点：温度 (°C) / 电阻 (Ω)",
            c.defaults.ntcPoints,
            idx(s, "form") === 1 ? degree : degree + 1,
            30,
          ),
        );
      fields.push(tempUnit());
    }
    fields.push(
      direction(),
      n("value", dir ? "电阻 (Ω)" : `温度 (${u})`, dir ? 10000 : temp(25, s)),
      n("min", "校准温区下限 (°C)", -20),
      n("max", "校准温区上限 (°C)", 100),
    );
    return [section("热敏电阻参数", ...fields)];
  }
  if (kind === "fit")
    return [
      {
        ...section(
          "拟合设置",
          sel("model", "曲线模型", [
            "一次：y = K1·x + K0",
            "过原点一次：y = K1·x",
            "二次多项式",
            "三次多项式",
          ]),
        ),
        advanced: {
          title: "坐标名称与单位（选填）",
          fields: [
            text("xName", "输入量名称", "输入量"),
            text("xUnit", "输入量单位", ""),
            text("yName", "响应量名称", "响应量"),
            text("yUnit", "响应量单位", ""),
          ],
        },
      },
      section(
        "校准数据",
        points("成对数据：输入量 x / 响应量 y", fitPoints, 2, 200),
      ),
    ];
  if (kind === "spline")
    return [
      section(
        "采样设置",
        n("start", "采样最小 x", 250),
        n("end", "采样最大 x", 2650),
        n("step", "采样步进", 0.5),
      ),
      section("Q 型值点", points("型值点：x / y", splinePoints, 2, 500)),
    ];
  if (kind === "humidity")
    return [
      section(
        "环境与已知参数",
        tempUnit(),
        n("temperature", `环境温度 (${u})`, temp(20, s)),
        n("pressure", "大气绝对压力 (kPa)", 101.325),
        sel("known", "已知参数", [
          "相对湿度",
          "露点 / 霜点",
          "湿球温度值",
          "混合比",
        ]),
        n(
          "value",
          [
            "相对湿度 (%RH)",
            `露点 / 霜点 (${u})`,
            `湿球温度 (${u})`,
            "混合比 (g/kg)",
          ][idx(s, "known")],
          [50, temp(10, s), temp(15, s), 7.5][idx(s, "known")],
        ),
      ),
    ];
  if (kind === "ear")
    return [
      section(
        "黑体与探头参数",
        sel("model", "反射模型", ["多次反射平衡", "一次反射近似"]),
        n("target", "黑体真实温度 (°C)", 35),
        n("emissivity", "黑体发射率", 0.9995),
        n("probeTemperature", "探头温度 (°C)", 18),
        n("probeEmissivity", "探头发射率", 0.8),
      ),
    ];
  if (kind === "blackbody" || kind === "radiation") {
    const f = [
      sel("model", "计算模型", ["有效波长法", "Planck 辐射平衡"]),
      sel("response", "光谱响应", [
        "单一有效波长",
        "8～14 µm 波段",
        "7～18 µm 波段",
      ]),
    ];
    if (idx(s, "response") === 0)
      f.push(n("wavelength", "有效波长 (µm)", 0.65));
    f.push(
      n(
        "temperature",
        kind === "radiation" ? "温度计示值 (°C)" : "辐射源真实温度 (°C)",
        1000,
      ),
      n("emissivity", "辐射源发射率", 0.99),
      n("ambient", "反射背景温度 (°C)", 20),
    );
    if (kind === "radiation")
      f.push(n("instrumentEmissivity", "温度计设定发射率", 0.95));
    return [section("辐射温度参数", ...f)];
  }
  if (kind === "repeatability")
    return [
      section("测量数据", vector("values", "逐项测量值", samples, 2, 50)),
    ];
  if (kind === "range") {
    const f = [sel("mode", "输入方式", ["逐项测量值", "最小值与最大值"])];
    if (idx(s, "mode") === 0)
      f.push(vector("values", "逐项测量值", samples, 2, 9));
    else
      f.push(
        n("min", "最小值", 10.01),
        n("max", "最大值", 10.03),
        n("count", "测量次数（2～9）", 5),
      );
    return [section("极差法参数", ...f)];
  }
  if (kind === "error")
    return [
      section(
        "误差参数",
        n("value", "测量值", 10.02),
        n("reference", "参考真值", 10),
        n("base", "引用值", 10),
      ),
    ];
  if (kind === "resultant") {
    const f = [
      sel("mode", "分量输入", [
        "同单位误差分量",
        "压力分量（相对 / 绝对误差）",
      ]),
    ];
    if (idx(s, "mode") === 0) {
      f.push(
        vector("values", "误差分量", [0.02, 0.03, 0.01], 1, 50),
        vector(
          "sensitivities",
          "灵敏系数",
          Array((s.values as unknown[] | undefined)?.length ?? 3).fill("1"),
          1,
          50,
        ),
      );
    } else
      f.push(
        sel(
          "pressureUnit",
          "结果压力单位",
          c.tools
            .find((x) => x.id === "unit_pressure")!
            .units!.map((x) => x.code),
        ),
        {
          key: "pressureComponents",
          label: "压力误差分量",
          type: "pressure",
          default: [
            { unit: "0", type: "0", low: "0", high: "100", value: "0.1" },
          ],
        },
      );
    return [section("误差分量", ...f)];
  }
  if (kind === "uncertainty") {
    const f = [sel("mode", "评定方式", ["分量预算表", "快速评定"])];
    if (idx(s, "mode") === 0)
      f.push({
        key: "components",
        label: "不确定度预算",
        type: "budget",
        default: [{ ...newBudget("重复测量"), kind: "0" }, newBudget("标准器")],
      });
    else
      f.push(
        vector("aValues", "A 类重复测量数据", samples, 2, 50),
        n("aSensitivity", "A 类灵敏系数", 1),
        {
          key: "bComponents",
          label: "B 类分量",
          type: "budget",
          min: 1,
          max: 50,
          budgetKinds: ["1"],
          // 兼容旧版快速评定的半宽列表，保留输入值和原先共用的分布。
          default: (
            (s.bValues as unknown[] | undefined) ?? ["0.01", "0.02"]
          ).map((v, j) => ({
            ...newBudget(`B 类分量 ${j + 1}`),
            halfWidth: String(v),
            distribution: String(s.distribution ?? "0"),
          })),
        },
      );
    f.push(
      sel("combine", "合成方法", ["平方和根", "最大绝对贡献"]),
      n("coverage", "包含因子 k", 2),
      sel("digits", "修约有效数字", ["1 位", "2 位", "3 位", "4 位"], 1),
      sel("rounding", "修约方式", ["四舍五入", "进位"]),
    );
    return [section("不确定度评定", ...f)];
  }
  if (kind === "height") {
    const f = [
      sel("medium", "介质类型", ["液体（恒定密度）", "气体（局部密度）"]),
      n("height", "被检表高于标准器的高度 (m)", 1),
      n("gravity", "重力加速度 (m/s²)", 9.80665),
    ];
    if (idx(s, "medium") === 1)
      f.push(
        sel(
          "gasState",
          "气体密度条件",
          ["输入当地状态密度", "由标准状态密度换算"],
          1,
        ),
      );
    if (idx(s, "medium") === 0 || idx(s, "gasState", 1) === 0)
      f.push(
        n(
          "density",
          "当地介质密度 (kg/m³)",
          idx(s, "medium") === 0 ? 1000 : 1.204,
        ),
      );
    else
      f.push(
        n("normalDensity", "标准状态气体密度 (kg/m³)", 1.293),
        n("temperature", "气体温度 (°C)", 20),
        n("pressure", "压力读数 (Pa)", 101325),
        n("ambient", "环境绝对压力 (Pa)", 101325),
        sel("reference", "压力参考基准", ["表压", "绝压"]),
      );
    return [section("高度差参数", ...f)];
  }
  if (kind === "altitude")
    return [
      section(
        "标准大气换算",
        sel("direction", "计算方式", ["高度 → 气压", "气压 → 高度"]),
        n(
          "value",
          dir ? "绝对气压 (Pa)" : "位势高度 (m)",
          dir ? 89874.56 : 1000,
        ),
      ),
    ];
  if (kind === "transmitter") {
    const family = idx(s, "family");
    const f = [
      sel("family", "过程量类型", [
        "压力",
        "温度",
        "相对湿度",
        "露点",
        "绝对湿度",
      ]),
    ];
    if ([1, 3].includes(family)) f.push(tempUnit());
    if (family === 0)
      f.push(
        sel(
          "pressureUnit",
          "过程量压力单位",
          c.tools
            .find((x) => x.id === "unit_pressure")!
            .units!.map((x) => x.code),
        ),
      );
    const low = family === 1 ? temp(0, s) : family === 3 ? temp(-20, s) : 0;
    const high =
      family === 1
        ? temp(100, s)
        : family === 3
          ? temp(60, s)
          : family === 4
            ? 30
            : 100;
    f.push(
      sel("direction", "计算方向", ["过程量 → 信号", "信号 → 过程量"]),
      sel("characteristic", "传递特性", ["线性", "开方"]),
      n("low", "过程量量程下限", low),
      n("high", "过程量量程上限", high),
      sel("signalUnit", "输出信号单位", ["mA", "V", "mV"]),
      n("signalLow", "输出信号下限", 4),
      n("signalHigh", "输出信号上限", 20),
      n("value", dir ? "输入信号" : "输入过程量", dir ? 12 : (low + high) / 2),
    );
    return [section("变送器量程参数", ...f)];
  }
  if (kind === "pressure_reference" || kind === "transmitter_reference") {
    const tx = kind === "transmitter_reference";
    const options = c.tools.find((x) => x.id === "unit_pressure")!.units!;
    const f = [
      sel("mode", "选型方式", [
        "核验指定标准器",
        "按被检量程推荐",
        "按检定范围推荐",
      ]),
      sel(
        "pressureUnit",
        "压力单位",
        options.map((x) => x.code),
        options.findIndex((x) => x.code === "MPa"),
      ),
      n("low", "被检 / 检定范围下限", 0),
      n("high", "被检 / 检定范围上限", 1),
      n("accuracy", "被检准确度 (%)", tx ? 0.1 : 0.25),
      n("ratio", "量传比要求", 3),
    ];
    if (!tx)
      f.push(
        sel("referenceType", "标准器类型", ["数字压力标准器", "活塞压力计"]),
      );
    if (idx(s, "mode") === 0)
      f.push(
        n("refLow", "标准器量程下限", 0),
        n("refHigh", "标准器量程上限", 1.6),
        n("refAccuracy", "标准器准确度 (%)", tx ? 0.01 : 0.05),
      );
    else {
      f.push(sel("series", "标准量程系列", ["常用公制量程", "扩展量程"]));
      if (idx(s, "mode") === 2)
        f.push(n("refAccuracy", "标准器准确度 (%)", 0.05));
    }
    if (tx) {
      f.push(
        sel("assessment", "评定模型", [
          "压力与电测合成评定",
          "两通道跨度比评定",
        ]),
        n("signalLow", "被检输出下限 (mA)", 4),
        n("signalHigh", "被检输出上限 (mA)", 20),
        n(
          "reading",
          idx(s, "assessment") === 1
            ? "标准电测量程准确度 (%)"
            : "标准电测读数准确度 (%)",
          0.02,
        ),
      );
      if (idx(s, "assessment") === 0)
        f.push(n("fixedError", "标准电测固定误差 (mA)", 0));
      else
        f.push(
          n("signalAccuracy", "被检输出准确度 (%)", 0.1),
          n("refSignalLow", "标准电测量程下限 (mA)", 0),
          n("refSignalHigh", "标准电测量程上限 (mA)", 24),
        );
    }
    return [section("选型参数", ...f)];
  }
  return [];
}

/** 只为尚未输入的字段填默认值，保留空串、负号等正在编辑的文本。 */
export function initialize(tool: Tool, state: State, catalog: Catalog) {
  for (let pass = 0; pass < 2; pass++)
    // 折叠仅影响显示；其中的默认值和已保存输入仍正常参与页面状态。
    for (const section of schema(tool, state, catalog))
      for (const f of [
        ...section.fields,
        ...(section.advanced?.fields ?? []),
      ]) {
        if (state[f.key] === undefined)
          state[f.key] = JSON.parse(JSON.stringify(f.default));
      }
}
export function adjust(tool: Tool, s: State, key: string, c: Catalog) {
  if (
    key === "direction" ||
    key === "unit" ||
    key === "known" ||
    key === "family"
  ) {
    delete s.value;
    if (key === "unit") {
      delete s.cold;
      delete s.temperature;
      delete s.t0;
    }
  }
  if (key === "medium") delete s.density;
  if (key === "graduation") {
    delete s.r0;
    delete s.value;
  }
  if (tool.kind === "transmitter" && (key === "family" || key === "unit")) {
    delete s.low;
    delete s.high;
  }
  if (
    tool.kind === "standard_couple" &&
    ["type", "zone", "source", "coefficientSource"].includes(key)
  ) {
    delete s.fixed;
    delete s.coefficients;
    delete s.points;
    delete s.value;
    if (key === "type") delete s.zone;
  }
  if (tool.kind === "thermistor" && (key === "order" || key === "form")) {
    delete s.aCoefficients;
    delete s.bCoefficients;
  }
  if (tool.kind === "sprt" && (key === "highRange" || key === "lowRange")) {
    delete s.highFixed;
    delete s.lowFixed;
    delete s.value;
  }
  if (tool.kind === "resultant" && key === "values") {
    const count = (s.values as unknown[]).length;
    const old = (s.sensitivities as string[] | undefined) ?? [];
    s.sensitivities = Array.from({ length: count }, (_, j) => old[j] ?? "1");
  }
  // 更换温标或有效温区后重新使用对应边界，避免把旧温区数字当成新温标。
  if (
    [
      "unit",
      "type",
      "graduation",
      "zone",
      "highRange",
      "lowRange",
      "min",
      "max",
    ].includes(key)
  ) {
    delete s.tableStart;
    delete s.tableEnd;
    if (key === "unit") delete s.tableStep;
  }
  initialize(tool, s, c);
}
