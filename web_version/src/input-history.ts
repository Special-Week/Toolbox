/** 与目录中的记忆范围保持一致；读取旧版输入时也剔除快捷换算。 */
import type { State, Tool } from "./types";

export const INPUT_STORAGE_KEY = "toolbox-web:v1";

export function rememberedInputs(
  tools: Tool[],
  input: unknown,
): Record<string, State> {
  const result: Record<string, State> = {};
  if (!input || typeof input !== "object" || Array.isArray(input))
    return result;
  for (const tool of tools) {
    if (!tool.rememberInputs) continue;
    const value = (input as Record<string, unknown>)[tool.id];
    if (value && typeof value === "object" && !Array.isArray(value))
      result[tool.id] = value as State;
  }
  return result;
}

export function readRememberedInputs(tools: Tool[], storage: Storage): Record<string, State> {
  const read = (key: string) => {
    const value = storage.getItem(key);
    try { return value === null ? undefined : JSON.parse(value); }
    catch { return undefined; }
  };
  const result = rememberedInputs(tools, read(INPUT_STORAGE_KEY));
  for (const tool of tools) {
    if (!tool.rememberInputs) continue;
    const state = read(`${INPUT_STORAGE_KEY}:${tool.id}`);
    if (state && typeof state === "object" && !Array.isArray(state)) result[tool.id] = state;
  }
  return result;
}

export function saveChangedInputs(
  tools: Tool[],
  states: Record<string, State>,
  saved: Map<string, string>,
  storage: Storage,
) {
  for (const [id, state] of Object.entries(rememberedInputs(tools, states))) {
    const text = JSON.stringify(state);
    if (text === saved.get(id)) continue;
    storage.setItem(`${INPUT_STORAGE_KEY}:${id}`, text);
    saved.set(id, text);
  }
}
