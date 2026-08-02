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
        :error="errors.name"
        maxlength="128"
        autocomplete="off"
        autofocus
        required
        :disabled="submitting"
      />
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
import { useFormValidation } from "../../composables/useFormValidation";
import { notice } from "../../notices/notice";
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
const errors = ref<Record<string, string>>({});
useFormValidation(errors);

watch(
  () => props.target,
  (target) => {
    if (!target) return;
    name.value = `${target.name} 副本`;
    errors.value = props.fieldError ? { name: props.fieldError } : {};
  },
);

watch(
  () => props.fieldError,
  (value) => {
    errors.value = value ? { name: value } : {};
  },
);

function submit(): void {
  const normalized = name.value.trim();
  const error = !normalized
    ? "请输入新模板名称"
    : normalized.length > 128
      ? "模板名称不能超过 128 个字符"
      : "";
  errors.value = error ? { name: error } : {};
  if (error) {
    notice.warning("请检查模板名称", { detail: error });
    return;
  }
  emit("submit", normalized);
}
</script>
