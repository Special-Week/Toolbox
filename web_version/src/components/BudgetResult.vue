<script setup lang="ts">
// 递归展示不确定度分量及其合成结果。
import { display } from "../format";
export interface Detail {
  name: string;
  rawStandard: number;
  standard: number;
  formattedStandard: string;
  sensitivity: number;
  contribution: number;
  dof?: string;
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
          >标准不确定度 <b>{{ display(node.rawStandard) }}</b></span
        ><span
          >修约后 <b>{{ node.formattedStandard }}</b></span
        ><span
          >灵敏系数 <b>{{ display(node.sensitivity) }}</b></span
        ><span
          >贡献 <b>{{ display(node.contribution) }}</b></span
        ><span v-if="node.dof"
          >自由度 ν <b>{{ node.dof }}</b></span
        >
      </div>
      <BudgetResult v-if="node.children.length" :nodes="node.children" />
    </div>
  </div>
</template>
