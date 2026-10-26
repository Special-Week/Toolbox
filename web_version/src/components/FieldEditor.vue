<script setup lang="ts">
import NumericInput from "./NumericInput.vue";
import { computed, ref } from "vue";
import type { Catalog, Field, Budget, BudgetDetail } from "../types";
import BudgetNode from "./BudgetNode.vue";
import PressureComponents from "./PressureComponents.vue";
import { newBudget } from "../schema";
const p = defineProps<{
  field: Field;
  modelValue: unknown;
  catalog: Catalog;
  budgetDetails?: BudgetDetail[];
}>();
const emit = defineEmits<{ "update:modelValue": [value: unknown] }>();
const page = ref(1);
const pasteOpen = ref(false);
const pasteText = ref("");
const pasteError = ref("");
const rows = computed(() => (Array.isArray(p.modelValue) ? p.modelValue : []));
const pages = computed(() => Math.max(1, Math.ceil(rows.value.length / 10)));
const current = computed(() => Math.min(page.value, pages.value));
const start = computed(() => (current.value - 1) * 10);
const shown = computed(() => rows.value.slice(start.value, start.value + 10));
function write(index: number, value: unknown, column?: number) {
  // Vue 的响应式数组不能直接 structuredClone；此处只复制要编辑的数据层。
  const a = rows.value.map((row) => (Array.isArray(row) ? [...row] : row));
  if (column !== undefined) {
    a[index] = [...a[index]];
    a[index][column] = value;
  } else a[index] = value;
  emit("update:modelValue", a);
}
function resize(delta: number) {
  const a = rows.value.map((row) => (Array.isArray(row) ? [...row] : row));
  const n = a.length + delta;
  if (n < (p.field.min ?? 1) || n > (p.field.max ?? 50)) return;
  if (delta < 0) a.pop();
  else a.push(p.field.type === "points" ? ["", ""] : "");
  emit("update:modelValue", a);
  if (delta > 0) page.value = Math.ceil(n / 10);
}
function paste() {
  pasteError.value = "";
  const lines = pasteText.value
    .trim()
    .split(/\r?\n/)
    .filter((x) => x.trim());
  const parsed =
    p.field.type === "points"
      ? lines.map((line) => line.trim().split(/[\t,;，；\s]+/))
      : pasteText.value.trim().split(/[\s,;，；]+/);
  if (
    parsed.length < (p.field.min ?? 1) ||
    parsed.length > (p.field.max ?? 50) ||
    parsed.some((x) =>
      Array.isArray(x)
        ? x.length !== 2 ||
          x.some((v) => !v.trim() || !Number.isFinite(Number(v)))
        : !x.trim() || !Number.isFinite(Number(x)),
    )
  ) {
    pasteError.value = `请粘贴 ${p.field.min ?? 1}～${p.field.max ?? 50} ${p.field.type === "points" ? "行，每行两列数值" : "个有效数值"}`;
    return;
  }
  emit("update:modelValue", parsed);
  page.value = 1;
  pasteOpen.value = false;
}
function updateBudget(index: number, value: Budget) {
  write(index, value);
}
function addBudget() {
  if (rows.value.length < (p.field.max ?? 20))
    emit("update:modelValue", [
      ...rows.value,
      newBudget(`分量 ${rows.value.length + 1}`),
    ]);
}
</script>

<template>
  <label
    v-if="
      field.type === 'number' ||
      field.type === 'text' ||
      field.type === 'select'
    "
    class="field"
  >
    <span class="field-label">{{ field.label }}</span>
    <select
      v-if="field.type === 'select'"
      :value="modelValue"
      @change="
        emit('update:modelValue', ($event.target as HTMLSelectElement).value)
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
    <NumericInput
      v-else-if="field.type === 'number'"
      :model-value="modelValue"
      @update:model-value="emit('update:modelValue', $event)"
    />
    <input
      v-else
      :type="'text'"
      :value="modelValue"
      :spellcheck="false"
      autocomplete="off"
      @input="
        emit('update:modelValue', ($event.target as HTMLInputElement).value)
      "
    />
    <small v-if="field.hint">{{ field.hint }}</small>
  </label>
  <div
    v-else-if="field.type === 'points' || field.type === 'vector'"
    class="array-editor"
  >
    <div class="array-toolbar">
      <strong>{{ field.label }}</strong
      ><span class="muted"
        >{{ rows.length }} {{ field.type === "points" ? "组" : "项" }}</span
      >
      <div class="toolbar-actions">
        <button
          class="button subtle"
          type="button"
          @click="pasteOpen = !pasteOpen"
        >
          粘贴数据
        </button>
        <template v-if="field.min !== field.max"
          ><button
            class="button subtle"
            type="button"
            :disabled="rows.length <= (field.min ?? 1)"
            @click="resize(-1)"
          >
            − 减少</button
          ><button
            class="button secondary"
            type="button"
            :disabled="rows.length >= (field.max ?? 50)"
            @click="resize(1)"
          >
            ＋ 添加
          </button></template
        >
      </div>
    </div>
    <div v-if="pasteOpen" class="paste-panel">
      <p>
        支持从表格复制{{
          field.type === "points" ? "两列数据，每行一个点" : "一列数据"
        }}。
      </p>
      <textarea v-model="pasteText" rows="5" aria-label="粘贴数据"></textarea>
      <p v-if="pasteError" class="error-text">{{ pasteError }}</p>
      <button class="button primary" type="button" @click="paste">
        应用数据
      </button>
    </div>
    <div v-if="field.type === 'points'" class="point-header">
      <span>序号</span><span>x</span><span>y</span>
    </div>
    <div
      v-for="(row, j) in shown"
      :key="start + j"
      :class="field.type === 'points' ? 'point-row' : 'vector-row'"
    >
      <span class="row-index">{{
        field.labels?.[start + j] ?? start + j + 1
      }}</span>
      <template v-if="field.type === 'points'">
        <NumericInput
          type="text"
          :model-value="row[0]"
          :aria-label="`${field.label} ${start + j + 1} x`"
          autocomplete="off"
          @update:model-value="write(start + j, $event, 0)"
        />
        <NumericInput
          type="text"
          :model-value="row[1]"
          :aria-label="`${field.label} ${start + j + 1} y`"
          autocomplete="off"
          @update:model-value="write(start + j, $event, 1)"
        />
      </template>
      <NumericInput
        v-else
        type="text"
        :model-value="field.locked?.includes(start + j) ? '0' : row"
        :disabled="field.locked?.includes(start + j)"
        :aria-label="`${field.label} ${start + j + 1}`"
        autocomplete="off"
        @update:model-value="write(start + j, $event)"
      />
    </div>
    <nav v-if="pages > 1" class="pagination" aria-label="输入数据分页">
      <button
        class="button subtle"
        :disabled="current <= 1"
        @click="page = current - 1"
      >
        上一页</button
      ><span>{{ current }} / {{ pages }}</span
      ><button
        class="button subtle"
        :disabled="current >= pages"
        @click="page = current + 1"
      >
        下一页
      </button>
    </nav>
  </div>
  <div v-else-if="field.type === 'budget'" class="budget-editor">
    <div class="array-toolbar">
      <strong>{{ field.label }} · {{ rows.length }} 项</strong
      ><button
        class="button secondary"
        :disabled="rows.length >= (field.max ?? 20)"
        @click="addBudget"
      >
        ＋ 添加分量
      </button>
    </div>
    <div v-for="(node, j) in rows" :key="j" class="budget-root">
      <BudgetNode
        :model-value="node as Budget"
        :depth="0"
        :kinds="field.budgetKinds"
        :detail="budgetDetails?.[j]"
        :initially-open="j === 0"
        @update:model-value="updateBudget(j, $event)"
      /><button
        class="button danger-text"
        :disabled="rows.length <= 1"
        @click="
          emit(
            'update:modelValue',
            rows.filter((_, index) => index !== j),
          )
        "
      >
        删除此分量
      </button>
    </div>
  </div>
  <PressureComponents
    v-else-if="field.type === 'pressure'"
    :model-value="rows"
    :catalog="catalog"
    @update:model-value="emit('update:modelValue', $event)"
  />
</template>
