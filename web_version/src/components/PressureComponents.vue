<script setup lang="ts">
import NumericInput from "./NumericInput.vue";
import { computed } from "vue";
import type { Catalog } from "../types";
const p = defineProps<{
  modelValue: Record<string, string>[];
  catalog: Catalog;
}>();
const emit = defineEmits<{
  "update:modelValue": [value: Record<string, string>[]];
}>();
const units = computed(
  () => p.catalog.tools.find((t) => t.id === "unit_pressure")!.units!,
);
function update(j: number, key: string, value: string) {
  const rows = p.modelValue.map((row) => ({ ...row }));
  rows[j][key] = value;
  emit("update:modelValue", rows);
}
</script>
<template>
  <div class="pressure-components">
    <div v-for="(row, j) in modelValue" :key="j" class="budget-root">
      <div class="array-toolbar">
        <strong>压力分量 {{ j + 1 }}</strong
        ><button
          class="button danger-text"
          :disabled="modelValue.length <= 1"
          @click="
            emit(
              'update:modelValue',
              modelValue.filter((_, index) => index !== j),
            )
          "
        >
          删除
        </button>
      </div>
      <div class="form-grid">
        <label class="field"
          ><span class="field-label">分量单位</span
          ><select
            :value="row.unit"
            @change="
              update(j, 'unit', ($event.target as HTMLSelectElement).value)
            "
            @wheel="($event.currentTarget as HTMLSelectElement).blur()"
          >
            <option
              v-for="(u, index) in units"
              :key="u.code"
              :value="String(index)"
            >
              {{ u.code }}
            </option>
          </select></label
        >
        <label class="field"
          ><span class="field-label">误差类型</span
          ><select
            :value="row.type"
            @change="
              update(j, 'type', ($event.target as HTMLSelectElement).value)
            "
            @wheel="($event.currentTarget as HTMLSelectElement).blur()"
          >
            <option value="0">相对误差（量程百分比）</option>
            <option value="1">绝对误差</option>
          </select></label
        >
        <template v-if="row.type === '0'"
          ><label class="field"
            ><span class="field-label">量程下限</span
            ><NumericInput
              :model-value="row.low"
              @update:model-value="update(j, 'low', $event)" /></label
          ><label class="field"
            ><span class="field-label">量程上限</span
            ><NumericInput
              :model-value="row.high"
              @update:model-value="update(j, 'high', $event)" /></label
        ></template>
        <label class="field"
          ><span class="field-label"
            >误差值 {{ row.type === "0" ? "(%)" : "" }}</span
          ><NumericInput
            :model-value="row.value"
            @update:model-value="update(j, 'value', $event)"
        /></label>
      </div>
    </div>
    <button
      class="button secondary"
      :disabled="modelValue.length >= 50"
      @click="
        emit('update:modelValue', [
          ...modelValue,
          { unit: '0', type: '0', low: '0', high: '100', value: '0.1' },
        ])
      "
    >
      ＋ 添加压力分量
    </button>
  </div>
</template>
