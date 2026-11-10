<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, onUpdated, ref, watch } from "vue";
import { placeCards } from "../masonry";
const props = defineProps<{ count: number }>();
const root = ref<HTMLElement>();
const cards: (HTMLElement | null)[] = [];
let assignments: number[] = [];
let observer: ResizeObserver | undefined;
let frame = 0;
function schedule() {
  if (!frame) frame = requestAnimationFrame(layout);
}
function setCard(element: unknown, index: number) {
  const card = element instanceof HTMLElement ? element : null;
  if (cards[index] === card) return;
  if (cards[index]) observer?.unobserve(cards[index]!);
  cards[index] = card;
  if (card) observer?.observe(card);
  schedule();
}
function layout() {
  frame = 0;
  if (!root.value) return;
  const width = root.value.clientWidth;
  if (!width) return;
  const columns = width >= 760 && props.count > 1 ? 2 : 1;
  const cardWidth = columns === 2 ? (width - 12) / 2 : width;
  const current = cards.slice(0, props.count);
  if (current.length !== props.count || current.some(card => !card)) return;
  for (const card of current) card!.style.width = `${cardWidth}px`;
  const result = placeCards(current.map(card => card!.offsetHeight), assignments, columns);
  assignments = result.assignments;
  current.forEach((card, i) => {
    card!.style.top = `${result.positions[i].top}px`;
    card!.style.left = `${result.positions[i].column * (cardWidth + 12)}px`;
  });
  root.value.style.height = `${result.height}px`;
}
function remove(index: number) { assignments.splice(index, 1); }
defineExpose({ remove });
watch(() => props.count, () => nextTick(schedule));
onUpdated(schedule);
onMounted(() => {
  if (typeof ResizeObserver !== "undefined") {
    observer = new ResizeObserver(schedule);
    if (root.value) observer.observe(root.value);
    for (const card of cards) if (card) observer.observe(card);
  }
  window.addEventListener("resize", schedule);
  nextTick(schedule);
});
onUnmounted(() => {
  observer?.disconnect();
  window.removeEventListener("resize", schedule);
  if (frame) cancelAnimationFrame(frame);
});
</script>
<template>
  <div ref="root" class="masonry-cards" @toggle.capture="schedule">
    <div v-for="i in count" :key="i - 1" :ref="el => setCard(el, i - 1)" class="masonry-card">
      <slot :index="i - 1" />
    </div>
  </div>
</template>
<style scoped>
.masonry-cards { position: relative; min-width: 0; }
.masonry-card { position: absolute; top: 0; left: 0; box-sizing: border-box; }
.masonry-card :deep(.budget-root) { margin-bottom: 0; }
</style>
