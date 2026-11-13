<script setup lang="ts">
// 数字输入保留编辑中的原始文本，回车时移动到下一项。
import { advanceNumeric } from "../mobile-input";
defineProps<{ modelValue?: unknown; disabled?: boolean }>();
const emit = defineEmits<{ "update:modelValue": [value: string] }>();
</script>
<template>
  <input
    type="text"
    inputmode="decimal"
    enterkeyhint="next"
    autocomplete="off"
    :spellcheck="false"
    data-numeric
    :disabled="disabled"
    :value="modelValue ?? ''"
    @input="
      emit('update:modelValue', ($event.target as HTMLInputElement).value)
    "
    @keydown.enter.prevent="advanceNumeric($event.target as HTMLInputElement)"
  />
</template>
