<!--
  本文件拥有用户管理用户名修改表单和本地校验，属于 frontend 用户管理组件层。
  它不调用用户 API，也不持久化会话或密码。
-->
<template>
  <ModalDialog
    :open="Boolean(user)"
    :title="$t('users.updateUsername')"
    :description="$t('users.updateUsernameDescription')"
    :busy="submitting"
    @close="emit('close')"
  >
    <template #context>
      <div v-if="user" class="dialog-account-context">
        <span>{{ $t("users.currentUsername") }}</span>
        <strong :title="user.username">{{ user.username }}</strong>
      </div>
    </template>

    <form id="user-username-form" class="dialog-form" novalidate @submit.prevent="submit">
      <FormInput
        v-model="username"
        :label="$t('users.newUsername')"
        validation-key="username"
        :error="fieldErrors.username"
        name="username"
        type="text"
        autocomplete="off"
        maxlength="64"
        autofocus
        :disabled="submitting"
        required
      />
    </form>

    <template #actions>
      <button class="secondary-button" type="button" :disabled="submitting" @click="emit('close')">
        {{ $t("common.cancel") }}
      </button>
      <button class="primary-button" type="submit" form="user-username-form" :disabled="submitting">
        {{ submitting ? $t("users.saving") : $t("users.saveUsername") }}
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
import FormInput from "../forms/FormInput.vue";

const { t } = useI18n();

const props = defineProps<{
  user: UserAdminResponse | null;
  submitting: boolean;
  errorMessage: string;
  serverFieldErrors?: Readonly<Record<string, string>>;
}>();

const emit = defineEmits<{
  close: [];
  submit: [username: string];
}>();

const username = ref("");
const fieldErrors = ref<Record<string, string>>({});
useFormValidation(fieldErrors);

watch(
  () => props.user,
  (user) => {
    if (user) {
      username.value = user.username;
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
  const normalizedUsername = username.value.trim();
  const errors: Record<string, string> = {};
  if (!normalizedUsername) {
    errors.username = t("users.usernameRequired");
  } else if (normalizedUsername.length > 64) {
    errors.username = t("users.usernameTooLong");
  }
  fieldErrors.value = errors;
  if (Object.keys(errors).length > 0) {
    notice.warning(t("users.checkUsername"), { detail: Object.values(errors)[0] });
    return;
  }
  emit("submit", normalizedUsername);
}
</script>
