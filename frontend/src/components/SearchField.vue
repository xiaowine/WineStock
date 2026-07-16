<!-- 本组件拥有目录搜索输入、图标和防抖提交；它不请求数据或管理业务筛选状态。 -->
<template>
  <label class="search-field">
    <span :class="{ 'visually-hidden': hideLabel }">{{ label }}</span>
    <span class="search-field__control">
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <circle cx="11" cy="11" r="6.5" />
        <path d="m16 16 4 4" />
      </svg>
      <input
        :value="modelValue"
        :name="name"
        type="search"
        :maxlength="maxlength"
        :placeholder="placeholder"
        :autocomplete="autocomplete"
        :autofocus="autofocus"
        :disabled="disabled"
        @input="handleInput"
        @search="commitSearch"
      />
    </span>
  </label>
</template>

<script setup lang="ts">
import { onBeforeUnmount } from "vue";

const props = withDefaults(
  defineProps<{
    modelValue: string;
    label: string;
    name: string;
    placeholder: string;
    maxlength?: number;
    autocomplete?: string;
    disabled?: boolean;
    hideLabel?: boolean;
    debounceMs?: number;
    autofocus?: boolean;
  }>(),
  {
    maxlength: 128,
    autocomplete: "off",
    disabled: false,
    hideLabel: false,
    debounceMs: 280,
    autofocus: false,
  },
);

const emit = defineEmits<{
  "update:modelValue": [value: string];
  search: [value: string];
}>();

let searchTimer: number | undefined;

/** 输入时同步草稿并延迟搜索；清空输入立即恢复完整目录。 */
function handleInput(event: Event): void {
  const value = (event.target as HTMLInputElement).value;
  emit("update:modelValue", value);
  window.clearTimeout(searchTimer);
  if (value.trim() === "") {
    emit("search", "");
    return;
  }
  searchTimer = window.setTimeout(() => emit("search", value.trim()), props.debounceMs);
}

/** 浏览器清除按钮或 Enter 触发 search 事件时立即应用当前关键词。 */
function commitSearch(event: Event): void {
  window.clearTimeout(searchTimer);
  emit("search", (event.target as HTMLInputElement).value.trim());
}

onBeforeUnmount(() => window.clearTimeout(searchTimer));
</script>

<style lang="scss" src="./SearchField.scss"></style>
