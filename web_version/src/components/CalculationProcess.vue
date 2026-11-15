<script setup lang="ts">
import { computed } from "vue";
import { processLines } from "../process";
import FormulaText from "./FormulaText.vue";
const props = defineProps<{ lines: string[] }>();
const displayed = computed(() => processLines(props.lines));
</script>

<template>
  <details v-if="displayed.length" class="collapsible-note calculation-process">
    <summary>计算过程</summary>
    <div class="process-lines">
      <p v-for="(line, index) in displayed" :key="index" :class="{ 'process-gap': !line }"><FormulaText :line="line" /></p>
    </div>
    <p>部分计算结果和后续代入值经过显示修约，实际计算精度不变。</p>
  </details>
</template>

<style scoped>
.process-lines p { margin: 0; white-space: pre-wrap; overflow-wrap: anywhere; }
.process-lines .process-gap { height: 1.3em; }
</style>
