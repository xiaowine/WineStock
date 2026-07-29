<!--
  本文件拥有当前用户修改密码页面，属于 frontend 鉴权页面层。
  它处理强制改密和主动改密，不拥有管理员重置密码或平台凭据存储。
-->
<template>
  <main class="auth-page">
    <section class="auth-panel" aria-labelledby="change-password-title">
      <header class="auth-header">
        <div class="brand-lockup">
          <BrandMark />
          <span class="brand-name">WineStock</span>
        </div>
        <div>
          <h1 id="change-password-title">{{ $route.meta.title }}</h1>
          <p>
            {{
              passwordChangeRequired
                ? "当前账号使用的是临时密码，修改后才能进入其它功能。"
                : "输入当前密码并设置一个新的登录密码。"
            }}
          </p>
        </div>
      </header>

      <p class="auth-account-context">
        当前账号：<strong>{{ username }}</strong>
      </p>

      <form class="auth-form" novalidate @submit.prevent="submitPasswordChange">
        <input
          class="visually-hidden"
          name="username"
          type="text"
          autocomplete="username"
          :value="username"
          tabindex="-1"
          aria-hidden="true"
          readonly
        />

        <FormField
          label="当前密码"
          control-id="change-current-password"
          validation-key="current_password"
          :error="currentPasswordError"
          v-slot="{ describedBy, invalid }"
        >
          <PasswordInput
            id="change-current-password"
            v-model="currentPassword"
            name="current_password"
            autocomplete="current-password"
            maxlength="256"
            autofocus
            :disabled="isSubmitting"
            :aria-invalid="invalid || undefined"
            :aria-describedby="describedBy"
          />
        </FormField>

        <FormField
          label="新密码"
          control-id="change-new-password"
          validation-key="new_password"
          :error="newPasswordError"
          hint="至少 8 个字符"
          v-slot="{ describedBy, invalid }"
        >
          <PasswordInput
            id="change-new-password"
            v-model="newPassword"
            name="new_password"
            autocomplete="new-password"
            minlength="8"
            maxlength="128"
            :disabled="isSubmitting"
            :aria-invalid="invalid || undefined"
            :aria-describedby="describedBy"
          />
        </FormField>

        <FormField
          label="确认新密码"
          control-id="change-new-password-confirmation"
          validation-key="new_password_confirmation"
          :error="newPasswordConfirmationError"
          v-slot="{ describedBy, invalid }"
        >
          <PasswordInput
            id="change-new-password-confirmation"
            v-model="newPasswordConfirmation"
            name="new_password_confirmation"
            autocomplete="new-password"
            maxlength="128"
            :disabled="isSubmitting"
            :aria-invalid="invalid || undefined"
            :aria-describedby="describedBy"
          />
        </FormField>

        <div v-if="errorMessage" class="form-error" role="alert">
          {{ errorMessage }}
        </div>

        <button class="primary-button primary-button--full" type="submit" :disabled="isSubmitting">
          {{ isSubmitting ? "正在修改…" : "修改密码" }}
        </button>
      </form>

      <div class="auth-page-actions">
        <RouterLink
          v-if="!passwordChangeRequired"
          class="secondary-button"
          :to="{ name: 'dashboard' }"
        >
          返回总览
        </RouterLink>
        <button
          class="secondary-button"
          type="button"
          :disabled="isSubmitting || isLoggingOut"
          @click="handleLogout"
        >
          {{ isLoggingOut ? "正在退出…" : "退出登录" }}
        </button>
      </div>
    </section>
  </main>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { changeOwnPassword } from "../api/auth";
import { ApiConfigurationError, ApiError, ApiNetworkError, ApiResponseError } from "../api/errors";
import {
  authSession,
  isLoggingOut,
  logoutAuthSession,
  markPasswordChangeCompleted,
  type LogoutResult,
} from "../auth/session";
import { AuthPersistenceError } from "../auth/storage";
import BrandMark from "../components/BrandMark.vue";
import { notice } from "../notices/notice";
import { resolvePostLoginLocation } from "../router/guards";
import PasswordInput from "../components/PasswordInput.vue";
import FormField from "../components/forms/FormField.vue";
import { useFormValidation } from "../composables/useFormValidation";

const router = useRouter();
const route = useRoute();
const currentPassword = ref("");
const newPassword = ref("");
const newPasswordConfirmation = ref("");
const isSubmitting = ref(false);
const errorMessage = ref("");
const fieldErrors = ref<Readonly<Record<string, readonly string[]>>>({});
useFormValidation(fieldErrors);

const username = computed(() => authSession.value?.user.username ?? "当前用户");
const passwordChangeRequired = computed(
  () => authSession.value?.user.password_change_required === true,
);
const currentPasswordError = computed(() => fieldErrors.value.current_password?.[0]);
const newPasswordError = computed(() => fieldErrors.value.new_password?.[0]);
const newPasswordConfirmationError = computed(
  () => fieldErrors.value.new_password_confirmation?.[0],
);

/** 校验并修改当前用户密码；成功后解除强制改密状态并恢复原内部目标。 */
async function submitPasswordChange(): Promise<void> {
  errorMessage.value = "";
  fieldErrors.value = validatePasswordChange(
    currentPassword.value,
    newPassword.value,
    newPasswordConfirmation.value,
  );
  if (Object.keys(fieldErrors.value).length > 0) {
    return;
  }

  isSubmitting.value = true;
  try {
    await changeOwnPassword({
      current_password: currentPassword.value,
      new_password: newPassword.value,
    });
    markPasswordChangeCompleted();
    await router.replace(resolvePostLoginLocation(router, route.query.redirect));
    notice.success("密码已修改");
  } catch (error) {
    applyPasswordChangeError(error);
  } finally {
    isSubmitting.value = false;
  }
}

/** 退出受限或普通会话；服务端吊销未确认时沿用登录页的本机退出提示。 */
async function handleLogout(): Promise<void> {
  errorMessage.value = "";

  let result: LogoutResult;
  try {
    result = await logoutAuthSession();
  } catch (error) {
    errorMessage.value =
      error instanceof AuthPersistenceError
        ? "无法清除本地登录状态，请检查浏览器存储权限后重试"
        : "退出失败，请稍后重试";
    notice.error(errorMessage.value);
    return;
  }

  await router.replace({
    name: "auth-entry",
    query: result === "local_only" ? { logout: "local_only" } : undefined,
  });
  if (result === "local_only") {
    notice.warning("本机已退出，但服务端会话吊销未确认");
  } else {
    notice.success("已退出登录");
  }
}

function validatePasswordChange(
  currentPasswordValue: string,
  newPasswordValue: string,
  confirmationValue: string,
): Readonly<Record<string, readonly string[]>> {
  const errors: Record<string, string[]> = {};
  if (!currentPasswordValue.trim()) {
    errors.current_password = ["请输入当前密码"];
  }
  if (!newPasswordValue.trim()) {
    errors.new_password = ["请输入新密码"];
  } else if (newPasswordValue.length < 8) {
    errors.new_password = ["新密码至少需要 8 个字符"];
  } else if (newPasswordValue === currentPasswordValue) {
    errors.new_password = ["新密码不能与当前密码相同"];
  }
  if (!confirmationValue) {
    errors.new_password_confirmation = ["请再次输入新密码"];
  } else if (confirmationValue !== newPasswordValue) {
    errors.new_password_confirmation = ["两次输入的新密码不一致"];
  }
  return errors;
}

function applyPasswordChangeError(error: unknown): void {
  if (error instanceof ApiError) {
    fieldErrors.value = error.fieldErrors;
    errorMessage.value =
      error.code === "invalid_credentials"
        ? "当前密码错误"
        : Object.keys(error.fieldErrors).length > 0
          ? "请检查输入内容"
          : error.message;
    notice.error(errorMessage.value);
    return;
  }
  if (error instanceof ApiConfigurationError) {
    errorMessage.value = error.message;
    notice.error(errorMessage.value);
    return;
  }
  if (error instanceof ApiNetworkError) {
    errorMessage.value = "无法连接到 WineStock 服务，请检查服务地址和运行状态";
    notice.error(errorMessage.value);
    return;
  }
  if (error instanceof ApiResponseError) {
    errorMessage.value = "服务响应格式无效，请检查服务版本";
    notice.error(errorMessage.value);
    return;
  }

  errorMessage.value = "修改密码失败，请稍后重试";
  notice.error(errorMessage.value);
}
</script>
