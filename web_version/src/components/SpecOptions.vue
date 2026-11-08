<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref } from "vue";

interface SpecOption {
  value: string;
  label: string;
  columns?: [string, string, string];
}

const props = defineProps<{ options: SpecOption[]; selected: string }>();
const emit = defineEmits<{ select: [value: string] }>();
const menu = ref<HTMLDetailsElement>();

function close(restoreFocus = false) {
  if (!menu.value?.open) return;
  menu.value.open = false;
  if (restoreFocus) menu.value.querySelector<HTMLElement>("summary")?.focus();
}
function choose(value: string) {
  emit("select", value);
  close(true);
}
function keydown(event: KeyboardEvent) {
  if (event.key === "Escape" && menu.value?.open) {
    event.preventDefault();
    close(true);
    return;
  }
  if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
  event.preventDefault();
  const root = menu.value;
  if (!root) return;
  root.open = true;
  void nextTick(() => {
    const buttons = Array.from(root.querySelectorAll<HTMLButtonElement>('[role="option"]'));
    if (!buttons.length) return;
    const current = buttons.findIndex((button) => button === document.activeElement);
    let index = current;
    if (event.key === "Home") index = 0;
    else if (event.key === "End") index = buttons.length - 1;
    else if (event.key === "ArrowDown") index = current < 0 ? 0 : Math.min(current + 1, buttons.length - 1);
    else index = current < 0 ? buttons.length - 1 : Math.max(current - 1, 0);
    buttons[index].focus();
    buttons[index].scrollIntoView({ block: "nearest" });
  });
}
function outside(event: PointerEvent) {
  if (event.target instanceof Node && !menu.value?.contains(event.target)) close();
}
function focusout(event: FocusEvent) {
  if (event.relatedTarget instanceof Node && !menu.value?.contains(event.relatedTarget)) close();
}
onMounted(() => document.addEventListener("pointerdown", outside));
onUnmounted(() => document.removeEventListener("pointerdown", outside));
</script>

<template>
  <details ref="menu" class="spec-menu" @keydown="keydown" @focusout="focusout">
    <summary aria-label="常用规格" aria-haspopup="listbox">常用规格</summary>
    <div class="spec-list" role="listbox" aria-label="常用规格">
      <button v-for="option in props.options" :key="option.value" type="button" role="option"
        :aria-selected="option.value === props.selected" tabindex="-1" @click="choose(option.value)">
        <template v-if="option.columns">
          <span v-for="(column, index) in option.columns" :key="index">{{ column }}</span>
        </template>
        <span v-else class="spec-fallback">{{ option.label }}</span>
      </button>
    </div>
  </details>
</template>

<style scoped>
.spec-menu { position: relative; flex: 0 0 112px; min-width: 0; }
.spec-menu summary {
  height: 42px; display: flex; align-items: center; justify-content: space-between; gap: 8px;
  padding: 0 12px; border: 1px solid #96a8c0; border-radius: 6px; background: #fafcff;
  color: #31425d; cursor: pointer; list-style: none;
}
.spec-menu summary::-webkit-details-marker { display: none; }
.spec-menu summary::after {
  content: ""; width: 7px; height: 7px; border-right: 1.5px solid currentColor;
  border-bottom: 1.5px solid currentColor; transform: rotate(45deg) translateY(-2px); flex-shrink: 0;
}
.spec-list {
  position: absolute; z-index: 20; top: calc(100% + 4px); right: 0;
  width: min(330px, calc(100vw - 32px)); max-height: 320px; overflow-y: auto;
  padding: 4px; border: 1px solid #dce5f1; border-radius: 6px; background: white;
  box-shadow: 0 6px 18px #23304326;
}
.spec-list button {
  display: grid; grid-template-columns: 4.7em 6.4em minmax(0, 1fr); gap: 12px; align-items: center;
  width: 100%; min-height: 34px; padding: 5px 8px; border: 0; border-radius: 4px;
  background: transparent; color: #233043; text-align: left; font: inherit;
}
.spec-list button:hover, .spec-list button:focus-visible { background: #eaf2ff; outline: none; }
.spec-list button[aria-selected="true"] { color: #2769dc; background: #edf3ff; }
.spec-list span { white-space: nowrap; }
.spec-fallback { grid-column: 1 / -1; }
@media (max-width: 860px) {
  .spec-menu { font-size: 16px; }
  .spec-menu summary { min-height: 40px; height: 40px; }
}
</style>
