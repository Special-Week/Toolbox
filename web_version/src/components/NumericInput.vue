<script setup lang="ts">
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
