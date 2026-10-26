/** 共用数字输入的导航逻辑，保留正在编辑的字符串，不参与数值计算。 */
export function nextNumeric(
  input: HTMLInputElement,
): HTMLInputElement | undefined {
  const fields = Array.from(
    document.querySelectorAll<HTMLInputElement>(
      ".input-panels input[data-numeric]:not(:disabled)",
    ),
  ).filter((field) => field.getClientRects().length > 0);
  const index = fields.indexOf(input);
  return index >= 0 ? fields[index + 1] : undefined;
}

export function advanceNumeric(input: HTMLInputElement) {
  const next = nextNumeric(input);
  if (next) {
    next.focus({ preventScroll: true });
    next.scrollIntoView({ block: "center", behavior: "auto" });
  } else input.blur();
}

export function toggleSign(input: HTMLInputElement) {
  const text = input.value.trim();
  const negative = text.startsWith("-");
  const positive = text.startsWith("+");
  const cursor = input.selectionStart ?? text.length;
  input.value = negative
    ? text.slice(1)
    : "-" + (positive ? text.slice(1) : text);
  input.dispatchEvent(new Event("input", { bubbles: true }));
  requestAnimationFrame(() => {
    if (!input.isConnected) return;
    input.focus({ preventScroll: true });
    const at = Math.max(0, cursor + (negative ? -1 : positive ? 0 : 1));
    input.setSelectionRange(at, at);
  });
}
