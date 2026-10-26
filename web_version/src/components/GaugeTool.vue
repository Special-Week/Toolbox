<script setup lang="ts">
/** 字段和报告由共享 Rust 模型返回，网页仅负责输入、布局与复制。 */
import { ref, watch, onUnmounted } from "vue";
import type { Engine } from "../engine";
import type { State, Tool } from "../types";
import ResultTable from "./ResultTable.vue";
import NumericInput from "./NumericInput.vue";
import { download } from "../format";
interface Option {
  value: string;
  label: string;
}
interface Field {
  key: string;
  label: string;
  type: string;
  options?: Option[];
  suggestions?: Option[];
}
interface Report {
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
const emit = defineEmits<{ state: [state: State] }>();
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
  // 先使旧响应失效；等待输入停顿后再计算，空串等编辑状态仍原样保留。
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
    message.value = "已复制结果";
  } catch {
    download(
      new Blob([page.value.report.text], { type: "text/plain;charset=utf-8" }),
      props.tool.id + ".txt",
    );
    message.value = "无法复制，已下载结果文本。";
  }
}
</script>

<template>
  <div class="gauge-tool">
    <section class="panel">
      <div class="section-heading">
        <h2>计算参数</h2>
        <span v-if="busy" class="working-label">计算中…</span>
      </div>
      <div class="form-grid">
        <label v-for="field in page?.fields" :key="field.key" class="field">
          <span class="field-label">{{ field.label }}</span>
          <select
            v-if="field.type === 'select'"
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
              :model-value="state[field.key]"
              @update:model-value="change(field.key, $event)"
            />
            <input
              v-else
              type="text"
              :value="state[field.key]"
              :aria-label="field.label"
              @input="
                change(field.key, ($event.target as HTMLInputElement).value)
              "
            />
            <select
              v-if="field.suggestions?.length"
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
        </label>
      </div>
    </section>
    <p v-if="page?.error" class="calculation-error">{{ page.error }}</p>
    <template v-else-if="page?.report">
      <section class="panel gauge-summary" :aria-busy="busy">
        <div class="section-heading">
          <h2>{{ page.report.title }}</h2>
          <button class="button secondary" :disabled="busy" @click="copy">
            复制完整结果
          </button>
        </div>
        <p v-for="note in page.report.notes" :key="note" class="notice">
          {{ note }}
        </p>
        <div class="metric-grid gauge-metrics">
          <div
            v-for="m in page.report.metrics.slice(0, 6)"
            :key="m.label"
            class="metric"
          >
            <span>{{ m.label }}</span
            ><strong>{{ m.value }}</strong>
          </div>
        </div>
        <details
          v-if="page.report.metrics.length > 6"
          class="advanced-settings"
        >
          <summary>几何与公差取值详情</summary>
          <dl class="gauge-details">
            <template v-for="m in page.report.metrics.slice(6)" :key="m.label"
              ><dt>{{ m.label }}</dt>
              <dd>{{ m.value }}</dd></template
            >
          </dl>
        </details>
        <p v-if="message" role="status">{{ message }}</p>
      </section>
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
}
.gauge-details dt {
  color: var(--muted);
}
.gauge-details dd {
  margin: 0;
  overflow-wrap: anywhere;
}
.gauge-result {
  min-width: 0;
}
.gauge-result h2 {
  font-size: 15px;
  margin: 0 0 10px;
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
