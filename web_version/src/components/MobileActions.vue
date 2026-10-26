<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { advanceNumeric, nextNumeric, toggleSign } from "../mobile-input";
const p = defineProps<{ selected: boolean; hidden?: boolean }>();
const emit = defineEmits<{ inputs: []; results: [] }>();
const active = ref<HTMLInputElement | null>(null);
const editing = ref(false);
const inset = ref(0);
const caption = ref("数字输入");
const hasNext = ref(false);
const visible = computed(() => p.selected && !p.hidden);
function viewport() {
  const view = window.visualViewport;
  inset.value = view
    ? Math.max(0, window.innerHeight - view.height - view.offsetTop)
    : 0;
}
function focus() {
  const element = document.activeElement;
  editing.value =
    element instanceof HTMLElement &&
    element.matches('input,textarea,select,[contenteditable="true"]');
  active.value =
    element instanceof HTMLInputElement && element.hasAttribute("data-numeric")
      ? element
      : null;
  if (active.value) {
    caption.value =
      active.value.getAttribute("aria-label") ||
      active.value.closest("label")?.querySelector(".field-label")
        ?.textContent ||
      "数字输入";
    hasNext.value = !!nextNumeric(active.value);
  }
  viewport();
}
function blur() {
  queueMicrotask(focus);
}
onMounted(() => {
  document.addEventListener("focusin", focus);
  document.addEventListener("focusout", blur);
  window.visualViewport?.addEventListener("resize", viewport);
  window.visualViewport?.addEventListener("scroll", viewport);
  window.addEventListener("resize", viewport);
});
onUnmounted(() => {
  document.removeEventListener("focusin", focus);
  document.removeEventListener("focusout", blur);
  window.visualViewport?.removeEventListener("resize", viewport);
  window.visualViewport?.removeEventListener("scroll", viewport);
  window.removeEventListener("resize", viewport);
});
</script>
<template>
  <nav
    v-if="visible && active"
    class="mobile-actions input-accessory"
    :style="{ bottom: inset + 'px' }"
    aria-label="数字输入快捷操作"
  >
    <button
      type="button"
      class="button secondary"
      aria-label="切换当前数值的正负号"
      @pointerdown.prevent
      @click="active && toggleSign(active)"
    >
      ±
    </button>
    <span class="input-caption">{{ caption }}</span>
    <button
      type="button"
      class="button primary"
      @pointerdown.prevent
      @click="active && advanceNumeric(active)"
    >
      {{ hasNext ? "下一项" : "完成" }}
    </button>
    <button
      v-if="hasNext"
      type="button"
      class="button subtle"
      @pointerdown.prevent
      @click="active?.blur()"
    >
      完成
    </button>
  </nav>
  <nav
    v-else-if="visible && !editing"
    class="mobile-actions jump-actions"
    aria-label="工具页面快捷导航"
  >
    <button type="button" class="button secondary" @click="emit('inputs')">
      返回输入
    </button>
    <button type="button" class="button primary" @click="emit('results')">
      查看结果 ↓
    </button>
  </nav>
</template>
