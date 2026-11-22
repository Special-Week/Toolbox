<script setup lang="ts">
// 应用入口负责现代导航外壳（双主题、Ctrl+K 快搜、多标签栏、收藏/最近）、输入状态、计算结果及导出。
import GaugeTool from "./components/GaugeTool.vue";
import PdfDownload from "./components/PdfDownload.vue";
import NumericInput from "./components/NumericInput.vue";
import MobileActions from "./components/MobileActions.vue";
import PwaStatus from "./components/PwaStatus.vue";
import {
  computed,
  nextTick,
  onMounted,
  onUnmounted,
  reactive,
  ref,
  watch,
} from "vue";
import type { Catalog, Result, State, Tool } from "./types";
import { changePressureUnit, Engine } from "./engine";
import { schema, initialize, adjust } from "./schema";
import { decimal, display, download } from "./format";
import { calculationMetadata, resultCsv } from "./export";
import { calculationNote } from "./notes";
import CalculationProcess from "./components/CalculationProcess.vue";
import FormulaText from "./components/FormulaText.vue";
import { version } from "../package.json";
import { readRememberedInputs, saveChangedInputs } from "./input-history";
import FieldEditor from "./components/FieldEditor.vue";
import QuickUncertainty from "./components/QuickUncertainty.vue";
import ResultantComponents from "./components/ResultantComponents.vue";
import CalibrationArchives from "./components/CalibrationArchives.vue";
import {
  applyCalibration,
  type CalibrationArchive,
} from "./calibration-archives";
import CurveChart from "./components/CurveChart.vue";
import ResultTable from "./components/ResultTable.vue";
import BudgetResult, { type Detail } from "./components/BudgetResult.vue";
import {
  applyTheme,
  groupIconPath,
  groupKey,
  groupShortName,
  loadUiPrefs,
  saveUiPrefs,
  type ThemeMode,
} from "./ui-prefs";

const engine = new Engine();
const catalog = ref<Catalog>();
const loading = ref(true);
const loadError = ref("");
const selected = ref<Tool | null>(null);
const pdfDownloader = ref<InstanceType<typeof PdfDownload>>();
const inputPanel = ref<HTMLElement>();
const resultPanel = ref<HTMLElement>();
let inputScroll: number | null = null;

// 界面偏好（深浅主题、收藏、最近使用）与多工具标签栏
const initialPrefs = loadUiPrefs();
const theme = ref<ThemeMode>(initialPrefs.theme);
const favorites = ref<string[]>([...initialPrefs.favorites]);
const recentIds = ref<string[]>([...initialPrefs.recent]);
const openTabIds = ref<string[]>([]);
const sidebarFilter = ref("");

// Ctrl+K 全局命令面板
const paletteOpen = ref(false);
const paletteQuery = ref("");
const paletteIndex = ref(0);
const paletteInput = ref<HTMLInputElement>();

// 右下角 Toast 队列
interface ToastItem {
  id: number;
  text: string;
}
const toasts = ref<ToastItem[]>([]);
let toastSeq = 0;
function showToast(text: string) {
  const id = ++toastSeq;
  toasts.value.push({ id, text });
  if (toasts.value.length > 3) toasts.value.shift();
  setTimeout(() => {
    toasts.value = toasts.value.filter((t) => t.id !== id);
  }, 2400);
}

function persistPrefs() {
  saveUiPrefs({
    theme: theme.value,
    favorites: favorites.value,
    recent: recentIds.value,
  });
}

function toggleTheme() {
  theme.value = theme.value === "light" ? "dark" : "light";
  applyTheme(theme.value);
  persistPrefs();
}

function isFavorite(id: string): boolean {
  return favorites.value.includes(id);
}

function toggleFavorite(tool: Tool) {
  const idx = favorites.value.indexOf(tool.id);
  if (idx >= 0) {
    favorites.value.splice(idx, 1);
    showToast(`已取消收藏：${tool.name}`);
  } else {
    favorites.value.unshift(tool.id);
    showToast(`已收藏：${tool.name}`);
  }
  persistPrefs();
}

function recordRecent(id: string) {
  recentIds.value = [id, ...recentIds.value.filter((x) => x !== id)].slice(0, 8);
  persistPrefs();
}

function openPalette() {
  paletteOpen.value = true;
  paletteQuery.value = "";
  paletteIndex.value = 0;
  nextTick(() => paletteInput.value?.focus());
}

function closePalette() {
  paletteOpen.value = false;
}

const paletteMatches = computed(() => {
  const q = paletteQuery.value.trim().toLocaleLowerCase("zh-CN");
  const all = catalog.value?.tools ?? [];
  if (!q) {
    // 优先展示收藏与最近使用，再补齐其余工具
    const priorityIds = [...new Set([...favorites.value, ...recentIds.value])];
    const priority = priorityIds
      .map((id) => all.find((t) => t.id === id))
      .filter((t): t is Tool => !!t);
    const rest = all.filter((t) => !priorityIds.includes(t.id));
    return [...priority, ...rest].slice(0, 12);
  }
  return all
    .filter((t) =>
      [t.name, t.group, t.description, t.id].some((s) =>
        s.toLocaleLowerCase("zh-CN").includes(q),
      ),
    )
    .slice(0, 14);
});

watch(paletteQuery, () => {
  paletteIndex.value = 0;
});

function onPaletteKey(e: KeyboardEvent) {
  const list = paletteMatches.value;
  if (e.key === "Escape") {
    e.preventDefault();
    closePalette();
  } else if (e.key === "ArrowDown" && list.length) {
    e.preventDefault();
    paletteIndex.value = (paletteIndex.value + 1) % list.length;
  } else if (e.key === "ArrowUp" && list.length) {
    e.preventDefault();
    paletteIndex.value = (paletteIndex.value - 1 + list.length) % list.length;
  } else if (e.key === "Enter" && list.length) {
    e.preventDefault();
    const target = list[paletteIndex.value] ?? list[0];
    if (target) {
      closePalette();
      open(target);
    }
  }
}

function onGlobalKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
    e.preventDefault();
    if (paletteOpen.value) closePalette();
    else openPalette();
  }
}

function jumpResults() {
  inputScroll = window.scrollY;
  (
    resultPanel.value ??
    document.querySelector<HTMLElement>(".gauge-tool .gauge-summary")
  )?.scrollIntoView({
    block: "start",
    behavior: navigationBehavior(),
  });
}
function navigationBehavior(): ScrollBehavior {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches
    ? "auto"
    : "smooth";
}
function jumpInputs() {
  if (inputScroll !== null)
    window.scrollTo({ top: inputScroll, behavior: navigationBehavior() });
  else
    (
      inputPanel.value ?? document.querySelector<HTMLElement>(".gauge-tool")
    )?.scrollIntoView({
      block: "start",
      behavior: navigationBehavior(),
    });
}

// 简要说明与公式共用当前模型的文案，复制和导出沿用同一口径。
const activeNote = computed(() =>
  selected.value
    ? calculationNote(selected.value, current.value)
    : { note: "", formula: "" },
);
const explanationParagraphs = computed(() =>
  activeNote.value.note
    .split(/\r?\n/u)
    .map((text) => text.trim())
    .filter(Boolean),
);
const filter = ref("全部工具");
const toolSearch = ref("");
const mobileNav = ref(false);
const collapsed = ref(false);
const expanded = reactive(new Set<string>());
const states = reactive<Record<string, State>>({});
const result = ref<Result | null>(null);
let resultSnapshot: { tool: string; input: State } | null = null;
const busy = ref(false);
const error = ref("");
const message = ref("");
const exporting = ref(false);
const progress = ref(0);
let cancelled = false;

const groups = computed(() => [
  ...new Set(catalog.value?.tools.map((t) => t.group) ?? []),
]);
const favoriteTools = computed(() =>
  favorites.value
    .map((id) => catalog.value?.tools.find((t) => t.id === id))
    .filter((t): t is Tool => !!t),
);
const quickAccessTools = computed(() => {
  const ids = [...new Set([...favorites.value, ...recentIds.value])].slice(0, 8);
  return ids
    .map((id) => catalog.value?.tools.find((t) => t.id === id))
    .filter((t): t is Tool => !!t);
});
const openTabs = computed(() =>
  openTabIds.value
    .map((id) => catalog.value?.tools.find((t) => t.id === id))
    .filter((t): t is Tool => !!t),
);

function sidebarToolsInGroup(group: string): Tool[] {
  const q = sidebarFilter.value.trim().toLocaleLowerCase("zh-CN");
  return (
    catalog.value?.tools.filter(
      (t) =>
        t.group === group &&
        (!q ||
          t.name.toLocaleLowerCase("zh-CN").includes(q) ||
          t.description.toLocaleLowerCase("zh-CN").includes(q)),
    ) ?? []
  );
}

const current = computed<State>(() =>
  selected.value ? (states[selected.value.id] ?? {}) : {},
);
const sections = computed(() =>
  selected.value && catalog.value
    ? schema(selected.value, current.value, catalog.value)
    : [],
);
const filtered = computed(() => {
  const query = toolSearch.value.trim().toLocaleLowerCase("zh-CN");
  return (
    catalog.value?.tools.filter(
      (t) =>
        (filter.value === "全部工具" || t.group === filter.value) &&
        (!query ||
          [t.name, t.group, t.description].some((text) =>
            text.toLocaleLowerCase("zh-CN").includes(query),
          )),
    ) ?? []
  );
});
const totalUnits = computed(
  () =>
    catalog.value?.tools.reduce((n, t) => n + (t.units?.length ?? 0), 0) ?? 0,
);
const divisions = computed(
  () =>
    ["thermocouple", "rtd", "standard_couple", "sprt", "thermistor"].includes(
      selected.value?.kind ?? "",
    ) && !(selected.value?.kind === "sprt" && current.value.model === "1"),
);
const axisX = computed(() =>
  selected.value?.kind === "fit"
    ? `${current.value.xName || "输入量"}${current.value.xUnit ? " (" + current.value.xUnit + ")" : ""}`
    : divisions.value
      ? `温度 (${["°C", "K", "°F"][Number(current.value.unit ?? 0)]})`
      : "x",
);
const axisY = computed(() =>
  selected.value?.kind === "fit"
    ? `${current.value.yName || "响应量"}${current.value.yUnit ? " (" + current.value.yUnit + ")" : ""}`
    : divisions.value
      ? ["thermocouple", "standard_couple"].includes(selected.value?.kind ?? "")
        ? "热电势 (mV)"
        : "电阻 (Ω)"
      : "y",
);
const budget = computed(
  () => (result.value?.details?.budget ?? []) as Detail[],
);
const coefficients = computed(
  () =>
    result.value?.details?.coefficients as
      | Record<string, number | number[]>
      | undefined,
);
const calibrationRows = computed(
  () => (result.value?.details?.calibrationResiduals ?? []) as unknown[][],
);
const equation = computed(() => {
  if (!result.value?.equation) return "";
  const terms = result.value.coefficients
    .map((value, index) => ({ value, index }))
    .reverse()
    .filter((t) => t.value !== 0);
  return (
    terms
      .map(
        (t, j) =>
          `${t.value < 0 ? "−" : j ? "+" : ""}${display(Math.abs(t.value))}${t.index === 0 ? "" : t.index === 1 ? "·x" : `·x^${t.index}`}`,
      )
      .join(" ") || "0"
  );
});
const logo = import.meta.env.BASE_URL + "toolbox.svg";
let timer: ReturnType<typeof setTimeout> | undefined;
let saveTimer: ReturnType<typeof setTimeout> | undefined;
const savedInputs = new Map<string, string>();
let pressureChange = 0;
let revision = 0;
const inputGeneration = ref(0);

function open(tool: Tool | null, hash = true) {
  if (tool && selected.value?.id === tool.id) {
    mobileNav.value = false;
    window.scrollTo({ top: 0, behavior: "instant" });
    return;
  }
  ++pressureChange;
  selected.value = tool;
  inputScroll = null;
  result.value = null;
  resultSnapshot = null;
  error.value = "";
  message.value = "";
  mobileNav.value = false;
  if (tool && catalog.value) {
    if (!tool.rememberInputs) states[tool.id] = {};
    else states[tool.id] ??= {};
    initialize(tool, states[tool.id], catalog.value);
    expanded.add(tool.group);
    if (!openTabIds.value.includes(tool.id)) {
      openTabIds.value.push(tool.id);
    }
    recordRecent(tool.id);
    document.title = `${tool.name} · 工具箱`;
  } else document.title = "工具箱 · 计量与工程计算";
  if (hash)
    history.replaceState(
      null,
      "",
      tool
        ? "#tool/" + encodeURIComponent(tool.id)
        : location.pathname + location.search,
    );
  window.scrollTo({ top: 0, behavior: "instant" });
}

function closeTab(id: string) {
  const idx = openTabIds.value.indexOf(id);
  if (idx < 0) return;
  openTabIds.value.splice(idx, 1);
  if (selected.value?.id === id) {
    const nextId = openTabIds.value[idx] ?? openTabIds.value[idx - 1];
    const nextTool = nextId
      ? (catalog.value?.tools.find((t) => t.id === nextId) ?? null)
      : null;
    open(nextTool);
  }
}

function route() {
  const id = decodeURIComponent(location.hash.replace(/^#tool\//, ""));
  open(catalog.value?.tools.find((t) => t.id === id) ?? null, false);
}
function chooseGroup(group: string) {
  filter.value = group;
  open(null);
  mobileNav.value = false;
}
async function change(key: string, value: unknown) {
  if (!selected.value || !catalog.value) return;
  if (
    key === "pressureUnit" &&
    ["pressure_reference", "transmitter_reference", "transmitter"].includes(
      selected.value.kind,
    )
  ) {
    const state = current.value;
    const request = ++pressureChange;
    const transmitter = selected.value.kind === "transmitter";
    const isCurrent = () => request === pressureChange && state === current.value &&
      (!transmitter || Number(state.family ?? 0) === 0);
    try {
      await changePressureUnit(state, value, transmitter, isCurrent,
        (input) => engine.request<string[]>("convertPressure", input));
    } catch (e) {
      if (isCurrent()) message.value = e instanceof Error ? e.message : String(e);
    }
    return;
  }
  current.value[key] = value;
  adjust(selected.value, current.value, key, catalog.value);
}
function divisionToggle() {
  current.value.division = !current.value.division;
  if (current.value.division) {
    delete current.value.tableStart;
    delete current.value.tableEnd;
    current.value.tableStep =
      Number(current.value.unit ?? 0) === 2 ? "1.8" : "1";
  }
}
function save() {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(saveNow, 500);
}
function saveNow() {
  clearTimeout(saveTimer);
  if (!catalog.value) return;
  try {
    saveChangedInputs(catalog.value.tools, states, savedInputs, localStorage);
  } catch {
    /* 存储不可用时继续保留本次会话输入。 */
  }
}
watch(
  () =>
    selected.value
      ? selected.value.id +
        ":" +
        inputGeneration.value +
        JSON.stringify(current.value)
      : "",
  () => {
    clearTimeout(timer);
    const version = ++revision;
    if (!selected.value || ["gauge", "pdf"].includes(selected.value.kind)) {
      save();
      busy.value = false;
      return;
    }
    busy.value = true;
    error.value = "";
    const tool = selected.value;
    timer = setTimeout(async () => {
      try {
        const input = JSON.parse(JSON.stringify(current.value));
        const value = await engine.request<Result>("calculate", {
          tool: tool.id,
          input,
        });
        if (version === revision) {
          resultSnapshot = { tool: tool.id, input };
          result.value = value;
          error.value = "";
        }
      } catch (e) {
        if (version === revision) {
          result.value = null;
          resultSnapshot = null;
          error.value = e instanceof Error ? e.message : String(e);
        }
      } finally {
        if (version === revision) busy.value = false;
      }
    }, 180);
    save();
  },
  { immediate: true },
);

async function curveValue(x: number): Promise<number> {
  const snapshot = resultSnapshot;
  if (!snapshot) throw new Error("曲线数据不可用");
  return engine.request<number>("curveValue", { ...snapshot, x });
}

function reset() {
  if (selected.value?.kind === "pdf") {
    pdfDownloader.value?.reset();
    return;
  }
  if (selected.value && catalog.value) {
    clearTimeout(timer);
    ++revision;
    ++inputGeneration.value;
    result.value = null;
    resultSnapshot = null;
    error.value = "";
    states[selected.value.id] = {};
    initialize(selected.value, states[selected.value.id], catalog.value);
    saveNow();
    message.value = "已恢复示例输入。";
    showToast("已恢复示例输入");
  }
}
function useCalibration(archive: CalibrationArchive) {
  if (!selected.value || !catalog.value) return;
  applyCalibration(selected.value, current.value, archive);
  initialize(selected.value, current.value, catalog.value);
}
function reload() {
  window.location.reload();
}
function report(tool: Tool, r: Result, input: State) {
  const note = calculationNote(tool, input);
  const lines = calculationMetadata(tool, input).map(([label, value]) => `${label}：${value}`);
  lines.push("");
  if (r.prediction_formula)
    lines.push("归一化预测式（用于复算）", r.prediction_formula, "");
  if (r.equation) lines.push("原坐标展开式（供参考）：y = " + r.equation, "");
  for (const m of r.metrics)
    lines.push(
      `${m.label}\t${typeof m.value === "number" ? decimal(m.value) : (m.value ?? "未定义")}\t${m.unit}`,
    );
  if (r.tolerances.length)
    lines.push(
      "",
      ...r.tolerances.map(
        (t) =>
          `${t.name}\t${t.delta === null ? "超出适用范围" : "±" + decimal(t.delta) + " " + t.unit}\t${t.formula}`,
      ),
    );
  if (r.details)
    lines.push(
      "",
      "详细结果：",
      JSON.stringify(
        r.details,
        (_key, value) => (typeof value === "number" ? decimal(value) : value),
        2,
      ),
    );
  if (r.rows.length)
    lines.push(
      "",
      r.headings.join("\t"),
      ...r.rows.map((row) =>
        row
          .map((v) => (typeof v === "number" ? decimal(v) : (v ?? "")))
          .join("\t"),
      ),
    );
  if (r.warnings.length) lines.push("", ...r.warnings);
  lines.push("", "计算说明", note.note);
  if (note.formula)
    lines.push("", "详细公式与取值", note.formula);
  return lines.join("\n");
}
async function copy() {
  const tool = selected.value;
  if (!tool || !result.value || busy.value || resultSnapshot?.tool !== tool.id) return;
  const text = report(tool, result.value, resultSnapshot.input);
  try {
    await navigator.clipboard.writeText(text);
    message.value = "已复制结果。";
    showToast("已复制完整计算结果");
  } catch {
    download(
      new Blob([text], { type: "text/plain;charset=utf-8" }),
      "toolbox-results.txt",
    );
    message.value = "无法复制，已下载结果文本。";
  }
}
async function copySingleValue(text: string, label: string) {
  try {
    await navigator.clipboard.writeText(text);
    showToast(`已复制 ${label}：${text}`);
  } catch {
    /* 剪贴板不可用时静默忽略 */
  }
}
function swapUnitSource(index: number) {
  if (!selected.value || selected.value.kind !== "unit") return;
  void change("source", String(index));
  const unitName = selected.value.units?.[index]?.name ?? "";
  if (unitName) showToast(`输入源单位已切换为：${unitName}`);
}

interface WritableFile {
  write(chunk: string): Promise<void>;
  close(): Promise<void>;
  abort(): Promise<void>;
}
type PickerWindow = Window & {
  showSaveFilePicker?: (
    options: unknown,
  ) => Promise<{ createWritable(): Promise<WritableFile> }>;
};
async function exportCsv() {
  const tool = selected.value;
  if (
    !tool || !result.value || busy.value || exporting.value ||
    resultSnapshot?.tool !== tool.id
  ) return;
  const snapshot = resultSnapshot.input;
  const r = result.value;
  exporting.value = true;
  cancelled = false;
  progress.value = 0;
  message.value = "";
  let writer: WritableFile | undefined;
  try {
    if (tool.kind !== "spline") {
      download(
        new Blob([resultCsv(tool, r, snapshot, calculationNote(tool, snapshot))], {
          type: "text/csv;charset=utf-8",
        }),
        tool.id + ".csv",
      );
      message.value = "完整结果 CSV 已下载。";
      showToast("已导出 CSV 文件");
      return;
    }
    const picker = (window as PickerWindow).showSaveFilePicker;
    if (picker) {
      const handle = await picker.call(window, {
        suggestedName: "bspline-results.csv",
        types: [{ description: "CSV 数据", accept: { "text/csv": [".csv"] } }],
      });
      writer = await handle.createWritable();
    } else if (r.total > 250_000)
      throw new Error(
        "此浏览器不支持流式保存，单次下载最多 250000 行。请增大采样步进，或使用支持文件流保存的桌面浏览器。",
      );
    const chunks: BlobPart[] = [];
    const header = "\uFEFFx,y\r\n";
    if (writer) await writer.write(header);
    else chunks.push(header);
    for (let offset = 0; offset < r.total; offset += 10_000) {
      if (cancelled) throw new Error("导出已取消");
      const text = await engine.request<string>("splineChunk", {
        input: snapshot,
        offset,
        limit: Math.min(10_000, r.total - offset),
      });
      if (cancelled) throw new Error("导出已取消");
      if (writer) await writer.write(text);
      else chunks.push(text);
      progress.value = Math.min(r.total, offset + 10_000) / r.total;
    }
    if (cancelled) throw new Error("导出已取消");
    if (writer) {
      await writer.close();
      writer = undefined;
    } else
      download(
        new Blob(chunks, { type: "text/csv;charset=utf-8" }),
        "bspline-results.csv",
      );
    message.value = `已导出全部 ${r.total} 行采样结果。`;
    showToast(`已导出 ${r.total} 行采样 CSV`);
  } catch (e) {
    if (writer) await writer.abort().catch(() => {});
    message.value =
      e instanceof DOMException && e.name === "AbortError"
        ? "已取消选择保存位置。"
        : e instanceof Error
          ? e.message
          : String(e);
  } finally {
    exporting.value = false;
  }
}

onMounted(async () => {
  applyTheme(theme.value);
  window.addEventListener("pagehide", saveNow);
  window.addEventListener("keydown", onGlobalKeydown);
  try {
    const c = await engine.request<Catalog>("catalog");
    catalog.value = c;
    try {
      const saved = readRememberedInputs(c.tools, localStorage);
      for (const [id, state] of Object.entries(saved)) {
        savedInputs.set(id, JSON.stringify(state));
      }
      Object.assign(states, saved);
    } catch {
      /* 损坏或旧版输入不阻碍网站启动。 */
    }
    saveNow();
    route();
    window.addEventListener("hashchange", route);
  } catch (e) {
    loadError.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
});
onUnmounted(() => {
  saveNow();
  window.removeEventListener("pagehide", saveNow);
  window.removeEventListener("keydown", onGlobalKeydown);
  cancelled = true;
  clearTimeout(timer);
  clearTimeout(saveTimer);
  window.removeEventListener("hashchange", route);
  engine.close();
});
</script>

<!-- prettier-ignore -->
<template>
  <header class="site-header">
    <div class="brand">
      <button
        class="icon-button mobile-menu"
        aria-label="打开工具导航"
        @click="mobileNav = !mobileNav"
      >
        <svg viewBox="0 0 24 24">
          <path d="M4 6h16M4 12h16M4 18h16" />
        </svg>
      </button>
      <a href="#" class="brand-link" @click.prevent="open(null)">
        <img :src="logo" width="32" height="32" alt="工具箱图标" />
        <div class="brand-text">
          <strong>工具箱</strong>
          <small>计量工程工作台 · v{{ version }}</small>
        </div>
      </a>
    </div>

    <button
      type="button"
      class="header-search-trigger"
      aria-label="快速搜索工具"
      @click="openPalette"
    >
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <circle cx="11" cy="11" r="6.5" />
        <path d="m16 16 4 4" />
      </svg>
      <span>快速搜索任意工具…</span>
      <kbd class="kbd-hint">Ctrl K</kbd>
    </button>

    <div class="header-actions">
      <div class="header-status">
        <span v-if="catalog" class="header-tool-count">{{ catalog.tools.length }} 项工具</span>
        <span class="privacy-dot">本地计算</span>
      </div>
      <button
        type="button"
        class="icon-button theme-toggle"
        :title="theme === 'light' ? '切换到深色主题' : '切换到浅色主题'"
        :aria-label="theme === 'light' ? '切换到深色主题' : '切换到浅色主题'"
        @click="toggleTheme"
      >
        <svg v-if="theme === 'light'" viewBox="0 0 24 24">
          <path d="M21 12.8A8.5 8.5 0 1 1 11.2 3a6.8 6.8 0 0 0 9.8 9.8Z" />
        </svg>
        <svg v-else viewBox="0 0 24 24">
          <circle cx="12" cy="12" r="4" />
          <path d="M12 2v2m0 16v2M4.9 4.9l1.4 1.4m11.4 11.4 1.4 1.4M2 12h2m16 0h2M4.9 19.1l1.4-1.4m11.4-11.4 1.4-1.4" />
        </svg>
      </button>
    </div>
  </header>

  <div v-if="mobileNav" class="nav-backdrop" @click="mobileNav = false"></div>
  <aside :class="['sidebar', { collapsed, 'mobile-open': mobileNav }]">
    <div class="sidebar-heading">
      <strong>{{ collapsed ? "分类" : "工具导航" }}</strong>
      <button
        class="icon-button sidebar-toggle"
        :aria-label="collapsed ? '展开导航' : '折叠导航'"
        @click="collapsed = !collapsed"
      >
        <svg viewBox="0 0 24 24">
          <path :d="collapsed ? 'm9 6 6 6-6 6' : 'm15 6-6 6 6 6'" />
        </svg>
      </button>
    </div>

    <div v-if="!collapsed" class="sidebar-filter">
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <circle cx="11" cy="11" r="6.5" />
        <path d="m16 16 4 4" />
      </svg>
      <input
        v-model="sidebarFilter"
        type="search"
        placeholder="筛选导航…"
        aria-label="筛选侧栏工具"
      />
    </div>

    <div v-if="!collapsed && favoriteTools.length && !sidebarFilter.trim()" class="sidebar-favorites">
      <div class="sidebar-section-label">
        <svg viewBox="0 0 24 24"><path d="m12 3 2.8 5.7 6.2.9-4.5 4.4 1 6.2L12 17.3 6.5 20.2l1-6.2L3 9.6l6.2-.9L12 3Z" /></svg>
        <span>我的收藏</span>
      </div>
      <div
        v-for="tool in favoriteTools"
        :key="'fav-' + tool.id"
        :data-group="groupKey(tool.group)"
        :class="['nav-tool-row', { selected: selected?.id === tool.id }]"
      >
        <button class="nav-tool-btn" @click="open(tool)">{{ tool.name }}</button>
        <button
          type="button"
          class="nav-star-btn starred"
          aria-label="取消收藏"
          @click.stop="toggleFavorite(tool)"
        >
          <svg viewBox="0 0 24 24"><path d="m12 3 2.8 5.7 6.2.9-4.5 4.4 1 6.2L12 17.3 6.5 20.2l1-6.2L3 9.6l6.2-.9L12 3Z" /></svg>
        </button>
      </div>
    </div>

    <nav aria-label="工具分类">
      <div
        v-for="group in groups"
        :key="group"
        :data-group="groupKey(group)"
        class="nav-group"
        v-show="!sidebarFilter.trim() || sidebarToolsInGroup(group).length > 0"
      >
        <button
          :class="[
            'group-button',
            { active: selected?.group === group || filter === group },
          ]"
          :title="group"
          @click="
            collapsed
              ? chooseGroup(group)
              : expanded.has(group)
                ? expanded.delete(group)
                : expanded.add(group)
          "
        >
          <span class="group-icon-badge" aria-hidden="true">
            <svg viewBox="0 0 24 24"><path :d="groupIconPath(group)" /></svg>
          </span>
          <span class="group-label">{{
            collapsed ? groupShortName(group) : group
          }}</span>
          <template v-if="!collapsed">
            <span class="group-count">{{ sidebarToolsInGroup(group).length }}</span>
            <svg
              :class="['group-chevron', { expanded: expanded.has(group) || !!sidebarFilter.trim() }]"
              viewBox="0 0 24 24"
            >
              <path d="m9 5 7 7-7 7" />
            </svg>
          </template>
        </button>
        <div v-if="!collapsed && (expanded.has(group) || !!sidebarFilter.trim())" class="nav-tools">
          <div
            v-for="tool in sidebarToolsInGroup(group)"
            :key="tool.id"
            :class="['nav-tool-row', { selected: selected?.id === tool.id }]"
          >
            <button class="nav-tool-btn" @click="open(tool)">
              {{ tool.name }}
            </button>
            <button
              type="button"
              :class="['nav-star-btn', { starred: isFavorite(tool.id) }]"
              :aria-label="isFavorite(tool.id) ? '取消收藏' : '收藏工具'"
              @click.stop="toggleFavorite(tool)"
            >
              <svg viewBox="0 0 24 24"><path d="m12 3 2.8 5.7 6.2.9-4.5 4.4 1 6.2L12 17.3 6.5 20.2l1-6.2L3 9.6l6.2-.9L12 3Z" /></svg>
            </button>
          </div>
        </div>
      </div>
    </nav>
  </aside>

  <main
    :class="['main', { wide: collapsed, 'has-mobile-actions': !!selected && selected.kind !== 'pdf' }]"
  >
    <!-- 多工具标签栏 -->
    <div v-if="catalog && (openTabs.length > 0 || selected)" class="workspace-tabs" role="tablist" aria-label="已打开的工具">
      <button
        type="button"
        role="tab"
        :aria-selected="!selected"
        :class="['tab-pill', { active: !selected }]"
        @click="open(null)"
      >
        首页
      </button>
      <div
        v-for="tab in openTabs"
        :key="'tab-' + tab.id"
        :data-group="groupKey(tab.group)"
        role="tab"
        :aria-selected="selected?.id === tab.id"
        :class="['tab-pill', { active: selected?.id === tab.id }]"
        @click="open(tab)"
        @auxclick.middle.prevent="closeTab(tab.id)"
      >
        <span class="tab-pill-dot" aria-hidden="true"></span>
        <span>{{ tab.name }}</span>
        <span
          class="tab-close"
          role="button"
          :aria-label="`关闭 ${tab.name}`"
          @click.stop="closeTab(tab.id)"
        >×</span>
      </div>
    </div>

    <PwaStatus :busy="busy || exporting" @before-update="saveNow" />
    <div v-if="loading" class="loading-state">
      <div class="loader"></div>
      <h1>正在加载工具箱</h1>
      <p>首次访问需要加载计算模块。</p>
    </div>
    <section v-else-if="loadError" class="panel fatal-error">
      <h1>计算模块暂时无法加载</h1>
      <p>{{ loadError }}</p>
      <button class="button primary" @click="reload">重新加载</button>
    </section>

    <template v-else-if="catalog">
      <template v-if="!selected">
        <section class="hero">
          <div class="eyebrow">MEASUREMENT &amp; ENGINEERING WORKSPACE</div>
          <h1>计量与工程计算，一处完成</h1>
          <p>常用换算、温标校准、误差评定与量规计算，打开即用。</p>
          <div class="hero-facts">
            <span>{{ groups.length }} 大分类</span><i></i>
            <span>{{ catalog.tools.length }} 项工具</span><i></i>
            <span>{{ totalUnits }} 项换算单位</span>
          </div>
        </section>

        <section v-if="quickAccessTools.length" class="quick-access" aria-label="常用快捷访问">
          <span class="quick-access-label">快捷访问</span>
          <button
            v-for="tool in quickAccessTools"
            :key="'qa-' + tool.id"
            :data-group="groupKey(tool.group)"
            class="quick-chip"
            @click="open(tool)"
          >
            <span class="quick-chip-dot" aria-hidden="true"></span>
            <span>{{ tool.name }}</span>
          </button>
        </section>

        <div class="dashboard-toolbar">
          <div class="category-tabs" aria-label="筛选工具分类">
            <button
              v-for="group in ['全部工具', ...groups]"
              :key="group"
              :data-group="group === '全部工具' ? undefined : groupKey(group)"
              :class="{ active: filter === group }"
              @click="filter = group"
            >
              {{ group === "ITS90 温标换算" ? "温标换算" : group }}
              <span>{{
                group === "全部工具"
                  ? catalog.tools.length
                  : catalog.tools.filter((t) => t.group === group).length
              }}</span>
            </button>
          </div>
          <label class="catalog-search">
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <circle cx="10.8" cy="10.8" r="6.3" />
              <path d="m15.5 15.5 4.2 4.2" />
            </svg>
            <input
              v-model="toolSearch"
              type="search"
              aria-label="搜索工具名称、分类或说明"
              placeholder="搜索工具名称、分类或说明…"
            />
          </label>
        </div>

        <div class="catalog-heading">
          <h2>{{ toolSearch.trim() ? "搜索结果" : filter }}</h2>
          <span>{{ filtered.length }} 项</span>
        </div>

        <div v-if="filtered.length" class="tool-grid">
          <div
            v-for="tool in filtered"
            :key="tool.id"
            :data-group="groupKey(tool.group)"
            class="tool-card"
            role="button"
            tabindex="0"
            @click="open(tool)"
            @keydown.enter.prevent="open(tool)"
          >
            <button
              type="button"
              :class="['tool-card-star', { starred: isFavorite(tool.id) }]"
              :title="isFavorite(tool.id) ? '取消收藏' : '收藏此工具'"
              :aria-label="isFavorite(tool.id) ? '取消收藏' : '收藏此工具'"
              @click.stop="toggleFavorite(tool)"
            >
              <svg viewBox="0 0 24 24"><path d="m12 3 2.8 5.7 6.2.9-4.5 4.4 1 6.2L12 17.3 6.5 20.2l1-6.2L3 9.6l6.2-.9L12 3Z" /></svg>
            </button>
            <div class="tool-card-top">
              <span class="tool-icon">{{ tool.icon || "工" }}</span>
              <div>
                <h3>{{ tool.name }}</h3>
                <span class="tool-category">{{ tool.group }}</span>
              </div>
            </div>
            <p>{{ tool.description }}</p>
          </div>
        </div>
        <section v-else class="catalog-empty" role="status">
          <strong>没有找到匹配的工具</strong>
          <p>试试更短的关键词，或切换到“全部工具”。</p>
          <button
            v-if="toolSearch"
            class="button secondary"
            @click="toolSearch = ''; filter = '全部工具'"
          >
            清除筛选
          </button>
        </section>
      </template>

      <template v-else>
        <nav class="breadcrumb" aria-label="当前位置">
          <button @click="open(null)">← 工具箱</button><span>/</span>
          <span>{{ selected.group }}</span>
          <span>/</span><span>{{ selected.name }}</span>
        </nav>

        <div class="tool-heading" :data-group="groupKey(selected.group)">
          <span class="tool-icon large">{{ selected.icon || "工" }}</span>
          <div>
            <h1>{{ selected.name }}</h1>
            <p>{{ selected.description }}</p>
          </div>
          <div class="tool-heading-actions">
            <button
              type="button"
              class="icon-button theme-toggle"
              :title="isFavorite(selected.id) ? '取消收藏' : '收藏工具'"
              :aria-label="isFavorite(selected.id) ? '取消收藏' : '收藏工具'"
              @click="toggleFavorite(selected)"
            >
              <svg viewBox="0 0 24 24" :style="isFavorite(selected.id) ? 'fill:#f59e0b;stroke:#f59e0b' : ''">
                <path d="m12 3 2.8 5.7 6.2.9-4.5 4.4 1 6.2L12 17.3 6.5 20.2l1-6.2L3 9.6l6.2-.9L12 3Z" />
              </svg>
            </button>
            <button
              type="button"
              class="button reset-button"
              :disabled="exporting || (selected.kind === 'pdf' && pdfDownloader?.busy)"
              @click="reset"
            >
              {{ selected.kind === "pdf" ? "清空" : "恢复示例" }}
            </button>
          </div>
        </div>

        <CalibrationArchives
          v-if="selected.kind === 'standard_couple' || selected.kind === 'sprt'"
          :key="`${selected.id}:${inputGeneration}`"
          :tool="selected"
          :state="current"
          :catalog="catalog"
          :engine="engine"
          @apply="useCalibration"
        />

        <GaugeTool
          v-if="selected.kind === 'gauge'"
          :key="selected.id + ':' + inputGeneration"
          :engine="engine"
          :tool="selected"
          :state="current"
          @state="states[selected!.id] = $event"
          @toast="showToast"
        />

        <template v-else-if="selected.kind !== 'pdf'">
          <div class="tool-workspace">
            <div ref="inputPanel" class="input-panels">
              <section
                v-for="section in sections"
                :key="selected.id + section.title"
                class="panel input-panel"
              >
                <div class="section-heading">
                  <h2>{{ section.title }}</h2>
                  <span v-if="busy" class="working-label">计算中…</span>
                </div>
                <div class="form-grid">
                  <template v-for="field in section.fields" :key="selected.id + field.key">
                    <h3 v-if="selected.kind === 'uncertainty' && Number(current.mode) === 1 && field.key === 'combine'" class="full-width">合成设置</h3>
                    <ResultantComponents
                      v-if="selected.kind === 'resultant' && field.key === 'values'"
                      :values="current.values"
                      :sensitivities="current.sensitivities"
                      @update:values="change('values', $event)"
                      @update:sensitivities="change('sensitivities', $event)"
                    />
                    <QuickUncertainty
                      v-else-if="selected.kind === 'uncertainty' && Number(current.mode) === 1 && field.key === 'aValues'"
                      :state="current" :fields="section.fields" :catalog="catalog" :details="busy ? [] : budget"
                      @change="change"
                    />
                    <FieldEditor
                      v-else-if="(selected.kind !== 'resultant' || field.key !== 'sensitivities') && !(selected.kind === 'uncertainty' && Number(current.mode) === 1 && ['aAverageCount', 'aSensitivity', 'aDigits', 'aRounding', 'bComponents'].includes(field.key))"
                      :class="{
                        'full-width': !['number', 'text', 'select'].includes(field.type) ||
                          (selected.kind === 'uncertainty' && ['mode', 'combine', 'coverage', 'digits', 'roundingV2'].includes(field.key)),
                      }"
                      :field="field"
                      :model-value="current[field.key]"
                      :catalog="catalog"
                      :budget-details="
                        busy
                          ? []
                          : field.key === 'components'
                            ? budget
                            : field.key === 'bComponents'
                              ? budget.slice(1)
                              : []
                      "
                      @update:model-value="change(field.key, $event)"
                    />
                  </template>
                </div>
                <details v-if="section.advanced" class="advanced-settings">
                  <summary>
                    <span>{{ section.advanced.title }}</span>
                    <span class="advanced-toggle" aria-hidden="true">
                      <span class="advanced-expand">展开</span>
                      <span class="advanced-collapse">收起</span>
                      <svg viewBox="0 0 20 20"><path d="m5 7 5 5 5-5" /></svg>
                    </span>
                  </summary>
                  <div class="form-grid">
                    <FieldEditor
                      v-for="field in section.advanced.fields"
                      :key="selected.id + field.key"
                      :field="field"
                      :model-value="current[field.key]"
                      :catalog="catalog"
                      @update:model-value="change(field.key, $event)"
                    />
                  </div>
                </details>
              </section>
              <section v-if="divisions" class="panel division-settings">
                <button class="disclosure" @click="divisionToggle">
                  <span>分度表与特性曲线</span>
                  <span>{{ current.division ? "收起 −" : "展开 ＋" }}</span>
                </button>
                <div v-if="current.division" class="form-grid">
                  <label
                    v-for="(label, key) in {
                      tableStart: '起始温度',
                      tableEnd: '终止温度',
                      tableStep: '温度步长',
                    }"
                    :key="key"
                    class="field"
                  >
                    <span class="field-label">{{ label }}</span>
                    <NumericInput
                      :model-value="current[key] ?? ''"
                      :placeholder="
                        key === 'tableStep' ? '1' : '留空使用有效温区边界'
                      "
                      @update:model-value="
                        $event.trim()
                          ? change(key, $event)
                          : delete current[key]
                      "
                    />
                  </label>
                </div>
              </section>
            </div>

            <div ref="resultPanel" class="output-panels" aria-live="polite">
              <section class="panel result-panel">
                <div class="section-heading">
                  <div>
                    <h2>计算结果</h2>
                    <p v-if="result?.metrics.length" class="metric-hint">点击结果卡片可一键复制数值</p>
                  </div>
                  <span v-if="!busy && !error && result" class="result-ready">
                    <i></i> 已更新
                  </span>
                </div>
                <div v-if="error" class="calculation-error">
                  <strong>请检查输入</strong>
                  <p>{{ error }}</p>
                </div>
                <div v-else-if="!result" class="result-placeholder">
                  {{ busy ? "正在计算…" : "填写参数后显示结果" }}
                </div>
                <template v-else>
                  <div v-if="result.equation" class="equation">
                    <span>拟合方程</span>
                    <p>y = {{ equation }}</p>
                  </div>
                  <div v-if="result.metrics.length" :class="['metric-grid', { 'is-updating': busy }]">
                    <div
                      v-for="(metric, j) in result.metrics"
                      :key="j"
                      class="metric copyable"
                      role="button"
                      tabindex="0"
                      :title="`点击复制 ${metric.label}`"
                      @click="copySingleValue(display(metric.value), metric.label)"
                      @keydown.enter.prevent="copySingleValue(display(metric.value), metric.label)"
                    >
                      <span>{{ metric.label }}</span>
                      <strong>
                        {{ display(metric.value) }}
                        <small v-if="metric.unit"> {{ metric.unit }}</small>
                      </strong>
                    </div>
                  </div>
                  <div
                    v-for="warning in result.warnings"
                    :key="warning"
                    class="notice"
                  >
                    {{ warning }}
                  </div>
                  <div class="result-actions">
                    <button
                      class="button primary"
                      :disabled="busy || exporting"
                      @click="exportCsv"
                    >
                      {{ exporting ? "正在导出…" : "导出 CSV" }}
                    </button>
                    <button
                      class="button secondary"
                      :disabled="busy || exporting"
                      @click="copy"
                    >
                      复制完整结果
                    </button>
                  </div>
                </template>
                <div v-if="exporting" class="export-progress">
                  <progress :value="progress" max="1"></progress>
                  <span>{{ Math.round(progress * 100) }}%</span>
                  <button class="button subtle" @click="cancelled = true">
                    取消导出
                  </button>
                </div>
                <p v-if="message" class="action-message" role="status">
                  {{ message }}
                </p>
              </section>

              <ResultTable
                v-if="selected.kind === 'unit' && result?.rows.length"
                :key="selected.id + '-unit'"
                :headings="result.headings"
                :rows="result.rows"
                :unit-table="true"
                :source-index="Number(current.source ?? 0)"
                :total="result.total"
                @copy="copySingleValue"
                @swap-unit="swapUnitSource"
              />

              <section v-if="budget.length" class="panel">
                <h2>分量贡献</h2>
                <BudgetResult :nodes="budget" />
              </section>
              <section v-if="coefficients" class="panel">
                <h2>计算使用的系数</h2>
                <div
                  v-for="(value, key) in coefficients"
                  :key="key"
                  class="coefficient-row"
                >
                  <span>{{ key }}</span>
                  <span>{{
                    Array.isArray(value)
                      ? value.map(display).join("，")
                      : display(value)
                  }}</span>
                </div>
              </section>
            </div>
          </div>

          <section v-if="result?.tolerances.length" class="panel">
            <div class="section-heading">
              <h2>等级允差</h2>
              <span class="muted">按当前温度判断适用范围</span>
            </div>
            <div class="tolerance-grid">
              <div
                v-for="t in result.tolerances"
                :key="t.name"
                class="tolerance-card"
              >
                <h3>{{ t.name }}</h3>
                <strong v-if="t.delta !== null">±{{ display(t.delta) }} {{ t.unit }}</strong>
                <span v-else class="muted">当前温度超出适用范围</span>
                <p v-if="t.temperature">
                  温度界限：{{ display(t.temperature[0]) }}～{{ display(t.temperature[1]) }} {{ t.unit }}
                </p>
                <p v-if="t.signal">
                  信号界限：{{ display(t.signal[0]) }}～{{ display(t.signal[1]) }} {{ t.signalUnit }}
                </p>
                <small>{{ t.formula }}</small>
                <small>适用范围：{{ t.min }}～{{ t.max }} °C</small>
              </div>
            </div>
          </section>

          <section v-if="result?.curve.length" class="panel chart-panel">
            <div class="section-heading">
              <h2>
                {{
                  selected.kind === "fit"
                    ? "校准曲线"
                    : selected.kind === "spline"
                      ? "插值曲线"
                      : "特性曲线"
                }}
              </h2>
              <div class="chart-legend">
                <span>— 曲线</span>
                <span v-if="result.points.length">● 数据点</span>
              </div>
            </div>
            <CurveChart
              :curve="result.curve"
              :points="result.points"
              :x-label="axisX"
              :y-label="axisY"
              :evaluate="selected.kind === 'fit' || selected.kind === 'spline' ? curveValue : undefined"
            />
          </section>

          <section v-if="result?.residuals.length" class="panel chart-panel">
            <div class="section-heading">
              <h2>残差分布</h2>
              <span class="muted">实测响应 − 拟合响应</span>
            </div>
            <CurveChart
              :curve="[]"
              :points="result.residuals"
              :scatter="true"
              :x-label="axisX"
              y-label="残差"
            />
          </section>

          <ResultTable
            v-if="selected.kind !== 'unit' && result?.rows.length"
            :key="selected.id"
            :headings="result.headings"
            :rows="result.rows"
            :unit-table="false"
            :total="result.total"
            @copy="copySingleValue"
          />

          <ResultTable
            v-if="calibrationRows.length"
            :key="selected.id + '-calibration'"
            :headings="[
              '校准温度 (°C)',
              selected.kind === 'thermistor'
                ? '温度残差 (°C)'
                : '热电势残差 (mV)',
            ]"
            :rows="calibrationRows"
            @copy="copySingleValue"
          />
        </template>

        <CalculationProcess v-if="selected.kind !== 'gauge' && result?.process.length" :key="selected.id + '-process'" :lines="result.process" />
        <details v-if="activeNote.note || activeNote.formula" :key="selected.id" class="collapsible-note explanation">
          <summary>计算说明</summary>
          <p v-for="(paragraph, index) in explanationParagraphs" :key="index">
            <FormulaText :line="paragraph" />
          </p>
          <template v-if="activeNote.formula">
            <strong>详细公式与取值</strong>
            <p v-for="(line, index) in activeNote.formula.split(/\r?\n/u)" :key="index"><FormulaText :line="line" /></p>
          </template>
        </details>
      </template>
    </template>

    <PdfDownload ref="pdfDownloader" v-show="selected?.kind === 'pdf'" :active="selected?.kind === 'pdf'" />
  </main>

  <MobileActions
    :selected="!!selected && selected.kind !== 'pdf'"
    :hidden="mobileNav"
    @inputs="jumpInputs"
    @results="jumpResults"
  />

  <!-- Ctrl+K 全局快速搜索命令面板 -->
  <div v-if="paletteOpen" class="palette-backdrop" @click.self="closePalette">
    <div class="palette-dialog" role="dialog" aria-modal="true" aria-label="快速搜索工具">
      <div class="palette-input-row">
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <circle cx="11" cy="11" r="6.5" />
          <path d="m16 16 4 4" />
        </svg>
        <input
          ref="paletteInput"
          v-model="paletteQuery"
          type="search"
          placeholder="输入工具名称、分类或关键词，回车直达…"
          aria-label="搜索工具"
          @keydown="onPaletteKey"
        />
        <kbd class="kbd-hint">ESC</kbd>
      </div>
      <div class="palette-list">
        <button
          v-for="(tool, idx) in paletteMatches"
          :key="'pal-' + tool.id"
          type="button"
          :data-group="groupKey(tool.group)"
          :class="['palette-item', { highlighted: idx === paletteIndex }]"
          @pointerenter="($event as PointerEvent).pointerType === 'mouse' && (paletteIndex = idx)"
          @click="closePalette(); open(tool)"
        >
          <span class="tool-icon">{{ tool.icon || "工" }}</span>
          <div class="palette-item-info">
            <strong>{{ tool.name }}</strong>
            <small>{{ tool.description }}</small>
          </div>
          <span class="palette-item-group">{{ groupShortName(tool.group) }}</span>
        </button>
        <p v-if="!paletteMatches.length" class="palette-empty">未找到匹配的工具</p>
      </div>
    </div>
  </div>

  <!-- 右下角 Toast 轻提示 -->
  <div class="toast-stack" aria-live="polite">
    <div v-for="t in toasts" :key="t.id" class="toast-item success">
      {{ t.text }}
    </div>
  </div>
</template>
