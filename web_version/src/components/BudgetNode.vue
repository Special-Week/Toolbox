<script setup lang="ts">
import NumericInput from "./NumericInput.vue";
import { computed, onMounted, ref } from "vue";
import type { Budget, BudgetDetail } from "../types";
import { display } from "../format";
import { newBudget } from "../schema";
const p = withDefaults(
  defineProps<{
    modelValue: Budget;
    depth: number;
    kinds?: string[];
    detail?: BudgetDetail;
    initiallyOpen?: boolean;
  }>(),
  {
    kinds: () => ["0", "1", "2"],
  },
);
const expanded = ref(true);
const summary = computed(() =>
  p.modelValue.kind === "1"
    ? "B 类 · " +
      [
        "均匀分布",
        "三角分布",
        "正态 k=2",
        "正态 k=3",
        "两点分布",
        "反正弦分布",
      ][Number(p.modelValue.distribution)]
    : p.modelValue.kind === "0"
      ? "A 类 · " + (p.modelValue.method === "1" ? "极差法" : "贝塞尔法")
      : "合成分量",
);
onMounted(() => {
  if (window.matchMedia("(max-width: 860px)").matches)
    expanded.value = !!p.initiallyOpen;
});
const emit = defineEmits<{ "update:modelValue": [value: Budget] }>();
function change(key: keyof Budget, value: unknown) {
  const node = { ...p.modelValue, [key]: value };
  if (key === "kind" && value === "2" && !node.children.length)
    node.children = [newBudget("子分量 1"), newBudget("子分量 2")];
  if (key === "method" && value === "1") node.values = node.values.slice(0, 9);
  emit("update:modelValue", node);
}
function sample(j: number, value: string) {
  const values = [...p.modelValue.values];
  values[j] = value;
  change("values", values);
}
function child(j: number, value: Budget) {
  const children = [...p.modelValue.children];
  children[j] = value;
  change("children", children);
}
</script>
<template>
  <details
    class="budget-node"
    :open="expanded"
    @toggle="expanded = ($event.target as HTMLDetailsElement).open"
  >
    <summary class="budget-summary">
      <span
        ><strong>{{ modelValue.name || "未命名分量" }}</strong
        ><small
          >{{ summary }} · c = {{ modelValue.sensitivity || "待填写" }}</small
        ></span
      >
      <span v-if="detail" class="budget-contribution"
        >贡献 {{ display(detail.contribution) }}</span
      >
      <svg class="budget-chevron" viewBox="0 0 20 20" aria-hidden="true">
        <path d="m5 7 5 5 5-5" />
      </svg>
    </summary>
    <div class="budget-body">
      <div class="form-grid">
        <label class="field"
          ><span class="field-label">分量名称</span
          ><input
            :value="modelValue.name"
            @input="change('name', ($event.target as HTMLInputElement).value)"
        /></label>
        <label v-if="kinds.length > 1" class="field"
          ><span class="field-label">类型</span
          ><select
            :value="modelValue.kind"
            @change="change('kind', ($event.target as HTMLSelectElement).value)"
            @wheel="($event.currentTarget as HTMLSelectElement).blur()"
          >
            <option v-if="kinds.includes('0')" value="0">A 类评定</option>
            <option v-if="kinds.includes('1')" value="1">B 类评定</option>
            <option v-if="depth < 4 && kinds.includes('2')" value="2">
              合成分量
            </option>
          </select></label
        >
        <label class="field"
          ><span class="field-label">灵敏系数</span
          ><NumericInput
            :model-value="modelValue.sensitivity"
            @update:model-value="change('sensitivity', $event)"
        /></label>
        <template v-if="modelValue.kind === '1'">
          <label class="field"
            ><span class="field-label">{{
              ["2", "3"].includes(String(modelValue.distribution))
                ? "扩展不确定度 U"
                : "区间半宽 a"
            }}</span
            ><NumericInput
              :model-value="modelValue.halfWidth"
              @update:model-value="change('halfWidth', $event)"
          /></label>
          <label class="field"
            ><span class="field-label">分布类型</span
            ><select
              :value="modelValue.distribution"
              @change="
                change(
                  'distribution',
                  ($event.target as HTMLSelectElement).value,
                )
              "
              @wheel="($event.currentTarget as HTMLSelectElement).blur()"
            >
              <option
                v-for="(label, j) in [
                  '均匀分布',
                  '三角分布',
                  '正态 k=2',
                  '正态 k=3',
                  '两点分布',
                  '反正弦分布',
                ]"
                :key="j"
                :value="String(j)"
              >
                {{ label }}
              </option>
            </select>
            <small
              >按该分量的信息依据选择分布；正态模式填写证书 U，按所选 k
              换算为标准不确定度。</small
            ></label
          >
        </template>
        <label v-else class="field"
          ><span class="field-label">{{
            modelValue.kind === "0" ? "A 类计算方法" : "合成方法"
          }}</span
          ><select
            :value="
              modelValue.kind === '0' ? modelValue.method : modelValue.combine
            "
            @change="
              change(
                modelValue.kind === '0' ? 'method' : 'combine',
                ($event.target as HTMLSelectElement).value,
              )
            "
            @wheel="($event.currentTarget as HTMLSelectElement).blur()"
          >
            <option value="0">
              {{ modelValue.kind === "0" ? "贝塞尔法" : "平方和根" }}
            </option>
            <option value="1">
              {{ modelValue.kind === "0" ? "极差法" : "最大绝对贡献" }}
            </option>
          </select></label
        >
      </div>
      <template v-if="modelValue.kind === '0'">
        <div class="array-toolbar">
          <span class="muted">测量值 · {{ modelValue.values.length }} 项</span>
          <div class="toolbar-actions">
            <button
              class="button subtle"
              :disabled="modelValue.values.length <= 2"
              @click="change('values', modelValue.values.slice(0, -1))"
            >
              − 减少</button
            ><button
              class="button secondary"
              :disabled="
                modelValue.values.length >= (modelValue.method === '1' ? 9 : 50)
              "
              @click="change('values', [...modelValue.values, ''])"
            >
              ＋ 添加
            </button>
          </div>
        </div>
        <div class="measurement-grid">
          <label v-for="(v, j) in modelValue.values" :key="j" class="field"
            ><span class="field-label">测量值 {{ j + 1 }}</span
            ><NumericInput
              :model-value="v"
              @update:model-value="sample(j, $event)"
          /></label>
        </div>
      </template>
      <template v-if="modelValue.kind === '2'">
        <div
          v-for="(node, j) in modelValue.children"
          :key="j"
          class="budget-child"
        >
          <BudgetNode
            :model-value="node"
            :depth="depth + 1"
            :detail="detail?.children[j]"
            @update:model-value="child(j, $event)"
          /><button
            class="button danger-text"
            :disabled="modelValue.children.length <= 1"
            @click="
              change(
                'children',
                modelValue.children.filter((_, index) => index !== j),
              )
            "
          >
            删除子分量
          </button>
        </div>
        <button
          class="button secondary"
          :disabled="modelValue.children.length >= 12"
          @click="
            change('children', [...modelValue.children, newBudget('子分量')])
          "
        >
          ＋ 添加子分量
        </button>
      </template>
    </div>
  </details>
</template>
