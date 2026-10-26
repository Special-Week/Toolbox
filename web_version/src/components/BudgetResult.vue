<script setup lang="ts">
import { display } from "../format";
export interface Detail {
  name: string;
  standard: number;
  sensitivity: number;
  contribution: number;
  children: Detail[];
}
defineProps<{ nodes: Detail[] }>();
</script>
<template>
  <div class="budget-result">
    <div v-for="(node, j) in nodes" :key="j" class="budget-detail">
      <strong>{{ node.name }}</strong>
      <div class="budget-numbers">
        <span
          >标准不确定度 <b>{{ display(node.standard) }}</b></span
        ><span
          >灵敏系数 <b>{{ display(node.sensitivity) }}</b></span
        ><span
          >贡献 <b>{{ display(node.contribution) }}</b></span
        >
      </div>
      <BudgetResult v-if="node.children.length" :nodes="node.children" />
    </div>
  </div>
</template>
