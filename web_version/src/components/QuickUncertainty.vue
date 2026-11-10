<script setup lang="ts">
import { computed, ref } from "vue";
import type { Budget, BudgetDetail, Catalog, Field, State } from "../types";
import { newBudget } from "../schema";
import FieldEditor from "./FieldEditor.vue";
import BudgetNode from "./BudgetNode.vue";
import MasonryCards from "./MasonryCards.vue";
const props = defineProps<{ state: State; fields: Field[]; catalog: Catalog; details: BudgetDetail[] }>();
const emit = defineEmits<{ change: [key: string, value: unknown] }>();
const masonry = ref<InstanceType<typeof MasonryCards>>();
const rows = computed(() => Array.isArray(props.state.bComponents) ? props.state.bComponents as Budget[] : []);
const samples = computed(() => Array.isArray(props.state.aValues) ? props.state.aValues.length : 0);
const aFields = computed(() => props.fields.filter(field => ["aValues", "aAverageCount", "aSensitivity", "aDigits", "aRounding"].includes(field.key)
  && (samples.value > 0 || !["aAverageCount", "aDigits", "aRounding"].includes(field.key))));
function update(index: number, value: Budget) {
  const next = [...rows.value]; next[index] = value; emit("change", "bComponents", next);
}
function add() {
  if (rows.value.length < 50) emit("change", "bComponents", [...rows.value, newBudget(`B 类分量 ${rows.value.length + 1}`)]);
}
function remove(index: number) {
  masonry.value?.remove(index + 1);
  emit("change", "bComponents", rows.value.filter((_, j) => j !== index));
}
</script>
<template>
  <div class="quick-uncertainty full-width">
    <div class="array-toolbar"><strong>B 类分量 · {{ rows.length }} 项</strong>
      <button class="button secondary" type="button" :disabled="rows.length >= 50" @click="add">＋ 添加分量</button>
    </div>
    <MasonryCards ref="masonry" :count="1 + rows.length" v-slot="{ index }">
      <div class="budget-root" v-if="index === 0">
        <strong>A 类评定数据</strong>
        <div class="form-grid quick-fields">
          <FieldEditor v-for="field in aFields" :key="field.key" :class="{ 'full-width': field.type === 'vector' }"
            :field="field" :model-value="state[field.key]" :catalog="catalog"
            @update:model-value="emit('change', field.key, $event)" />
        </div>
      </div>
      <div v-else class="budget-root">
        <BudgetNode :model-value="rows[index - 1]" :depth="0" :kinds="['1']" :detail="details[index]"
          :initially-open="index === 1" @update:model-value="update(index - 1, $event)" />
        <button class="button danger-text" type="button" @click="remove(index - 1)">删除此分量</button>
      </div>
    </MasonryCards>
  </div>
</template>
<style scoped>.quick-fields { margin-top: 16px; }</style>
