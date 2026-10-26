<script setup lang="ts">
import GaugeTool from "./components/GaugeTool.vue";
import NumericInput from "./components/NumericInput.vue";
import MobileActions from "./components/MobileActions.vue";
import PwaStatus from "./components/PwaStatus.vue";
import { computed, onMounted, onUnmounted, reactive, ref, watch } from "vue";
import type { Catalog, Result, State, Tool } from "./types";
import { Engine } from "./engine";
import { schema, initialize, adjust } from "./schema";
import { decimal, display, download } from "./format";
import { resultCsv } from "./export";
import { calculationNote } from "./notes";
import FieldEditor from "./components/FieldEditor.vue";
import CurveChart from "./components/CurveChart.vue";
import ResultTable from "./components/ResultTable.vue";
import BudgetResult, { type Detail } from "./components/BudgetResult.vue";

const engine = new Engine();
const catalog = ref<Catalog>();
const loading = ref(true);
const loadError = ref("");
const selected = ref<Tool | null>(null);
const inputPanel = ref<HTMLElement>();
const resultPanel = ref<HTMLElement>();
let inputScroll: number | null = null;
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
const activeNote = computed(() => selected.value
  ? calculationNote(selected.value, current.value)
  : { note: "", formula: "" });
const explanationParagraphs = computed(() =>
  activeNote.value.note
    .split(/\r?\n/u)
    .map((text) => text.trim())
    .filter(Boolean),
);
const filter = ref("全部工具");
const query = ref("");
const mobileNav = ref(false);
const collapsed = ref(false);
const expanded = reactive(new Set<string>());
const states = reactive<Record<string, State>>({});
const result = ref<Result | null>(null);
const busy = ref(false);
const error = ref("");
const message = ref("");
const exporting = ref(false);
const progress = ref(0);
let cancelled = false;
const groups = computed(() => [
  ...new Set(catalog.value?.tools.map((t) => t.group) ?? []),
]);
const current = computed<State>(() =>
  selected.value ? (states[selected.value.id] ?? {}) : {},
);
const sections = computed(() =>
  selected.value && catalog.value
    ? schema(selected.value, current.value, catalog.value)
    : [],
);
const filtered = computed(
  () =>
    catalog.value?.tools.filter(
      (t) =>
        (filter.value === "全部工具" || t.group === filter.value) &&
        `${t.name} ${t.description} ${t.group}`
          .toLowerCase()
          .includes(query.value.trim().toLowerCase()),
    ) ?? [],
);
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
      Record<string, number | number[]> | undefined,
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
let revision = 0;

function open(tool: Tool | null, hash = true) {
  if (tool && selected.value?.id === tool.id) {
    mobileNav.value = false;
    window.scrollTo({ top: 0, behavior: "instant" });
    return;
  }
  selected.value = tool;
  inputScroll = null;
  result.value = null;
  error.value = "";
  message.value = "";
  mobileNav.value = false;
  if (tool && catalog.value) {
    states[tool.id] ??= {};
    initialize(tool, states[tool.id], catalog.value);
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
    const from = Number(state.pressureUnit);
    const to = Number(value);
    const names = [
      "low",
      "high",
      "refLow",
      "refHigh",
      ...(selected.value.kind === "transmitter" && Number(state.direction) === 0
        ? ["value"]
        : []),
    ].filter((k) => state[k] !== undefined && String(state[k]).trim());
    try {
      const values = await engine.request<string[]>("convertPressure", {
        source: from,
        target: to,
        values: names.map((k) => state[k]),
      });
      names.forEach((k, j) => (state[k] = values[j]));
      state[key] = value;
      return;
    } catch (e) {
      message.value = e instanceof Error ? e.message : String(e);
      return;
    }
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
  try {
    localStorage.setItem("toolbox-web:v1", JSON.stringify(states));
  } catch {
    /* 存储不可用时继续保留本次会话输入。 */
  }
}
watch(
  // 不同工具可能拥有完全相同的默认输入，必须把工具 ID 纳入更新依据。
  () =>
    selected.value ? selected.value.id + JSON.stringify(current.value) : "",
  () => {
    clearTimeout(timer);
    const version = ++revision;
    if (!selected.value || selected.value.kind === "gauge") {
      save();
      busy.value = false;
      return;
    }
    busy.value = true;
    error.value = "";
    const tool = selected.value;
    timer = setTimeout(async () => {
      try {
        const value = await engine.request<Result>("calculate", {
          tool: tool.id,
          input: JSON.parse(JSON.stringify(current.value)),
        });
        if (version === revision) {
          result.value = value;
          error.value = "";
        }
      } catch (e) {
        if (version === revision) {
          result.value = null;
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

function reset() {
  if (selected.value && catalog.value) {
    states[selected.value.id] = {};
    initialize(selected.value, states[selected.value.id], catalog.value);
    message.value = "已恢复示例输入。";
  }
}
function reload() {
  window.location.reload();
}
function report() {
  if (!selected.value || !result.value) return "";
  const r = result.value;
  const lines = [
    selected.value.name,
    "",
    `输入参数：${JSON.stringify(current.value)}`,
    "",
  ];
  if (r.equation) lines.push("拟合方程：y = " + r.equation, "");
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
  lines.push("", "计算说明", activeNote.value.note);
  if (activeNote.value.formula) lines.push("", "详细公式与取值", activeNote.value.formula);
  return lines.join("\n");
}
async function copy() {
  try {
    await navigator.clipboard.writeText(report());
    message.value = "已复制结果。";
  } catch {
    download(
      new Blob([report()], { type: "text/plain;charset=utf-8" }),
      "toolbox-results.txt",
    );
    message.value = "无法复制，已下载结果文本。";
  }
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
  if (!selected.value || !result.value || busy.value || exporting.value) return;
  const tool = selected.value;
  const snapshot = JSON.parse(JSON.stringify(current.value));
  const r = result.value;
  exporting.value = true;
  cancelled = false;
  progress.value = 0;
  message.value = "";
  let writer: WritableFile | undefined;
  try {
    if (tool.kind !== "spline") {
      download(
        new Blob([resultCsv(tool, r, activeNote.value)], { type: "text/csv;charset=utf-8" }),
        tool.id + ".csv",
      );
      message.value = "完整结果 CSV 已下载。";
      return;
    }
    // 支持文件流的浏览器逐块写入，其余浏览器使用有界内存下载。
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
  window.addEventListener("pagehide", saveNow);
  try {
    const c = await engine.request<Catalog>("catalog");
    catalog.value = c;
    try {
      const saved = JSON.parse(localStorage.getItem("toolbox-web:v1") ?? "{}");
      if (saved && typeof saved === "object" && !Array.isArray(saved))
        for (const tool of c.tools) {
          const v = saved[tool.id];
          if (v && typeof v === "object" && !Array.isArray(v))
            states[tool.id] = v;
        }
    } catch {
      /* 损坏或旧版输入不阻碍网站启动。 */
    }
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
  cancelled = true;
  clearTimeout(timer);
  clearTimeout(saveTimer);
  window.removeEventListener("hashchange", route);
  engine.close();
});
</script>

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
        </svg></button
      ><a href="#" class="brand-link" @click.prevent="open(null)"
        ><img :src="logo" width="42" height="42" alt="工具箱图标" /><span
          >工具箱</span
        ></a
      >
    </div>
    <div class="header-status">
      <span v-if="catalog">{{ catalog.tools.length }} 项工具</span
      ><span class="privacy-dot">浏览器内计算</span>
    </div>
  </header>
  <div v-if="mobileNav" class="nav-backdrop" @click="mobileNav = false"></div>
  <aside :class="['sidebar', { collapsed, 'mobile-open': mobileNav }]">
    <div class="sidebar-heading">
      <strong>{{ collapsed ? "分类" : "工具导航" }}</strong
      ><button
        class="icon-button sidebar-toggle"
        :aria-label="collapsed ? '展开导航' : '折叠导航'"
        @click="collapsed = !collapsed"
      >
        <svg viewBox="0 0 24 24">
          <path :d="collapsed ? 'm9 6 6 6-6 6' : 'm15 6-6 6 6 6'" />
        </svg>
      </button>
    </div>
    <label v-if="!collapsed" class="sidebar-search"
      ><svg viewBox="0 0 24 24">
        <circle cx="10.5" cy="10.5" r="6.5" />
        <path d="m16 16 4 4" /></svg
      ><input
        v-model="query"
        placeholder="搜索工具"
        aria-label="搜索工具"
        @input="selected && open(null)"
    /></label>
    <nav aria-label="工具分类">
      <div v-for="group in groups" :key="group" class="nav-group">
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
          <span>{{
            collapsed
              ? ({
                  单位换算: "单位",
                  "ITS90 温标换算": "温标",
                  压力工具: "压力",
                  温湿度工具: "温湿度",
                  误差工具: "误差",
                  量规计算: "量规",
                  其他工具: "其他",
                }[group] ?? group)
              : group
          }}</span
          ><template v-if="!collapsed"
            ><span class="group-count">{{
              catalog?.tools.filter((t) => t.group === group).length
            }}</span
            ><svg
              :class="{ expanded: expanded.has(group) }"
              viewBox="0 0 24 24"
            >
              <path d="m9 5 7 7-7 7" /></svg
          ></template>
        </button>
        <div v-if="!collapsed && expanded.has(group)" class="nav-tools">
          <button
            v-for="tool in catalog?.tools.filter(
              (t) => t.group === group && (!query || t.name.includes(query)),
            )"
            :key="tool.id"
            :class="{ selected: selected?.id === tool.id }"
            @click="open(tool)"
          >
            {{ tool.name }}
          </button>
        </div>
      </div>
    </nav>
  </aside>
  <main
    :class="['main', { wide: collapsed, 'has-mobile-actions': !!selected }]"
  >
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
          <div class="eyebrow">MEASUREMENT & ENGINEERING</div>
          <h1>计量与工程计算，一处完成</h1>
          <p>常用换算、误差评定与校准计算，打开即用。</p>
          <div class="hero-facts">
            <span>{{ groups.length }} 大分类</span><i></i
            ><span>{{ catalog.tools.length }} 项工具</span><i></i
            ><span>{{ totalUnits }} 项换算单位</span>
          </div>
        </section>
        <div class="dashboard-toolbar">
          <div class="category-tabs" aria-label="筛选工具分类">
            <button
              v-for="group in ['全部工具', ...groups]"
              :key="group"
              :class="{ active: filter === group }"
              @click="filter = group"
            >
              {{ group === "ITS90 温标换算" ? "温标换算" : group
              }}<span>{{
                group === "全部工具"
                  ? catalog.tools.length
                  : catalog.tools.filter((t) => t.group === group).length
              }}</span>
            </button>
          </div>
          <label class="dashboard-search"
            ><svg viewBox="0 0 24 24">
              <circle cx="10.5" cy="10.5" r="6.5" />
              <path d="m16 16 4 4" /></svg
            ><input
              v-model="query"
              placeholder="搜索名称或功能"
              aria-label="搜索名称或功能"
          /></label>
        </div>
        <div class="catalog-heading">
          <h2>{{ filter }}</h2>
          <span>{{ filtered.length }} 项</span>
        </div>
        <div class="tool-grid">
          <button
            v-for="tool in filtered"
            :key="tool.id"
            class="tool-card"
            @click="open(tool)"
          >
            <div class="tool-card-top">
              <span class="tool-icon">{{ tool.icon || "工" }}</span>
              <div>
                <h3>{{ tool.name }}</h3>
                <span class="tool-category">{{ tool.group }}</span>
              </div>
            </div>
            <p>{{ tool.description }}</p>
            <span class="open-tool">打开工具 <span>→</span></span>
          </button>
        </div>
        <div v-if="!filtered.length" class="empty-state">
          <h2>没有找到匹配的工具</h2>
          <p>试试其他名称，或切换到全部工具。</p>
          <button
            class="button secondary"
            @click="
              query = '';
              filter = '全部工具';
            "
          >
            显示全部工具
          </button>
        </div>
      </template>
      <template v-else>
        <nav class="breadcrumb" aria-label="当前位置">
          <button @click="open(null)">← 工具箱</button><span>/</span
          ><span>{{ selected.group }}</span
          ><span>/</span><span>{{ selected.name }}</span>
        </nav>
        <div class="tool-heading">
          <span class="tool-icon large">{{ selected.icon || "工" }}</span>
          <div>
            <h1>{{ selected.name }}</h1>
            <p>{{ selected.description }}</p>
          </div>
          <button
            class="button subtle reset-button"
            :disabled="exporting"
            @click="reset"
          >
            恢复示例
          </button>
        </div>
        <GaugeTool
          v-if="selected.kind === 'gauge'"
          :key="selected.id"
          :engine="engine"
          :tool="selected"
          :state="current"
          @state="states[selected!.id] = $event"
        />
        <template v-else>
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
                  <FieldEditor
                    v-for="field in section.fields"
                    :key="selected.id + field.key"
                    :class="{
                      'full-width': !['number', 'text', 'select'].includes(
                        field.type,
                      ),
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
                </div>
                <details v-if="section.advanced" class="advanced-settings">
                  <summary>
                    <span>{{ section.advanced.title }}</span>
                    <span class="advanced-toggle" aria-hidden="true">
                      <span class="advanced-expand">展开</span
                      ><span class="advanced-collapse">收起</span>
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
                  <span>分度表与特性曲线</span
                  ><span>{{ current.division ? "收起 −" : "展开 ＋" }}</span>
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
                    ><span class="field-label">{{ label }}</span
                    ><NumericInput
                      :model-value="current[key] ?? ''"
                      :placeholder="
                        key === 'tableStep' ? '1' : '留空使用有效温区边界'
                      "
                      @update:model-value="
                        $event.trim()
                          ? change(key, $event)
                          : delete current[key]
                      "
                  /></label>
                </div>
              </section>
            </div>
            <div ref="resultPanel" class="output-panels" aria-live="polite">
              <section class="panel result-panel">
                <div class="section-heading">
                  <h2>计算结果</h2>
                  <span v-if="!busy && !error && result" class="result-ready"
                    ><i></i> 已更新</span
                  >
                </div>
                <div v-if="error" class="calculation-error">
                  <strong>请检查输入</strong>
                  <p>{{ error }}</p>
                </div>
                <div v-else-if="!result" class="result-placeholder">
                  {{ busy ? "正在计算…" : "填写参数后显示结果" }}
                </div>
                <template v-else
                  ><div v-if="result.equation" class="equation">
                    <span>拟合方程</span>
                    <p>y = {{ equation }}</p>
                  </div>
                  <div :class="['metric-grid', { 'is-updating': busy }]">
                    <div
                      v-for="(metric, j) in result.metrics"
                      :key="j"
                      class="metric"
                    >
                      <span>{{ metric.label }}</span
                      ><strong
                        >{{ display(metric.value)
                        }}<small v-if="metric.unit">
                          {{ metric.unit }}</small
                        ></strong
                      >
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
                      {{ exporting ? "正在导出…" : "导出 CSV" }}</button
                    ><button
                      class="button secondary"
                      :disabled="busy || exporting"
                      @click="copy"
                    >
                      复制完整结果
                    </button>
                  </div></template
                >
                <div v-if="exporting" class="export-progress">
                  <progress :value="progress" max="1"></progress
                  ><span>{{ Math.round(progress * 100) }}%</span
                  ><button class="button subtle" @click="cancelled = true">
                    取消导出
                  </button>
                </div>
                <p v-if="message" class="action-message" role="status">
                  {{ message }}
                </p>
              </section>
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
                  <span>{{ key }}</span
                  ><span>{{
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
                <strong v-if="t.delta !== null"
                  >±{{ display(t.delta) }} {{ t.unit }}</strong
                ><span v-else class="muted">当前温度超出适用范围</span>
                <p v-if="t.temperature">
                  温度界限：{{ display(t.temperature[0]) }}～{{
                    display(t.temperature[1])
                  }}
                  {{ t.unit }}
                </p>
                <p v-if="t.signal">
                  信号界限：{{ display(t.signal[0]) }}～{{
                    display(t.signal[1])
                  }}
                  {{ t.signalUnit }}
                </p>
                <small>{{ t.formula }}</small
                ><small>适用范围：{{ t.min }}～{{ t.max }} °C</small>
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
                <span>— 曲线</span
                ><span v-if="result.points.length">● 数据点</span>
              </div>
            </div>
            <CurveChart
              :curve="result.curve"
              :points="result.points"
              :x-label="axisX"
              :y-label="axisY"
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
            v-if="result?.rows.length"
            :key="selected.id"
            :headings="result.headings"
            :rows="result.rows"
            :unit-table="selected.kind === 'unit'"
            :total="result.total"
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
          />
        </template>
        <section v-if="activeNote.note || activeNote.formula" class="panel explanation">
          <h2>计算说明</h2>
          <p v-for="(paragraph, index) in explanationParagraphs" :key="index">
            {{ paragraph }}
          </p>
          <details v-if="activeNote.formula" :key="selected.id + activeNote.formula">
            <summary>详细公式与取值</summary>
            <p v-for="(line, index) in activeNote.formula.split(/\r?\n/u)" :key="index">{{ line }}</p>
          </details>
        </section>
      </template>
    </template>
  </main>
  <MobileActions
    :selected="!!selected"
    :hidden="mobileNav"
    @inputs="jumpInputs"
    @results="jumpResults"
  />
</template>
