<!--
  本文件拥有用户管理用户名修改表单和本地校验，属于 frontend 用户管理组件层。
  它不调用用户 API，也不持久化会话或密码。
-->
<template>
  <ModalDialog
    :open="Boolean(user)"
    title="修改用户名"
    description="修改后，用户需要使用新用户名登录；现有登录会话不会失效。"
    :busy="submitting"
    @close="emit('close')"
  >
    <template #context>
      <div v-if="user" class="dialog-account-context">
        <span>当前用户名</span>
        <strong :title="user.username">{{ user.username }}</strong>
      </div>
    </template>

    <form id="user-username-form" class="dialog-form" novalidate @submit.prevent="submit">
      <FormInput
        v-model="username"
        label="新用户名"
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
        取消
      </button>
      <button class="primary-button" type="submit" form="user-username-form" :disabled="submitting">
        {{ submitting ? "正在保存…" : "保存用户名" }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import type { UserAdminResponse } from "../../api/users";
import { useFormValidation } from "../../composables/useFormValidation";
import { notice } from "../../notices/notice";
import ModalDialog from "../ModalDialog.vue";
import FormInput from "../forms/FormInput.vue";

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
    errors.username = "请输入用户名";
  } else if (normalizedUsername.length > 64) {
    errors.username = "用户名不能超过 64 个字符";
  }
  fieldErrors.value = errors;
  if (Object.keys(errors).length > 0) {
    notice.warning("请检查用户名", { detail: Object.values(errors)[0] });
    return;
  }
  emit("submit", normalizedUsername);
}
</script>
