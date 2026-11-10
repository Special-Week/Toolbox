import { schema } from "./schema";
import type { Catalog, Field, State, Tool } from "./types";

export const ARCHIVE_STORAGE_KEY = "toolbox-web:calibrations:v1";
const allowed: Record<string, Set<string>> = {
  standard_couple: new Set(["type", "source", "zone", "coefficientSource", "fixed", "coefficients", "points"]),
  sprt: new Set(["model", "highRange", "lowRange", "deviation", "source", "rtp", "a", "b", "c", "a4", "b4", "d", "w660", "highFixed", "lowFixed", "silver", "r0", "points"]),
};
export interface CalibrationArchive { tool: string; name: string; fields: State }
const object = (value: unknown): value is Record<string, unknown> =>
  !!value && typeof value === "object" && !Array.isArray(value);
const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value));

function number(value: unknown): boolean {
  if (typeof value !== "number" && typeof value !== "string") return false;
  const text = String(value).trim();
  return /^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/.test(text) && Number.isFinite(Number(text));
}
function validField(field: Field, value: unknown): boolean {
  if (field.type === "select") return field.options?.some(option => option.value === String(value)) ?? false;
  if (field.type === "number") return number(value);
  if (field.type === "vector" || field.type === "points") {
    if (!Array.isArray(value) || value.length < (field.min ?? 1) || value.length > (field.max ?? 50)) return false;
    return value.every(row => field.type === "points"
      ? Array.isArray(row) && row.length === 2 && row.every(number)
      : number(row));
  }
  return false;
}
export function calibrationFields(tool: Tool, state: State, catalog: Catalog): State {
  const keys = allowed[tool.kind];
  if (!keys) throw new Error("该工具不支持系数档案。");
  let fields: Field[];
  try { fields = schema(tool, state, catalog).flatMap(section => section.fields).filter(field => keys.has(field.key)); }
  catch { throw new Error("档案的模型或温区无效。"); }
  const snapshot: State = {};
  for (const field of fields) {
    const value = state[field.key];
    if (!validField(field, value)) throw new Error(`请检查${field.label}，填写完整的证书参数。`);
    snapshot[field.key] = clone(value);
  }
  return snapshot;
}
export function readCalibrationArchives(storage: Storage, catalog: Catalog): CalibrationArchive[] {
  let text: string | null;
  try { text = storage.getItem(ARCHIVE_STORAGE_KEY); }
  catch { throw new Error("无法读取浏览器本地档案。"); }
  if (text === null) return [];
  if (text.length > 8 * 1024 * 1024) throw new Error("系数档案过大，未覆盖原数据。");
  try {
    const data: unknown = JSON.parse(text);
    if (!object(data) || data.version !== 1 || !Array.isArray(data.archives)) throw new Error();
    const seen = new Set<string>();
    return data.archives.map((item: unknown) => {
      if (!object(item) || typeof item.tool !== "string" || typeof item.name !== "string" || !item.name.trim() || [...item.name].length > 100 || !object(item.fields)) throw new Error();
      const tool = catalog.tools.find(tool => tool.id === item.tool && allowed[tool.kind]);
      if (!tool) throw new Error();
      const fields = calibrationFields(tool, item.fields, catalog);
      if (Object.keys(fields).length !== Object.keys(item.fields).length) throw new Error();
      const key = JSON.stringify([item.tool, item.name]);
      if (seen.has(key)) throw new Error();
      seen.add(key);
      return { tool: item.tool, name: item.name, fields };
    });
  } catch { throw new Error("系数档案格式错误或数据不完整，未覆盖原数据。"); }
}
export function writeCalibrationArchives(storage: Storage, entries: CalibrationArchive[]): void {
  const text = JSON.stringify({ version: 1, archives: entries });
  if (text.length > 8 * 1024 * 1024) throw new Error("系数档案过大，未覆盖原数据。");
  try { storage.setItem(ARCHIVE_STORAGE_KEY, text); }
  catch { throw new Error("无法保存到浏览器本地存储，请检查存储空间和权限。"); }
}
export function applyCalibration(tool: Tool, state: State, archive: CalibrationArchive): void {
  if (archive.tool !== tool.id || !allowed[tool.kind]) throw new Error("该档案不适用于当前工具。");
  for (const key of allowed[tool.kind]) delete state[key];
  Object.assign(state, clone(archive.fields));
}
