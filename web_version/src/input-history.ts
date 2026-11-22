/** 与目录中的记忆范围保持一致；读取旧版输入时也剔除快捷换算。 */
import type { Field, State, Tool } from "./types";

export const INPUT_STORAGE_KEY = "toolbox-web:v1";
type Limits = [min: number, max: number];
type RowChange = { path: string[]; value?: unknown; limits?: Limits; previousMethod?: number };
type CellEdits = {
  base: unknown[];
  version: string;
  values: Map<string, unknown>;
  rows: Map<string, RowChange>;
};
export type InputEdit = {
  fields: Map<string, unknown>;
  cells?: Map<string, CellEdits>;
  reset?: boolean;
};
type RowIds = Map<string, string[]>;
type PressureVersion = { epoch: string; values: State };
type ToolVersions = {
  epoch: string;
  fields: Map<string, string>;
  rows: Map<string, RowIds>;
  pressure?: PressureVersion;
  rejected: Set<string>;
};
export type InputVersions = Map<string, ToolVersions>;
const pressureFields = ["pressureUnit", "low", "high", "refLow", "refHigh"];
const threadFields = ["size", "tpi", "series", "_lastSeries", "_threadChange", "_tpiAtSize"];
const newEpoch = () => Array.from(crypto.getRandomValues(new Uint32Array(4)), v => v.toString(16)).join("-");
const toolVersions = (epoch: string): ToolVersions => ({ epoch, fields: new Map(), rows: new Map(), rejected: new Set() });
const baseRowIds = (rows: unknown[]) => rows.map((_, index) => String(index));
const pathKey = (path: string[]) => JSON.stringify(path);

function storedPath(path: string): string[] {
  try {
    const names: unknown = JSON.parse(decodeURIComponent(path));
    if (Array.isArray(names) && names.every(name => typeof name === "string")) return names;
  } catch { /* 忽略损坏的路径。 */ }
  return [];
}

function appendRowId(ids: string[]) {
  const previous = ids.at(-1);
  const order = previous?.startsWith("r") ? Number(previous.slice(1, 17)) + 1 : 0;
  return `r${String(Math.max(order, Date.now())).padStart(16, "0")}-${newEpoch()}`;
}

function readJson(storage: Storage, key: string): unknown {
  const value = storage.getItem(key);
  try { return value === null ? undefined : JSON.parse(value); }
  catch { return undefined; }
}

function snapshot(storage: Storage, key: string) {
  const data = readJson(storage, key);
  if (!data || typeof data !== "object") return undefined;
  const { epoch, state } = data as { epoch?: string; state?: State };
  if (typeof epoch === "string" && state && typeof state === "object" && !Array.isArray(state))
    return { epoch, state };
}

function keysWithPrefix(storage: Storage, prefix: string): string[] {
  const keys: string[] = [];
  for (let i = 0; i < storage.length; i++) {
    const key = storage.key(i);
    if (key?.startsWith(prefix)) keys.push(key);
  }
  return keys;
}

function appendChange(storage: Storage, prefix: string, value: unknown) {
  const latest = keysWithPrefix(storage, prefix).sort().at(-1)?.slice(prefix.length);
  const name = prefix + appendRowId(latest ? [latest] : []);
  storage.setItem(name, JSON.stringify(value));
  return name;
}

const pressureTool = (tool: Tool) => tool.kind === "pressure_reference" || tool.kind === "transmitter_reference";
const pressureValues = (state: State) => Object.fromEntries(pressureFields
  .filter(name => Object.hasOwn(state, name)).map(name => [name, state[name]]));
const pressureSignature = (state: State) => JSON.stringify(pressureFields.map(name => state[name]));

function readFields(
  storage: Storage, prefix: string, entries: Map<string, unknown>, versions?: Map<string, string>,
) {
  for (const field of keysWithPrefix(storage, prefix)) {
    const edit = readJson(storage, field);
    if (!Array.isArray(edit)) continue;
    const name = field.slice(prefix.length);
    if (typeof edit[1] === "string") versions?.set(name, edit[1]);
    if (edit.length && edit[2] !== false) entries.set(name, edit[0]);
    else entries.delete(name);
  }
}

function retainedRows(before: unknown[], after: unknown[]) {
  // 对象可能有完全相同的内容，优先按编辑器保留下来的对象引用匹配。
  for (const equal of [Object.is, (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b)]) {
    let next = 0;
    const retained = before.map(row => {
      if (next === after.length || !equal(row, after[next])) return false;
      next++;
      return true;
    });
    if (next === after.length) return retained;
  }
}

function collectCells(
  before: unknown, after: unknown, path: string[], edit: CellEdits, ids: RowIds, limits?: Limits,
): boolean {
  if (Object.is(before, after)) return true;
  if (Array.isArray(before) && Array.isArray(after)) {
    const key = pathKey(path);
    const previous = ids.get(key) ?? baseRowIds(before);
    if (before.length === after.length)
      return before.every((row, i) => collectCells(row, after[i], [...path, previous[i]], edit, ids));
    if (before.length > after.length) {
      const retained = retainedRows(before, after);
      if (!retained) return false;
      previous.forEach((id, index) => {
        const rowPath = [...path, id];
        if (!retained[index]) edit.rows.set(pathKey(rowPath), { path: rowPath, limits });
      });
      ids.set(key, previous.filter((_, index) => retained[index]));
      return true;
    }
    if (!before.every((row, i) => Object.is(row, after[i]) || JSON.stringify(row) === JSON.stringify(after[i])))
      return false;
    const next = [...previous];
    for (const row of after.slice(before.length)) {
      const id = appendRowId(next);
      next.push(id);
      const rowPath = [...path, id];
      edit.rows.set(pathKey(rowPath), { path: rowPath, value: row, limits });
    }
    ids.set(key, next);
    return true;
  }
  if (before && typeof before === "object" && after && typeof after === "object" &&
      !Array.isArray(before) && !Array.isArray(after)) {
    const a = before as State;
    const b = after as State;
    const keys = Object.keys(a);
    return keys.length === Object.keys(b).length && keys.every(key => {
      if (key === "method" && Number(b.kind) === 0 && Array.isArray(b.values) && Number(a.method) !== Number(b.method)) {
        const methodPath = [...path, key];
        edit.rows.set(pathKey(methodPath), { path: methodPath, value: b.method, previousMethod: Number(a.method ?? 0) });
        return true;
      }
      let bounds: Limits | undefined;
      if (key === "children" && Number(b.kind) === 2) bounds = [1, 12];
      if (key === "values" && Number(b.kind) === 0) bounds = [2, Number(b.method) === 1 ? 9 : 50];
      return Object.hasOwn(b, key) && collectCells(a[key], b[key], [...path, key], edit, ids, bounds);
    });
  }
  if ((before !== null && typeof before === "object") || (after !== null && typeof after === "object"))
    return false;
  edit.values.set(pathKey(path), after);
  return true;
}

export function recordInputChanges(
  edit: InputEdit, before: State, after: State, versions?: ToolVersions, replace?: string | true, field?: Field,
) {
  for (const key of new Set([...Object.keys(before), ...Object.keys(after)])) {
    const a = before[key], b = after[key];
    if (key !== replace && (Object.is(a, b) || JSON.stringify(a) === JSON.stringify(b))) continue;
    if (replace !== true && key !== replace && !threadFields.includes(key) &&
        !edit.fields.has(key) && Array.isArray(a) && Array.isArray(b)) {
      const changes: CellEdits = {
        base: a,
        version: versions?.fields.get(key) ?? "0",
        values: new Map(),
        rows: new Map(),
      };
      const ids = new Map(versions?.rows.get(key));
      const limits: Limits | undefined = field?.key === key
        ? [field.min ?? 1, field.max ?? (field.type === "budget" ? 20 : 50)] : undefined;
      if (collectCells(a, b, [], changes, ids, limits)) {
        edit.cells ??= new Map();
        const prior = edit.cells.get(key) ?? changes;
        if (prior !== changes) {
          for (const [path, row] of changes.rows) {
            if (row.value === undefined) {
              const descendants = `${path.slice(0, -1)},`;
              for (const pending of [prior.rows, prior.values]) {
                for (const saved of pending.keys()) {
                  if (saved === path || saved.startsWith(descendants)) pending.delete(saved);
                }
              }
            }
            const previousMethod = prior.rows.get(path)?.previousMethod ?? row.previousMethod;
            prior.rows.set(path, { ...row, previousMethod });
          }
          for (const [path, value] of changes.values) prior.values.set(path, value);
        }
        versions?.rows.set(key, ids);
        edit.cells.set(key, prior);
        continue;
      }
    }
    edit.cells?.delete(key);
    edit.fields.set(key, b);
  }
}

function indexRows(value: unknown): unknown {
  if (Array.isArray(value)) return new Map(baseRowIds(value).map((id, i) => [id, indexRows(value[i])]));
  if (value && typeof value === "object")
    return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, indexRows(item)]));
  return value;
}

function restoreRows(value: unknown, path: string[], ids: RowIds): unknown {
  if (value instanceof Map) {
    ids.set(pathKey(path), [...value.keys()]);
    return [...value].map(([id, item]) => restoreRows(item, [...path, id], ids));
  }
  if (value && typeof value === "object")
    return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, restoreRows(item, [...path, key], ids)]));
  return value;
}

function entryAt(target: unknown, path: string[]) {
  for (const key of path) {
    target = target instanceof Map ? target.get(key)
      : target && typeof target === "object" && Object.hasOwn(target, key) ? (target as State)[key] : undefined;
  }
  return target;
}

function applyRowBatch(root: unknown, changes: RowChange[], methods?: Map<string, unknown>) {
  const touched = new Map<Map<string, unknown>, { before: Map<string, unknown>; limits?: Limits; path: string[] }>();
  const changedMethods = new Map<string, unknown>();
  const methodAt = (path: string[], node: State) => {
    const key = pathKey([...path, "method"]);
    return Number(changedMethods.get(key) ?? methods?.get(key) ?? node.method ?? 0);
  };
  const measurementLimits = (path: string[]): Limits | undefined => {
    if (!methods || path.at(-1) !== "values") return;
    const parent = path.slice(0, -1);
    const node = entryAt(root, parent) as State | undefined;
    if (node && Object.hasOwn(node, "method")) return [2, methodAt(parent, node) === 1 ? 9 : 50];
  };
  const rollback = () => {
    for (const [table, { before }] of touched) {
      table.clear();
      for (const [id, value] of before) table.set(id, value);
    }
    return false;
  };
  for (const row of changes) {
    if (!row || !Array.isArray(row.path) || !row.path.length ||
        !row.path.every(name => typeof name === "string") ||
        (row.limits !== undefined && (!Array.isArray(row.limits) || row.limits.length !== 2 ||
          !row.limits.every(n => Number.isSafeInteger(n) && n >= 0)))) return rollback();
    const id = row.path.at(-1)!;
    const parent = row.path.slice(0, -1);
    const table = entryAt(root, parent);
    if (row.previousMethod !== undefined) {
      if (!methods || id !== "method" || !table || typeof table !== "object" || !Object.hasOwn(table, "method") ||
          ![0, 1].includes(Number(row.value)) || methodAt(parent, table as State) !== row.previousMethod) return rollback();
      changedMethods.set(pathKey(row.path), row.value);
      continue;
    }
    if (!(table instanceof Map)) return rollback();
    if (!touched.has(table)) touched.set(table, { before: new Map(table), limits: row.limits, path: parent });
    if (Object.hasOwn(row, "value")) table.set(id, indexRows(row.value));
    else table.delete(id);
  }
  for (const [table, { before, limits, path }] of touched) {
    const bounds = measurementLimits(path) ?? limits;
    if (!bounds) continue;
    const [min, max] = bounds;
    if ((table.size < min && table.size < before.size) || (table.size > max && table.size > before.size))
      return rollback();
  }
  for (const [key, method] of changedMethods) {
    const path = (JSON.parse(key) as string[]).slice(0, -1);
    const values = entryAt(root, [...path, "values"]);
    if (!(values instanceof Map) || values.size < 2 || values.size > (Number(method) === 1 ? 9 : 50)) return rollback();
  }
  for (const [key, method] of changedMethods) methods?.set(key, method);
  return true;
}

function applyRowsAndCells(storage: Storage, key: string, entries: Map<string, unknown>, versions: ToolVersions) {
  const { epoch, fields, rows } = versions;
  const prefix = `${key}:cell:${epoch}:`;
  const rowPrefix = `${key}:row:${epoch}:`;
  const batchPrefix = `${key}:row-batch:${epoch}:`;
  const rowEdits = keysWithPrefix(storage, rowPrefix).sort();
  const rowBatches = keysWithPrefix(storage, batchPrefix).sort();
  const cellEdits = keysWithPrefix(storage, prefix);
  for (const [basePrefix, names] of [[rowPrefix, rowEdits], [batchPrefix, rowBatches], [prefix, cellEdits]] as const) {
    for (const name of names) {
      const [field, version] = name.slice(basePrefix.length).split(":");
      if (!field || version !== (fields.get(field) ?? "0") || entries.has(field)) continue;
      const base = readJson(storage, `${key}:cell-base:${epoch}:${field}:${version}`);
      if (Array.isArray(base)) entries.set(field, base);
    }
  }
  const tables = new Map<string, unknown>();
  for (const [key, value] of entries) {
    if (Array.isArray(value)) tables.set(key, indexRows(value));
  }
  const operations = rowEdits.map(row => {
    const [field, version, name] = row.slice(rowPrefix.length).split(":");
    const path = name?.startsWith("%") ? storedPath(name) : name ? [name] : [];
    return { row, field, version, path };
  }).sort((a, b) => a.path.length - b.path.length);
  for (const { row, field, version, path } of operations) {
    const id = path.at(-1);
    if (!id || version !== (fields.get(field) ?? "0")) continue;
    const table = entryAt(tables.get(field), path.slice(0, -1));
    const edit = readJson(storage, row);
    if (!(table instanceof Map) || !Array.isArray(edit)) continue;
    if (edit.length) table.set(id, indexRows(edit[0]));
    else table.delete(id);
  }
  const methods = new Map<string, Map<string, unknown>>();
  // 旧版本把方法写成单独的单元格。读取它作为按批次重放时的初始方法。
  for (const cell of cellEdits) {
    const [field, version, path] = cell.slice(prefix.length).split(":");
    if (!path || version !== (fields.get(field) ?? "0")) continue;
    const names = storedPath(path);
    if (names.at(-1) !== "method") continue;
    const edit = readJson(storage, cell);
    if (!Array.isArray(edit) || !edit.length) continue;
    const initial = methods.get(field) ?? new Map();
    initial.set(pathKey(names), edit[0]);
    methods.set(field, initial);
  }
  // 同一批的方法和测量值一起生效，按最新方法校验合并后的数量。
  for (const row of rowBatches) {
    const [field, version] = row.slice(batchPrefix.length).split(":");
    if (version !== (fields.get(field) ?? "0")) continue;
    const stored = readJson(storage, row);
    const changes = Array.isArray(stored) ? stored : (stored as { changes?: unknown } | undefined)?.changes;
    const current = methods.get(field) ?? new Map();
    if (!Array.isArray(changes) || !applyRowBatch(tables.get(field), changes, Array.isArray(stored) ? undefined : current)) versions.rejected.add(row);
    else methods.set(field, current);
  }
  for (const cell of cellEdits) {
    const [field, version, path] = cell.slice(prefix.length).split(":");
    if (!path || version !== (fields.get(field) ?? "0")) continue;
    const names = storedPath(path);
    const name = names.at(-1);
    const edit = readJson(storage, cell);
    if (name === undefined || !Array.isArray(edit) || !edit.length) continue;
    if (name === "method") continue;
    const target = entryAt(tables.get(field), names.slice(0, -1));
    if (target instanceof Map) {
      if (target.has(name)) target.set(name, edit[0]);
    } else if (target && typeof target === "object" && Object.hasOwn(target, name))
      (target as State)[name] = edit[0];
  }
  for (const [field, values] of methods) {
    for (const [key, method] of values) {
      const node = entryAt(tables.get(field), (JSON.parse(key) as string[]).slice(0, -1));
      if (node && typeof node === "object" && Object.hasOwn(node, "method")) (node as State).method = method;
    }
  }
  for (const [field, table] of tables) {
    const ids: RowIds = new Map();
    entries.set(field, restoreRows(table, [], ids));
    rows.set(field, ids);
  }
}

function applyGroup(entries: Map<string, unknown>, fields: string[], state: State) {
  for (const name of fields) {
    if (Object.hasOwn(state, name)) entries.set(name, state[name]);
    else entries.delete(name);
  }
}

type PressureChange = { epoch: string; fields: [string, unknown?][]; before?: string };

function readPressure(storage: Storage, key: string, entries: Map<string, unknown>, versions: ToolVersions) {
  const base = snapshot(storage, `${key}:pressure:${versions.epoch}`);
  let epoch = base?.epoch ?? "0";
  if (base) {
    applyGroup(entries, pressureFields, base.state);
    readFields(storage, `${key}:pressure-field:${versions.epoch}:${epoch}:`, entries);
  }
  const prefix = `${key}:pressure-change:${versions.epoch}:`;
  for (const name of keysWithPrefix(storage, prefix).sort()) {
    const change = readJson(storage, name) as PressureChange | undefined;
    const before = pressureSignature(Object.fromEntries(entries));
    if (!change || !Array.isArray(change.fields) ||
        !change.fields.every(field => Array.isArray(field) && pressureFields.includes(field[0])) || change.epoch !== epoch ||
        (change.before !== undefined && change.before !== before)) {
      versions.rejected.add(name);
      continue;
    }
    if (change.before !== undefined) {
      epoch = name.slice(prefix.length);
      applyGroup(entries, pressureFields, Object.fromEntries(change.fields));
    } else {
      for (const [field, value] of change.fields) {
        if (!pressureFields.includes(field)) continue;
        if (value === undefined) entries.delete(field);
        else entries.set(field, value);
      }
    }
  }
  versions.pressure = { epoch, values: pressureValues(Object.fromEntries(entries)) };
}

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

export function readRememberedInputs(
  tools: Tool[], storage: Storage, inputVersions?: InputVersions,
): Record<string, State> {
  const result = rememberedInputs(tools, readJson(storage, INPUT_STORAGE_KEY));
  for (const tool of tools) {
    if (!tool.rememberInputs) continue;
    const key = `${INPUT_STORAGE_KEY}:${tool.id}`;
    const reset = snapshot(storage, `${key}:reset`);
    const legacy = readJson(storage, key);
    const state = reset?.state ??
      (legacy && typeof legacy === "object" && !Array.isArray(legacy) ? legacy : result[tool.id]);
    const epoch = reset?.epoch ?? "0";
    const entries = new Map(Object.entries(state ?? {}));
    const versions = toolVersions(epoch);
    readFields(storage, `${key}:field:${epoch}:`, entries, versions.fields);
    applyRowsAndCells(storage, key, entries, versions);
    inputVersions?.set(tool.id, versions);
    if (pressureTool(tool)) readPressure(storage, key, entries, versions);
    const thread = snapshot(storage, `${key}:thread:${epoch}`);
    if (thread) applyGroup(entries, threadFields, thread.state);
    if (entries.size || state) result[tool.id] = Object.fromEntries(entries);
  }
  return result;
}

export function saveChangedInputs(
  tools: Tool[],
  states: Record<string, State>,
  edited: Map<string, InputEdit>,
  storage: Storage,
  inputVersions: InputVersions = new Map(),
) {
  const conflicts = new Set<string>();
  for (const [id, edit] of edited) {
    const tool = tools.find((tool) => tool.id === id && tool.rememberInputs);
    if (tool) {
      const key = `${INPUT_STORAGE_KEY}:${id}`;
      const activeEpoch = () => snapshot(storage, `${key}:reset`)?.epoch ?? "0";
      const previous = activeEpoch();
      const isActive = () => activeEpoch() === previous;
      const state = states[id] ?? {};
      const inspect = () => {
        const latest: InputVersions = new Map();
        readRememberedInputs([tool], storage, latest);
        return latest.get(id)!;
      };
      const versions = inputVersions.get(id) ?? inspect();
      inputVersions.set(id, versions);
      if (edit.reset) {
        const obsolete = [
          ...keysWithPrefix(storage, `${key}:field:${previous}:`),
          ...keysWithPrefix(storage, `${key}:cell:${previous}:`),
          ...keysWithPrefix(storage, `${key}:row:${previous}:`),
          ...keysWithPrefix(storage, `${key}:row-batch:${previous}:`),
          ...keysWithPrefix(storage, `${key}:cell-base:${previous}:`),
          ...keysWithPrefix(storage, `${key}:pressure-field:${previous}:`),
          ...keysWithPrefix(storage, `${key}:pressure-change:${previous}:`),
          `${key}:pressure:${previous}`, `${key}:thread:${previous}`,
        ];
        const epoch = newEpoch();
        storage.setItem(`${key}:reset`, JSON.stringify({ epoch, state }));
        const next = toolVersions(epoch);
        if (pressureTool(tool)) next.pressure = { epoch: "0", values: pressureValues(state) };
        inputVersions.set(id, next);
        edited.delete(id);
        for (const field of obsolete) storage.removeItem(field);
      } else if (versions.epoch !== previous) {
        conflicts.add(id);
        continue;
      } else {
        const saveGroup = (key: string, fields: string[], epoch: string) => {
          const values = Object.fromEntries(fields.filter(name => Object.hasOwn(state, name)).map(name => [name, state[name]]));
          storage.setItem(key, JSON.stringify({ epoch, state: values }));
          if (isActive()) for (const field of fields) edit.fields.delete(field);
        };
        if (id === "gauge_unified" && threadFields.some(field => edit.fields.has(field)))
          saveGroup(`${key}:thread:${previous}`, threadFields, previous);

        const pressureFieldsEdited = pressureTool(tool) ? pressureFields.filter(field => edit.fields.has(field)) : [];
        if (pressureFieldsEdited.length) {
          const latest = inspect().pressure!;
          const original = versions.pressure ?? latest;
          const changesUnit = edit.fields.has("pressureUnit");
          const before = changesUnit ? pressureSignature(original.values) : undefined;
          const storedUnit = latest.values.pressureUnit;
          if (latest.epoch !== original.epoch || (changesUnit && before !== pressureSignature(latest.values)) ||
              (!changesUnit && storedUnit !== undefined && Number(storedUnit) !== Number(state.pressureUnit))) {
            conflicts.add(id);
          } else {
            const fields: PressureChange["fields"] = changesUnit ? Object.entries(pressureValues(state))
              : pressureFieldsEdited.map(field => edit.fields.get(field) === undefined ? [field] : [field, edit.fields.get(field)]);
            const prefix = `${key}:pressure-change:${previous}:`;
            const name = appendChange(storage, prefix, { epoch: original.epoch, fields, before });
            const saved = inspect();
            if (saved.epoch !== previous || saved.rejected.has(name) ||
                saved.pressure?.epoch !== (changesUnit ? name.slice(prefix.length) : original.epoch)) {
              conflicts.add(id);
            } else {
              const values = changesUnit ? {} : { ...original.values };
              for (const [field, value] of fields) {
                if (value === undefined) delete values[field];
                else values[field] = value;
              }
              versions.pressure = { epoch: saved.pressure.epoch, values };
              for (const field of pressureFieldsEdited) edit.fields.delete(field);
            }
          }
        }
        // 每个字段独立写入，不依赖跨标签页的读改写锁。
        for (const [field, value] of edit.fields) {
          if (pressureFieldsEdited.includes(field) || !isActive()) continue;
          const prefix = `${key}:field:${previous}:`;
          const old = readJson(storage, prefix + field);
          const obsolete = Array.isArray(old) && typeof old[1] === "string" ? old[1] : "0";
          const epoch = newEpoch();
          storage.setItem(prefix + field, JSON.stringify(value === undefined ? [null, epoch, false] : [value, epoch]));
          if (!isActive()) continue;
          versions.fields.set(field, epoch);
          versions.rows.delete(field);
          edit.fields.delete(field);
          for (const name of [
            ...keysWithPrefix(storage, `${key}:cell:${previous}:${field}:${obsolete}:`),
            ...keysWithPrefix(storage, `${key}:row:${previous}:${field}:${obsolete}:`),
            ...keysWithPrefix(storage, `${key}:row-batch:${previous}:${field}:${obsolete}:`),
            `${key}:cell-base:${previous}:${field}:${obsolete}`,
          ]) storage.removeItem(name);
        }
        for (const [field, cellEdit] of edit.cells ?? []) {
          const isCurrent = () => {
            const stored = readJson(storage, `${key}:field:${previous}:${field}`);
            const latest = Array.isArray(stored) && typeof stored[1] === "string" ? stored[1] : "0";
            return isActive() && latest === cellEdit.version;
          };
          if (versions.epoch !== previous || !isCurrent()) {
            conflicts.add(id);
            continue;
          }
          const cellPrefix = `${previous}:${field}:${cellEdit.version}`;
          const baseKey = `${key}:cell-base:${cellPrefix}`;
          if (storage.getItem(baseKey) === null)
            storage.setItem(baseKey, JSON.stringify(cellEdit.base));
          let latest = inspect();
          if (cellEdit.rows.size) {
            const name = appendChange(storage, `${key}:row-batch:${cellPrefix}:`, { changes: [...cellEdit.rows.values()] });
            latest = inspect();
            if (isCurrent() && !latest.rejected.has(name)) cellEdit.rows.clear();
            else if (latest.rejected.has(name)) storage.removeItem(name);
          }
          const removed = (names: string[], current: ToolVersions) => names.some((name, i) => {
            const ids = current.rows.get(field)?.get(pathKey(names.slice(0, i)));
            return ids !== undefined && !ids.includes(name);
          });
          const written = new Map<string, string[]>();
          for (const [path, value] of cellEdit.values) {
            const names = JSON.parse(path) as string[];
            if (removed(names, latest) || !isCurrent()) continue;
            storage.setItem(`${key}:cell:${cellPrefix}:${encodeURIComponent(path)}`, JSON.stringify([value]));
            written.set(path, names);
          }
          if (written.size && isCurrent()) {
            latest = inspect();
            for (const [path, names] of written) {
              if (!removed(names, latest)) cellEdit.values.delete(path);
            }
          }
          if (isCurrent() && !cellEdit.rows.size && !cellEdit.values.size) edit.cells?.delete(field);
          else conflicts.add(id);
        }
        if (!isActive()) conflicts.add(id);
      }
    }
    if (!edit.fields.size && !edit.cells?.size) edited.delete(id);
  }
  return conflicts;
}
