<!-- 本组件只维护模板复制名称草稿，不调用复制 API。 -->
<template>
  <ModalDialog
    :open="Boolean(target)"
    :title="$t('templates.copyTemplate')"
    :description="target ? $t('templates.copyTemplateDescription', { name: target.name }) : undefined"
    :busy="submitting"
    compact
    @close="emit('close')"
  >
    <form :id="formId" class="dialog-form" novalidate @submit.prevent="submit">
      <FormInput
        v-model="name"
        :label="$t('templates.newTemplateNameLabel')"
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
        {{ $t('common.cancel') }}
      </button>
      <button class="primary-button" type="submit" :form="formId" :disabled="submitting">
        {{ submitting ? $t('templates.copyingNow') : $t('templates.copyAndEdit') }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { ref, useId, watch } from "vue";
import { useI18n } from "vue-i18n";
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

const { t } = useI18n();
const formId = `template-copy-form-${useId()}`;
const name = ref("");
const errors = ref<Record<string, string>>({});
useFormValidation(errors);

watch(
  () => props.target,
  (target) => {
    if (!target) return;
    name.value = t("templates.copyDraftName", { name: target.name });
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
    ? t("templates.newTemplateNameRequired")
    : normalized.length > 128
      ? t("templates.nameTooLong")
      : "";
  errors.value = error ? { name: error } : {};
  if (error) {
    notice.warning(t("templates.checkTemplateName"), { detail: error });
    return;
  }
  emit("submit", normalized);
}
</script>
