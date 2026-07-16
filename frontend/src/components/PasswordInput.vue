<!--
  本文件拥有 frontend 各类密码字段共用的显示与隐藏交互。
  它只管理输入呈现和焦点，不校验、提交或持久化密码。
-->
<template>
  <div class="password-input">
    <input
      ref="inputElement"
      v-model="model"
      v-bind="attrs"
      :type="visible ? 'text' : 'password'"
    />
    <button
      class="password-input__toggle"
      type="button"
      :disabled="disabled"
      :aria-label="visible ? '隐藏密码' : '显示密码'"
      :aria-pressed="visible"
      :title="visible ? '隐藏密码' : '显示密码'"
      @click="toggleVisibility"
    >
      <svg v-if="visible" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
        <path d="M3 3l18 18" />
        <path d="M10.6 6.2A9.7 9.7 0 0 1 12 6c6 0 9.5 6 9.5 6a15.8 15.8 0 0 1-2.1 2.8" />
        <path d="M6.1 6.1C3.8 7.7 2.5 12 2.5 12s3.5 6 9.5 6a9.6 9.6 0 0 0 3.1-.5" />
      </svg>
      <svg v-else viewBox="0 0 24 24" aria-hidden="true" focusable="false">
        <path d="M2.5 12s3.5-6 9.5-6 9.5 6 9.5 6-3.5 6-9.5 6-9.5-6-9.5-6Z" />
        <circle cx="12" cy="12" r="2.5" />
      </svg>
    </button>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, useAttrs, watch } from "vue";

defineOptions({ inheritAttrs: false });

const model = defineModel<string>({ required: true });
const attrs = useAttrs();
const inputElement = ref<HTMLInputElement | null>(null);
const visible = ref(false);
const disabled = computed(
  () => attrs.disabled === "" || attrs.disabled === true || attrs.disabled === "disabled",
);

watch(model, (value) => {
  if (!value) {
    visible.value = false;
  }
});

/** 切换呈现类型后恢复原焦点和选区，避免用户继续输入时光标跳动。 */
async function toggleVisibility(): Promise<void> {
  const input = inputElement.value;
  const selectionStart = input?.selectionStart ?? null;
  const selectionEnd = input?.selectionEnd ?? null;

  visible.value = !visible.value;
  await nextTick();

  input?.focus({ preventScroll: true });
  if (input && selectionStart !== null && selectionEnd !== null) {
    input.setSelectionRange(selectionStart, selectionEnd);
  }
}
</script>
