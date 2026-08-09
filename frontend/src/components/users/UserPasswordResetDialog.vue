<!--
  本文件拥有管理员设置临时密码的输入和本地校验，属于 frontend 用户管理组件层。
  它不调用密码重置 API，也不持久化任何密码。
-->
<template>
  <ModalDialog
    :open="Boolean(user)"
    :title="$t('users.setTemporaryPassword')"
    :busy="submitting"
    @close="emit('close')"
  >
    <template #context>
      <div v-if="user" class="dialog-account-context">
        <span>{{ $t("users.targetUser") }}</span>
        <strong :title="user.username">{{ user.username }}</strong>
      </div>
    </template>

    <div class="dialog-content">
      <p class="confirmation-copy">{{ $t("users.temporaryPasswordDescription") }}</p>

      <form id="user-password-reset-form" class="dialog-form" novalidate @submit.prevent="submit">
        <FormField
          :label="$t('users.temporaryPassword')"
          control-id="user-temporary-password"
          validation-key="password"
          :error="fieldErrors.password"
          v-slot="{ describedBy, invalid }"
        >
          <PasswordInput
            id="user-temporary-password"
            v-model="password"
            name="temporary_password"
            autocomplete="off"
            minlength="8"
            maxlength="128"
            autofocus
            :disabled="submitting"
            :aria-invalid="invalid || undefined"
            :aria-describedby="describedBy"
          />
        </FormField>

        <FormField
          :label="$t('users.confirmTemporaryPassword')"
          control-id="user-temporary-password-confirmation"
          validation-key="confirmation"
          :error="fieldErrors.confirmation"
          v-slot="{ describedBy, invalid }"
        >
          <PasswordInput
            id="user-temporary-password-confirmation"
            v-model="confirmation"
            name="temporary_password_confirmation"
            autocomplete="off"
            maxlength="128"
            :disabled="submitting"
            :aria-invalid="invalid || undefined"
            :aria-describedby="describedBy"
          />
        </FormField>
      </form>
    </div>

    <template #actions>
      <button class="secondary-button" type="button" :disabled="submitting" @click="emit('close')">
        {{ $t("common.cancel") }}
      </button>
      <button
        class="primary-button"
        type="submit"
        form="user-password-reset-form"
        :disabled="submitting"
      >
        {{ submitting ? $t("users.settingTemporaryPassword") : $t("users.setTemporaryPassword") }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { UserAdminResponse } from "../../api/users";
import { useFormValidation } from "../../composables/useFormValidation";
import { notice } from "../../notices/notice";
import ModalDialog from "../ModalDialog.vue";
import PasswordInput from "../PasswordInput.vue";
import FormField from "../forms/FormField.vue";

const { t } = useI18n();

const props = defineProps<{
  user: UserAdminResponse | null;
  submitting: boolean;
  errorMessage: string;
  serverFieldErrors?: Readonly<Record<string, string>>;
}>();

const emit = defineEmits<{
  close: [];
  submit: [password: string];
}>();

const password = ref("");
const confirmation = ref("");
const fieldErrors = ref<Record<string, string>>({});
useFormValidation(fieldErrors);

watch(
  () => props.user,
  (user) => {
    if (user) {
      password.value = "";
      confirmation.value = "";
      fieldErrors.value = {};
    }
  },
);

watch(
  () => props.serverFieldErrors,
  (errors) => {
    if (props.user) fieldErrors.value = { ...errors };
  },
  { deep: true },
);

function submit(): void {
  const errors: Record<string, string> = {};
  if (!password.value) {
    errors.password = t("users.temporaryPasswordRequired");
  } else if (password.value.length < 8) {
    errors.password = t("users.passwordMinLength");
  }
  if (!confirmation.value) {
    errors.confirmation = t("users.confirmTemporaryPasswordRequired");
  } else if (confirmation.value !== password.value) {
    errors.confirmation = t("users.passwordMismatch");
  }
  fieldErrors.value = errors;
  if (Object.keys(errors).length > 0) {
    notice.warning(t("users.checkTemporaryPassword"), { detail: Object.values(errors)[0] });
    return;
  }
  emit("submit", password.value);
}
</script>
