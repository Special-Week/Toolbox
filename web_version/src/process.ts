import sharedFormulaCatalog from "../../assets/math/formula-catalog.json";
import sharedFormulaPatterns from "../../assets/math/formula-patterns.json";

/** 给减法中的负操作数加括号；指数负号与普通名称、范围保持原样。 */
export function parenthesizeNegativeSubtrahends(line: string): string {
  const pattern = /[-−][ \t]*([-−](?:(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?|[A-Za-zα-ωΑ-Ω][A-Za-z0-9_α-ωΑ-Ω₀-₉]*|√(?:[0-9]+(?:\.[0-9]*)?|[A-Za-zα-ωΑ-Ω][A-Za-z0-9_]*|\([^()]*\))|\([^()]*\))(?:[⁰¹²³⁴⁵⁶⁷⁸⁹ⁿⁱ⁻⁺]+|\^(?:\([^()]*\)|\{[^{}]*\}|[+-]?[0-9]+|[A-Za-zα-ωΑ-Ω][A-Za-z0-9_]*))?)/gu;
  let result = "", copied = 0;
  for (const match of line.matchAll(pattern)) {
    const start = match.index, before = line.slice(0, start).trimEnd();
    const left = Array.from(before).at(-1);
    if (!left || !/[A-Za-z0-9α-ωΑ-Ω₀-₉)\]}⁰¹²³⁴⁵⁶⁷⁸⁹ⁿⁱ⁻⁺]/u.test(left)) continue;
    if (match[0].startsWith("-") && /[eE]$/u.test(before) && /[0-9.][eE]$/u.test(before)) continue;
    const end = start + match[0].length;
    if (/^[A-Za-z0-9._^]/u.test(line.slice(end))) continue;
    if (match[0].startsWith("-")) {
      const clauseStart = Math.max(...["；", "。", "，", "：", "\n"].map((mark) => line.lastIndexOf(mark, start - 1))) + 1;
      const offsets = ["；", "。", "，", "：", "\n"].map((mark) => line.indexOf(mark, end)).filter((at) => at >= 0);
      const clause = line.slice(clauseStart, offsets.length ? Math.min(...offsets) : line.length);
      const standalone = /^[A-Za-z0-9α-ωΑ-Ω₀-₉⁰¹²³⁴⁵⁶⁷⁸⁹ⁿⁱ⁻⁺._+\-−×÷*/()[\]{}^ \t]+$/u.test(line.trim());
      if (!/[=≈]/u.test(clause) && !standalone) continue;
    }
    const valueStart = end - match[1].length;
    result += line.slice(copied, valueStart) + `(${match[1]})`;
    copied = end;
  }
  return copied ? result + line.slice(copied) : line;
}

/** 仅整理带标记的计算值；相近数相减时保留足够位数解释差值。 */
export function processLines(lines: string[]): string[] {
  const values = new Map<string, string>();
  return lines.map((line) => {
    const parts = line.split("\u2063");
    for (let i = 1; i < parts.length; i += 2) {
      const raw = parts[i], value = Number(raw);
      if (Number.isFinite(value)) {
        const digits = raw.replace(/[^0-9]/gu, "").replace(/^0+/u, "");
        const tail = raw.includes(".") && digits.length >= 13 && /0000000|9999999/u.test(digits);
        const rounded = Number(value.toPrecision(tail ? 12 : 13));
        values.set(raw, rounded === 0 && value !== 0 ? raw : String(rounded));
      }
    }
    const rawLine = parts.join("");
    const preserve = new Set<string>();
    const expression = /(-?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?)\s*−\s*(-?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?)/gu;
    for (const match of rawLine.matchAll(expression)) {
      const a = Number(match[1]), b = Number(match[2]);
      const shown = Number(values.get(match[1]) ?? match[1]) - Number(values.get(match[2]) ?? match[2]);
      if (!Number.isFinite(a - b) || !Number.isFinite(shown) || Math.abs(shown - (a - b)) > Math.abs(a - b) * 1e-12) {
        preserve.add(match[1]); preserve.add(match[2]);
      }
    }
    return parenthesizeNegativeSubtrahends(parts.map((part, index) => index % 2 ? (preserve.has(part) ? part : values.get(part) ?? part) : part).join(""));
  });
}

/** 只排版明确的简单除法；复杂嵌套、连除和单位保持原样。 */
export type FormulaPart = { text: string; tex?: string; denominator?: string; image?: "helix" | "wire" | "sampleSd" | "repeatability" | "tcGaussian" | "tcPolynomial" | "tcSum" | "threadK2" | "threadK1" | "threadK2P" | "threadK1P"; wireNumerator?: string; wireAngle?: "30" | "55" | "60" | "70" | "75" };

const formulaSources: Record<NonNullable<FormulaPart["image"]>, string> = {
  helix: String.raw`\operatorname{atan}\!\left(\frac{L}{\pi D_2}\right)\times\frac{180}{\pi}`,
  wire: String.raw`\frac{P}{2\cos\!\left(\frac{\alpha}{2}\right)}`,
  sampleSd: String.raw`\sqrt{\frac{\sum v_i^2}{n-1}}`,
  repeatability: String.raw`\sqrt{\frac{\sum (x_i-\bar{x})^2}{n-1}}`,
  tcGaussian: String.raw`a_0\cdot\exp\!\left[a_1(t-a_2)^2\right]`,
  tcPolynomial: String.raw`E(t)=c_0+c_1t+c_2t^2+\cdots+c_nt^n`,
  tcSum: String.raw`\sum_i c_i t^i`,
  threadK2: String.raw`\frac{3\sqrt{3}}{8}`,
  threadK1: String.raw`\frac{5\sqrt{3}}{8}`,
  threadK2P: String.raw`\frac{3\sqrt{3}\,P}{8}`,
  threadK1P: String.raw`\frac{5\sqrt{3}\,P}{8}`,
};

const additionalFormulas: ReadonlyArray<readonly [string, string]> = [
  ["Σcᵢzⁱ", String.raw`\sum_i c_i z^i`], ["Σcᵢxⁱ", String.raw`\sum_i c_i x^i`],
  ["R/R₀", String.raw`\frac{R}{R_0}`],
  ["1 + At + Bt² + C(t−100)t³", String.raw`1+At+Bt^2+C(t-100)t^3`],
  ["1 + At + Bt²", String.raw`1+At+Bt^2`],
  ["√(Σeᵢ²/n)", String.raw`\sqrt{\frac{\sum e_i^2}{n}}`],
  ["√[Σeᵢ²/(n−p)]", String.raw`\sqrt{\frac{\sum e_i^2}{n-p}}`],
  ["√(Σe²/n)", String.raw`\sqrt{\frac{\sum e^2}{n}}`],
  ["√[Σe²/(n−p)]", String.raw`\sqrt{\frac{\sum e^2}{n-p}}`],
  ["Σ(xᵢ − x₀)/n", String.raw`\frac{\sum(x_i-x_0)}{n}`],
  ["Σ(xᵢ−x̄)²", String.raw`\sum(x_i-\bar{x})^2`],
  ["Σeᵢ²/Σ(yᵢ−ȳ)²", String.raw`\frac{\sum e_i^2}{\sum(y_i-\bar{y})^2}`],
  ["√[(cA·uA)²+(cB·uB)²]", String.raw`\sqrt{(c_Au_A)^2+(c_Bu_B)^2}`],
  ["√Σuᵢ²", String.raw`\sqrt{\sum u_i^2}`], ["√Σq[i]²", String.raw`\sqrt{\sum q_i^2}`],
  ["√Σe[i]²", String.raw`\sqrt{\sum e_i^2}`], ["u=R/(C√m)", String.raw`u=\frac{R}{C\sqrt{m}}`],
  ["u=a/√3", String.raw`u=\frac{a}{\sqrt{3}}`], ["u=a/√6", String.raw`u=\frac{a}{\sqrt{6}}`],
  ["u=a/√2", String.raw`u=\frac{a}{\sqrt{2}}`], ["s/√n", String.raw`\frac{s}{\sqrt{n}}`],
  ["x = (Wr^(1/6)−0.65)/0.35", String.raw`x=\frac{W_r^{1/6}-0.65}{0.35}`],
  ["Wr = W − a₄(W−1) − b₄(W−1)ln(W)", String.raw`W_r=W-a_4(W-1)-b_4(W-1)\ln(W)`],
  ["C(u)=ΣBᵢ(u)Pᵢ", String.raw`C(u)=\sum_i B_i(u)P_i`],
  ["B₀=(1−u)³/6", String.raw`B_0=\frac{(1-u)^3}{6}`],
  ["B₁=(3u³−6u²+4)/6", String.raw`B_1=\frac{3u^3-6u^2+4}{6}`],
  ["B₂=(−3u³+3u²+3u+1)/6", String.raw`B_2=\frac{-3u^3+3u^2+3u+1}{6}`],
  ["B₃=u³/6", String.raw`B_3=\frac{u^3}{6}`],
  ["Pᵢ₋₁+4Pᵢ+Pᵢ₊₁=6Qᵢ", String.raw`P_{i-1}+4P_i+P_{i+1}=6Q_i`],
  ["U=2√(aₚ²+aₑ²)/√3", String.raw`U=\frac{2\sqrt{a_p^2+a_e^2}}{\sqrt{3}}`],
  ["λ⁻⁵ / [exp(c₂/(λT)) − 1]", String.raw`\frac{\lambda^{-5}}{\exp\left(\frac{c_2}{\lambda T}\right)-1}`],
];

const numberPattern = String.raw`(?:-?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?|\([-−](?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?\))`;
const dynamicPatterns = sharedFormulaPatterns.map(([pattern, latex]) => [new RegExp(pattern.replaceAll("@", numberPattern), "u"), latex] as const);

function texNumber(value: string): string {
  if (value.startsWith("(") && value.endsWith(")")) return String.raw`\left(${texNumber(value.slice(1, -1))}\right)`;
  if (value.startsWith("−")) return `-${texNumber(value.slice(1))}`;
  const match = /^(-?(?:\d+(?:\.\d*)?|\.\d+))[eE]([+-]?\d+)$/u.exec(value);
  return match ? String.raw`${match[1]}\times 10^{${match[2]}}` : value;
}

// 只转换规则识别出的字符，不把任意输入当作 TeX 命令执行。
function fractionOperand(value: string): string {
  if (/^-?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/u.test(value)) return texNumber(value);
  const symbols: Record<string, string> = {
    "Δ": String.raw`\Delta `, "Σ": String.raw`\sum `,
    "²": "^2", "³": "^3", "−": "-", "×": String.raw`\times `,
  };
  const subscripts: Record<string, string> = { "₀": "0", "₁": "1", "₂": "2", "₃": "3", "₄": "4", "₅": "5", "₆": "6", "₇": "7", "₈": "8", "₉": "9", "ₖ": "k", "ᵢ": "i", "ₓ": "x", "ᵧ": "y", "ₙ": "n" };
  const chars = Array.from(value);
  let result = "";
  for (let i = 0; i < chars.length; i++) {
    const char = chars[i];
    if (subscripts[char]) {
      let subscript = subscripts[char];
      while (i + 1 < chars.length && subscripts[chars[i + 1]]) subscript += subscripts[chars[++i]];
      result += `_{${subscript}}`;
    } else if (char === "√") {
      let root = "";
      while (i + 1 < chars.length && /^[0-9.]$/u.test(chars[i + 1])) root += chars[++i];
      if (!root && i + 1 < chars.length) root = chars[++i];
      result += String.raw`\sqrt{${root}}`;
    } else if (symbols[char]) result += symbols[char];
    else if (/^[\p{Script=Han}]$/u.test(char)) result += String.raw`\text{${char}}`;
    else if (/[#%&_{}$\\]/u.test(char)) result += `\\${char}`;
    else result += char;
  }
  return result;
}

export function formulaTex(part: FormulaPart): string | undefined {
  if (part.tex) return part.tex;
  if (part.image) return formulaSources[part.image];
  if (part.wireAngle && part.wireNumerator) {
    return String.raw`\frac{${texNumber(part.wireNumerator)}}{2\cos\!\left(\frac{${part.wireAngle}^{\circ}}{2}\right)}`;
  }
  if (part.denominator !== undefined) {
    return String.raw`\frac{${fractionOperand(part.text)}}{${fractionOperand(part.denominator)}}`;
  }
  return undefined;
}

export function formulaParts(line: string): FormulaPart[] {
  let best: { at: number; part: FormulaPart } | undefined;
  const consider = (at: number, part: FormulaPart) => {
    if (at >= 0 && (!best || at < best.at || (at === best.at && part.text.length > best.part.text.length))) best = { at, part };
  };
  // K 型热电偶的数值代入由引擎按当前数据排版，而不是拼接预制图片。
  const gaussian = /(-?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?)\s*×\s*exp\[(-?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?)\s*×\s*\((-?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?)\s*−\s*(\([-−](?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?\)|-?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?)\)²\]/u.exec(line);
  if (gaussian) {
    const at = gaussian.index;
    const right = /^[-−]/u.test(gaussian[4]) ? String.raw`\left(${texNumber(gaussian[4])}\right)` : texNumber(gaussian[4]);
    const tex = String.raw`${texNumber(gaussian[1])}\times\exp\!\left[${texNumber(gaussian[2])}\times\left(${texNumber(gaussian[3])}-${right}\right)^2\right]`;
    consider(at, { text: gaussian[0], tex });
  }
  const helix = /atan\[(-?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?)\s*\/\s*\(π\s*×\s*(-?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?)\)\]\s*×\s*180\/π/u.exec(line);
  if (helix) {
    const at = helix.index;
    const tex = String.raw`\operatorname{atan}\left(\frac{${texNumber(helix[1])}}{\pi\times ${texNumber(helix[2])}}\right)\times\frac{180}{\pi}`;
    consider(at, { text: helix[0], tex });
  }
  let dynamic: RegExpExecArray | null = null;
  let dynamicTemplate = "";
  for (const [pattern, template] of dynamicPatterns) {
    const match = pattern.exec(line);
    if (match && (!dynamic || match.index < dynamic.index || (match.index === dynamic.index && match[0].length > dynamic[0].length))) {
      dynamic = match; dynamicTemplate = template;
    }
  }
  if (dynamic) {
    const tex = dynamicTemplate.replace(/\$(\d+)/gu, (_, index: string) => texNumber(dynamic![Number(index)]));
    consider(dynamic.index, { text: dynamic[0], tex });
  }
  const wire = /-?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?\s*\/\s*\[2cos\((30|55|60|70|75)°\/2\)\]/u.exec(line);
  if (wire && (wire.index === 0 || !/[\p{L}\p{N}_.]/u.test(line[wire.index - 1]))) {
    const at = wire.index, numerator = wire[0].split("/")[0].trim();
    consider(at, { text: wire[0], wireNumerator: numerator, wireAngle: wire[1] as FormulaPart["wireAngle"] });
  }
  for (const [formula, image] of [["3√3/8", "threadK2"], ["5√3/8", "threadK1"], ["3√3P/8", "threadK2P"], ["5√3P/8", "threadK1P"], ["3√3·P/8", "threadK2P"], ["5√3·P/8", "threadK1P"], ["atan[L/(πD₂)] × 180/π", "helix"], ["P/[2cos(α/2)]", "wire"], ["√[Σvᵢ²/(n − 1)]", "sampleSd"], ["√[Σ(xᵢ−x̄)²/(n−1)]", "repeatability"], ["a₀·exp[a₁(t−a₂)²]", "tcGaussian"], ["a₀exp[a₁(t−a₂)²]", "tcGaussian"], ["E(t) = c₀ + c₁t + c₂t² + … + cₙtⁿ", "tcPolynomial"], ["Σcᵢtⁱ", "tcSum"], ["Σc[i]·t^i", "tcSum"]] as const) {
    const at = line.indexOf(formula);
    consider(at, { text: formula, image });
  }
  for (const expression of [...additionalFormulas, ...sharedFormulaCatalog] as ReadonlyArray<readonly [string, string]>) {
    consider(line.indexOf(expression[0]), { text: expression[0], tex: expression[1] });
  }
  if (best) {
    const { at, part } = best;
    return [...formulaParts(line.slice(0, at)), part, ...formulaParts(line.slice(at + part.text.length))];
  }
  const pattern = /(\([^()；。，\/\r\n]{1,80}\)|\[[^\[\]；。，\/\r\n]{1,80}\]|-?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?|源系数|输入值|Σxᵢ|xᵢ|Δ|R|s)\s*\/\s*(\([^()；。，\/\r\n]{1,80}\)|\[[^\[\]；。，\/\r\n]{1,80}\]|√(?:n|[0-9]+(?:\.[0-9]+)?)|-?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?|目标系数|引用值|真值|跨度|Rtp|R₀|T₀|d₂|sₓ|sᵧ|T|d|n)/gu;
  const bad = (c: string) => /[\p{L}\p{N}_./√°%‰]/u.test(c);
  const result: FormulaPart[] = [];
  let end = 0;
  for (const match of line.matchAll(pattern)) {
    const start = match.index, stop = start + match[0].length;
    const numeric = (text: string) => /^\s*\(?\s*[+-]?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?\s*\)?\s*$/u.test(text);
    if (numeric(match[1]) && numeric(match[2])) continue;
    const before = Array.from(line.slice(0, start)).at(-1), after = Array.from(line.slice(stop))[0];
    if ((before && (bad(before) || ")]".includes(before))) || (after && (bad(after) || "([".includes(after)))) continue;
    if (line.slice(0, start).trimEnd().endsWith("/") || line.slice(stop).trimStart().startsWith("/")) continue;
    const clause = line.slice(0, start).split(/[；。：\n]/u).at(-1) ?? "";
    const leftHandRatio = match[1] === "R" && match[2] === "R₀" && (line.slice(stop).split(/[；。：\n]/u)[0] ?? "").includes("=");
    if (!clause.includes("=") && !leftHandRatio) continue;
    if (start > end) result.push({ text: line.slice(end, start) });
    result.push({ text: match[1], denominator: match[2] });
    end = stop;
  }
  if (end < line.length) result.push({ text: line.slice(end) });
  return result;
}
