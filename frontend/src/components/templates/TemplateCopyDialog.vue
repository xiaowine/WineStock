<!-- 本组件只维护模板复制名称草稿，不调用复制 API。 -->
<template>
  <ModalDialog
    :open="Boolean(target)"
    title="复制模板"
    :description="target ? `以“${target.name}”的完整字段结构创建副本。` : undefined"
    :busy="submitting"
    compact
    @close="emit('close')"
  >
    <form :id="formId" class="dialog-form" novalidate @submit.prevent="submit">
      <FormInput
        v-model="name"
        label="新模板名称"
        validation-key="name"
        :error="nameError"
        maxlength="128"
        autocomplete="off"
        autofocus
        required
        :disabled="submitting"
      />
      <p v-if="errorMessage" class="form-error" role="alert">{{ errorMessage }}</p>
    </form>
    <template #actions>
      <button class="secondary-button" type="button" :disabled="submitting" @click="emit('close')">
        取消
      </button>
      <button class="primary-button" type="submit" :form="formId" :disabled="submitting">
        {{ submitting ? "正在复制…" : "复制并编辑" }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { ref, useId, watch } from "vue";
import ModalDialog from "../ModalDialog.vue";
import FormInput from "../forms/FormInput.vue";

export interface TemplateCopyTarget {
  id: number;
  name: string;
}

const props = defineProps<{
  target: TemplateCopyTarget | null;
  submitting: boolean;
  errorMessage: string;
  fieldError: string;
}>();

const emit = defineEmits<{
  close: [];
  submit: [name: string];
}>();

const formId = `template-copy-form-${useId()}`;
const name = ref("");
const nameError = ref("");

watch(
  () => props.target,
  (target) => {
    if (!target) return;
    name.value = `${target.name} 副本`;
    nameError.value = props.fieldError;
  },
);

watch(
  () => props.fieldError,
  (value) => {
    nameError.value = value;
  },
);

function submit(): void {
  const normalized = name.value.trim();
  nameError.value = !normalized
    ? "请输入新模板名称"
    : normalized.length > 128
      ? "模板名称不能超过 128 个字符"
      : "";
  if (!nameError.value) emit("submit", normalized);
}
</script>
