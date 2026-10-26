/** 展开指数记数法，结果显示和 CSV 均不出现 e-6 一类文本。 */
export function decimal(value: number): string {
  if (!Number.isFinite(value)) return "—";
  if (Object.is(value, -0)) return "0";
  const text = String(value);
  if (!/[eE]/.test(text)) return text;
  const [mantissa, exponent] = text.toLowerCase().split("e");
  const negative = mantissa.startsWith("-");
  const unsigned = negative ? mantissa.slice(1) : mantissa;
  const [integer, fraction = ""] = unsigned.split(".");
  const digits = integer + fraction;
  const at = integer.length + Number(exponent);
  return (
    (negative ? "-" : "") +
    (at <= 0
      ? "0." + "0".repeat(-at) + digits
      : at >= digits.length
        ? digits + "0".repeat(at - digits.length)
        : digits.slice(0, at) + "." + digits.slice(at))
  );
}
export function display(value: unknown): string {
  if (value === null || value === undefined) return "未定义";
  if (typeof value !== "number") return String(value);
  // 十一位有效数字用于阅读；复制和导出使用未经显示舍入的原始值。
  return decimal(Number(value.toPrecision(11)));
}
export function csvCell(value: unknown): string {
  const text =
    typeof value === "number"
      ? decimal(value)
      : value == null
        ? ""
        : String(value);
  return /[",\r\n]/.test(text) ? '"' + text.replaceAll('"', '""') + '"' : text;
}
export function download(blob: Blob, name: string) {
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 30_000);
}
