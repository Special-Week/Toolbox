/** 与目录中的记忆范围保持一致；读取旧版输入时也剔除快捷换算。 */
import type { State, Tool } from "./types";

export const INPUT_STORAGE_KEY = "toolbox-web:v1";

export function rememberedInputs(tools: Tool[], input: unknown): Record<string, State> {
  const result: Record<string, State> = {};
  if (!input || typeof input !== "object" || Array.isArray(input)) return result;
  for (const tool of tools) {
    if (!tool.rememberInputs) continue;
    const value = (input as Record<string, unknown>)[tool.id];
    if (value && typeof value === "object" && !Array.isArray(value))
      result[tool.id] = value as State;
  }
  return result;
}
