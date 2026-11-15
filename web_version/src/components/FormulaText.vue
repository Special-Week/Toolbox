<script setup lang="ts">
import { computed } from "vue";
import katex from "katex";
import { formulaParts, formulaTex, parenthesizeNegativeSubtrahends } from "../process";

const props = defineProps<{ line: string }>();
const parts = computed(() => formulaParts(parenthesizeNegativeSubtrahends(props.line)).map((part) => {
  const tex = formulaTex(part);
  if (!tex) return { ...part, html: "" };
  try {
    return { ...part, html: katex.renderToString(tex, { throwOnError: true, trust: false }) };
  } catch {
    // 排版失败时保留原始表达式，不能让计算过程消失。
    return { ...part, html: "" };
  }
}));
</script>

<template>
  <template v-for="(part, index) in parts" :key="index"><span v-if="part.html" class="typeset-formula" :aria-label="part.denominator === undefined ? part.text : `${part.text} 除以 ${part.denominator}`" v-html="part.html"></span><template v-else>{{ part.denominator === undefined ? part.text : `${part.text}/${part.denominator}` }}</template></template>
</template>

<style scoped>
/* 保留 KaTeX 自带的基线，让公式与前后的正文自然对齐。 */
.typeset-formula { margin: 0 0.12em; }
/* 适度放大数学字体，避免小号分式与上下标难以辨认。 */
.typeset-formula :deep(.katex) { font-size: 1.28em; }
</style>
