<script setup lang="ts">
import { computed, ref } from "vue";
import NumericInput from "./NumericInput.vue";

const props = defineProps<{ values: unknown; sensitivities: unknown }>();
const emit = defineEmits<{
  "update:values": [value: unknown[]];
  "update:sensitivities": [value: unknown[]];
}>();
const values = computed(() => Array.isArray(props.values) ? props.values : []);
const sensitivities = computed(() => Array.isArray(props.sensitivities) ? props.sensitivities : []);
const page = ref(1);
const pages = computed(() => Math.max(1, Math.ceil(values.value.length / 10)));
const current = computed(() => Math.min(page.value, pages.value));
const start = computed(() => (current.value - 1) * 10);
const shown = computed(() => values.value.slice(start.value, start.value + 10));
const pasteColumn = ref<"values" | "sensitivities" | null>(null);
const pasteText = ref("");
const pasteError = ref("");

function write(column: "values" | "sensitivities", index: number, value: string) {
  const next = [...(column === "values" ? values.value : sensitivities.value)];
  while (column === "sensitivities" && next.length < values.value.length) next.push("1");
  next[index] = value;
  if (column === "values") emit("update:values", next);
  else emit("update:sensitivities", next);
}
function resize(delta: number) {
  const next = [...values.value];
  if (next.length + delta < 1 || next.length + delta > 50) return;
  if (delta < 0) next.pop();
  else next.push("");
  emit("update:values", next);
  if (delta > 0) page.value = Math.ceil(next.length / 10);
}
function synchronize() {
  emit("update:sensitivities", values.value.map((_, i) => sensitivities.value[i] ?? "1"));
}
function openPaste(column: "values" | "sensitivities") {
  pasteColumn.value = pasteColumn.value === column ? null : column;
  pasteText.value = "";
  pasteError.value = "";
}
function paste() {
  const parsed = pasteText.value.trim().split(/[\s,;，；]+/);
  if (parsed.length < 1 || parsed.length > 50 || parsed.some(v => !v || !Number.isFinite(Number(v)))) {
    pasteError.value = "请粘贴 1～50 个有效数值";
    return;
  }
  if (pasteColumn.value === "sensitivities") {
    if (parsed.length !== values.value.length) {
      pasteError.value = `灵敏系数需要 ${values.value.length} 项，与误差分量数量一致`;
      return;
    }
    emit("update:sensitivities", parsed);
  } else {
    emit("update:values", parsed);
  }
  page.value = 1;
  pasteColumn.value = null;
}
</script>

<template>
  <div class="array-editor">
    <div class="array-toolbar">
      <strong>误差分量</strong><span class="muted">{{ values.length }} 项</span>
      <div class="toolbar-actions">
        <button class="button subtle" type="button" @click="openPaste('values')">粘贴误差</button>
        <button class="button subtle" type="button" @click="openPaste('sensitivities')">粘贴系数</button>
        <button class="button subtle" type="button" :disabled="values.length <= 1" @click="resize(-1)">− 减少</button>
        <button class="button secondary" type="button" :disabled="values.length >= 50" @click="resize(1)">＋ 添加</button>
      </div>
    </div>
    <div v-if="pasteColumn" class="paste-panel">
      <p>支持从表格复制一列{{ pasteColumn === 'values' ? '误差分量' : '灵敏系数' }}。</p>
      <textarea v-model="pasteText" rows="5" aria-label="粘贴数据"></textarea>
      <p v-if="pasteError" class="error-text">{{ pasteError }}</p>
      <button class="button primary" type="button" @click="paste">应用数据</button>
    </div>
    <p v-if="sensitivities.length !== values.length" class="error-text">
      灵敏系数与误差分量的数量不一致。
      <button class="button subtle" type="button" @click="synchronize">同步数量</button>
      缺少的系数填 1，多余的系数将移除。
    </p>
    <div class="point-header"><span>序号</span><span>误差分量</span><span>灵敏系数</span></div>
    <div v-for="(value, j) in shown" :key="start + j" class="point-row">
      <span class="row-index">{{ start + j + 1 }}</span>
      <NumericInput :model-value="value" :aria-label="`误差分量 ${start + j + 1}`"
                    @update:model-value="write('values', start + j, $event)" />
      <NumericInput :model-value="sensitivities[start + j] ?? ''" :aria-label="`灵敏系数 ${start + j + 1}`"
                    @update:model-value="write('sensitivities', start + j, $event)" />
    </div>
    <nav v-if="pages > 1" class="pagination" aria-label="输入数据分页">
      <button class="button subtle" :disabled="current <= 1" @click="page = current - 1">上一页</button>
      <span>{{ current }} / {{ pages }}</span>
      <button class="button subtle" :disabled="current >= pages" @click="page = current + 1">下一页</button>
    </nav>
  </div>
</template>
