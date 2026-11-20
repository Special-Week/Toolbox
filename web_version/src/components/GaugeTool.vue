<script setup lang="ts">
/** 字段和报告由共享 Rust 模型返回；宽屏上区左右双栏（左参数右摘要），下区全宽展示宽表格防裁切。 */
import { ref, watch, onUnmounted } from "vue";
import type { Engine } from "../engine";
import type { State, Tool } from "../types";
import ResultTable from "./ResultTable.vue";
import NumericInput from "./NumericInput.vue";
import SpecOptions from "./SpecOptions.vue";
import CalculationProcess from "./CalculationProcess.vue";
import { download } from "../format";

interface Option {
  value: string;
  label: string;
  columns?: [string, string, string];
}
interface Field {
  key: string;
  label: string;
  type: string;
  options?: Option[];
  suggestions?: Option[];
}
interface Report {
  process: string[];
  title: string;
  metrics: { label: string; value: string }[];
  tables: { title: string; headings: string[]; rows: string[][] }[];
  notes: string[];
  text: string;
}
interface Page {
  state: State;
  fields: Field[];
  report: Report | null;
  error: string;
  convention: string;
}

const props = defineProps<{ engine: Engine; tool: Tool; state: State }>();
const emit = defineEmits<{
  state: [state: State];
  toast: [message: string];
}>();

const page = ref<Page>();
const busy = ref(false);
const message = ref("");
let revision = 0;
let timer: ReturnType<typeof setTimeout>;
let normalized = "";

async function calculate(state: State, changed = "") {
  const version = ++revision;
  busy.value = true;
  try {
    const value = await props.engine.request<Page>("gaugePage", {
      tool: props.tool.id,
      input: state,
      changed,
    });
    if (version !== revision) return;
    page.value = value;
    normalized = JSON.stringify(value.state);
    emit("state", value.state);
  } catch (e) {
    if (version === revision)
      page.value = {
        state,
        fields: page.value?.fields ?? [],
        report: null,
        error: String(e),
        convention: page.value?.convention ?? "",
      };
  } finally {
    if (version === revision) busy.value = false;
  }
}

function change(key: string, value: string) {
  message.value = "";
  const state = { ...props.state, [key]: value };
  ++revision;
  busy.value = true;
  normalized = JSON.stringify(state);
  emit("state", state);
  clearTimeout(timer);
  timer = setTimeout(() => calculate(state, key), 120);
}

watch(
  () => JSON.stringify(props.state),
  (state) => {
    if (state !== normalized) {
      clearTimeout(timer);
      void calculate(JSON.parse(state));
    }
  },
  { immediate: true },
);

onUnmounted(() => {
  ++revision;
  clearTimeout(timer);
});

async function copy() {
  if (!page.value?.report) return;
  try {
    await navigator.clipboard.writeText(page.value.report.text);
    message.value = "已复制完整结果";
    emit("toast", "已复制量规完整结果");
  } catch {
    download(
      new Blob([page.value.report.text], { type: "text/plain;charset=utf-8" }),
      props.tool.id + ".txt",
    );
    message.value = "无法复制，已下载结果文本。";
  }
}

async function copyMetric(label: string, value: string) {
  try {
    await navigator.clipboard.writeText(value);
    emit("toast", `已复制 ${label}：${value}`);
  } catch {
    /* 剪贴板不可用时静默忽略 */
  }
}
</script>

<!-- prettier-ignore -->
<template>
  <div class="gauge-tool">
    <div class="gauge-workspace">
      <section class="panel gauge-input-panel">
        <div class="section-heading">
          <h2>计算参数</h2>
          <span v-if="busy" class="working-label">计算中…</span>
        </div>
        <div class="form-grid gauge-form-grid">
          <div v-for="field in page?.fields" :key="field.key" class="field">
            <label class="field-label" :for="`${tool.id}-${field.key}`">{{ field.label }}</label>
            <select
              v-if="field.type === 'select'"
              :id="`${tool.id}-${field.key}`"
              :value="state[field.key]"
              @change="
                change(field.key, ($event.target as HTMLSelectElement).value)
              "
              @wheel="($event.currentTarget as HTMLSelectElement).blur()"
            >
              <option
                v-for="option in field.options"
                :key="option.value"
                :value="option.value"
              >
                {{ option.label }}
              </option>
            </select>
            <div v-else class="gauge-input-row">
              <NumericInput
                v-if="field.key !== 'size'"
                :id="`${tool.id}-${field.key}`"
                :model-value="state[field.key]"
                @update:model-value="change(field.key, $event)"
              />
              <input
                v-else
                :id="`${tool.id}-${field.key}`"
                type="text"
                :value="state[field.key]"
                :aria-label="field.label"
                @input="
                  change(field.key, ($event.target as HTMLInputElement).value)
                "
              />
              <SpecOptions
                v-if="tool.id === 'gauge_unified' && field.key === 'size' && field.suggestions?.length"
                :options="field.suggestions"
                :selected="String(state[field.key] ?? '')"
                @select="change(field.key, $event)"
              />
              <select
                v-else-if="field.suggestions?.length"
                value=""
                aria-label="常用规格"
                @change="
                  change(field.key, ($event.target as HTMLSelectElement).value);
                  ($event.target as HTMLSelectElement).value = '';
                "
                @wheel="($event.currentTarget as HTMLSelectElement).blur()"
              >
                <option value="" disabled>常用规格</option>
                <option
                  v-for="option in field.suggestions"
                  :key="option.value"
                  :value="option.value"
                >
                  {{ option.label }}
                </option>
              </select>
            </div>
          </div>
        </div>
      </section>

      <div class="gauge-summary-col">
        <p v-if="page?.error" class="calculation-error">{{ page.error }}</p>
        <section v-else-if="page?.report" class="panel gauge-summary" :aria-busy="busy">
          <div class="section-heading">
            <div>
              <h2>{{ page.report.title }}</h2>
              <p class="muted">点击结果卡片可复制数值</p>
            </div>
            <button class="button secondary" :disabled="busy" @click="copy">
              复制完整结果
            </button>
          </div>
          <details v-if="page.report.notes.length" class="collapsible-note gauge-notes">
            <summary>⚠ 取值说明与适用限制 · {{ page.report.notes.length }} 项</summary>
            <p v-for="note in page.report.notes" :key="note">{{ note }}</p>
          </details>
          <div class="metric-grid gauge-metrics">
            <div
              v-for="m in page.report.metrics.slice(0, 6)"
              :key="m.label"
              class="metric copyable"
              role="button"
              tabindex="0"
              :title="`点击复制 ${m.label}`"
              @click="copyMetric(m.label, m.value)"
              @keydown.enter.prevent="copyMetric(m.label, m.value)"
            >
              <span>{{ m.label }}</span>
              <strong>{{ m.value }}</strong>
            </div>
          </div>
          <details
            v-if="page.report.metrics.length > 6"
            class="advanced-settings"
          >
            <summary>
              <span>几何与公差取值详情 · {{ page.report.metrics.length - 6 }} 项</span>
              <span class="advanced-toggle" aria-hidden="true">
                <span class="advanced-expand">展开</span>
                <span class="advanced-collapse">收起</span>
                <svg viewBox="0 0 20 20"><path d="m5 7 5 5 5-5" /></svg>
              </span>
            </summary>
            <dl class="gauge-details">
              <template v-for="m in page.report.metrics.slice(6)" :key="m.label">
                <dt>{{ m.label }}</dt>
                <dd>{{ m.value }}</dd>
              </template>
            </dl>
          </details>
          <p v-if="message" class="action-message" role="status">{{ message }}</p>
        </section>
      </div>
    </div>

    <!-- 宽结果表格统一 100% 全宽展示，彻底避免双栏裁切 -->
    <template v-if="!page?.error && page?.report">
      <section
        v-for="(table, i) in page.report.tables"
        :key="tool.id + i"
        class="gauge-result"
      >
        <ResultTable
          :title="table.title"
          :compact="true"
          :headings="table.headings"
          :rows="table.rows"
        />
      </section>
      <CalculationProcess :lines="page.report.process" />
    </template>
  </div>
</template>

<style scoped>
.gauge-tool {
  display: grid;
  gap: 16px;
  min-width: 0;
  scroll-margin-top: 100px;
}
.gauge-workspace {
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  gap: 16px;
  align-items: start;
  min-width: 0;
}
.gauge-input-panel,
.gauge-summary-col,
.gauge-summary {
  min-width: 0;
  margin-bottom: 0;
}
@media (min-width: 1024px) {
  .gauge-workspace {
    grid-template-columns: minmax(320px, 0.4fr) minmax(0, 0.6fr);
  }
  .gauge-form-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}
.gauge-summary {
  scroll-margin-top: 100px;
}
.gauge-input-row {
  display: flex;
  gap: 8px;
  min-width: 0;
}
.gauge-input-row input {
  min-width: 0;
  width: 100%;
}
.gauge-input-row select {
  width: 112px;
  flex: 0 0 112px;
}
.gauge-metrics strong,
.gauge-details dd {
  white-space: pre-line;
}
.gauge-details {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  gap: 8px 16px;
  font-size: 13px;
  margin-top: 10px;
}
.gauge-details dt {
  color: var(--muted);
}
.gauge-details dd {
  margin: 0;
  color: var(--ink);
  font-family: var(--mono);
  overflow-wrap: anywhere;
}
.gauge-result {
  min-width: 0;
}
.gauge-result :deep(.panel) {
  margin-bottom: 0;
}
.gauge-result :deep(td),
.gauge-result :deep(th) {
  text-align: center;
  white-space: pre;
}
@media (max-width: 600px) {
  .gauge-tool {
    gap: 12px;
  }
  .gauge-input-row select {
    width: 96px;
    flex-basis: 96px;
  }
  .gauge-details {
    font-size: 12px;
    gap: 8px;
  }
  .gauge-metrics {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
</style>
