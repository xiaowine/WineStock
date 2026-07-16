<!--
  本文件拥有首个用户注册页面，属于 frontend 鉴权页面层。
  它在桌面和移动视口共用同一注册流程，不负责后续用户管理或平台安全存储。
-->
<template>
  <main class="auth-page">
    <section class="auth-panel" aria-labelledby="register-title">
      <header class="auth-header">
        <div class="brand-lockup">
          <span class="brand-mark">W</span>
          <span class="brand-name">WineStock</span>
        </div>
        <div>
          <h1 id="register-title">{{ $route.meta.title }}</h1>
          <p>首个用户无需登录即可创建，并会自动获得全部内置权限。</p>
        </div>
      </header>

      <form class="auth-form" novalidate @submit.prevent="submitRegistration">
        <FormInput
          v-model="username"
          label="用户名"
          validation-key="username"
          :error="usernameError"
          name="username"
          type="text"
          autocomplete="username"
          maxlength="64"
          autofocus
          :disabled="isSubmitting"
        />

        <FormField
          label="密码"
          control-id="register-password"
          validation-key="password"
          :error="passwordError"
          v-slot="{ describedBy, invalid }"
        >
          <PasswordInput
            id="register-password"
            v-model="password"
            name="password"
            autocomplete="new-password"
            maxlength="256"
            :disabled="isSubmitting"
            :aria-invalid="invalid || undefined"
            :aria-describedby="describedBy"
          />
        </FormField>

        <FormField
          label="确认密码"
          control-id="register-password-confirmation"
          validation-key="password_confirmation"
          :error="passwordConfirmationError"
          v-slot="{ describedBy, invalid }"
        >
          <PasswordInput
            id="register-password-confirmation"
            v-model="passwordConfirmation"
            name="password_confirmation"
            autocomplete="new-password"
            maxlength="256"
            :disabled="isSubmitting"
            :aria-invalid="invalid || undefined"
            :aria-describedby="describedBy"
          />
        </FormField>

        <div v-if="errorMessage" class="form-error" role="alert">
          {{ errorMessage }}
        </div>

        <button class="primary-button primary-button--full" type="submit" :disabled="isSubmitting">
          {{ isSubmitting ? "正在创建…" : "创建并进入 WineStock" }}
        </button>
      </form>

      <div class="auth-page-switch">
        <span>服务中已经存在用户？</span>
        <RouterLink :to="{ name: 'login' }">返回登录</RouterLink>
      </div>

      <p class="auth-runtime-note">注册成功后会使用同一组凭据自动登录当前服务。</p>
    </section>
  </main>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { useRouter } from "vue-router";
import { login, registerInitialUser } from "../api/auth";
import { ApiConfigurationError, ApiError, ApiNetworkError, ApiResponseError } from "../api/errors";
import { resolveApiClientMetadata } from "../api/runtime-config";
import { establishAuthSession } from "../auth/session";
import { AuthPersistenceError } from "../auth/storage";
import { notice } from "../notices/notice";
import PasswordInput from "../components/PasswordInput.vue";
import FormField from "../components/forms/FormField.vue";
import FormInput from "../components/forms/FormInput.vue";
import { useFormValidation } from "../composables/useFormValidation";

const router = useRouter();
const username = ref("");
const password = ref("");
const passwordConfirmation = ref("");
const isSubmitting = ref(false);
const errorMessage = ref("");
const fieldErrors = ref<Readonly<Record<string, readonly string[]>>>({});
useFormValidation(fieldErrors);

const usernameError = computed(() => fieldErrors.value.username?.[0]);
const passwordError = computed(() => fieldErrors.value.password?.[0]);
const passwordConfirmationError = computed(() => fieldErrors.value.password_confirmation?.[0]);

/** 创建首个用户后立即登录；注册成功但登录失败时提示改用登录页，避免重复注册。 */
async function submitRegistration(): Promise<void> {
  errorMessage.value = "";
  fieldErrors.value = validateRegistrationInput(
    username.value,
    password.value,
    passwordConfirmation.value,
  );
  if (Object.keys(fieldErrors.value).length > 0) {
    return;
  }

  isSubmitting.value = true;
  let registrationCompleted = false;
  try {
    const normalizedUsername = username.value.trim();
    await registerInitialUser({
      username: normalizedUsername,
      password: password.value,
    });
    registrationCompleted = true;

    const metadata = resolveApiClientMetadata();
    const response = await login({
      username: normalizedUsername,
      password: password.value,
      device_name: metadata.deviceName,
      client_kind: metadata.clientKind,
      version: metadata.appVersion,
    });

    establishAuthSession(response);
    await router.replace({ name: "dashboard" });
    notice.success("首个用户已创建并登录");
  } catch (error) {
    applyRegistrationError(error, registrationCompleted);
  } finally {
    isSubmitting.value = false;
  }
}

function validateRegistrationInput(
  usernameValue: string,
  passwordValue: string,
  passwordConfirmationValue: string,
): Readonly<Record<string, readonly string[]>> {
  const errors: Record<string, string[]> = {};
  if (!usernameValue.trim()) {
    errors.username = ["请输入用户名"];
  }
  if (!passwordValue) {
    errors.password = ["请输入密码"];
  }
  if (!passwordConfirmationValue) {
    errors.password_confirmation = ["请再次输入密码"];
  } else if (passwordValue !== passwordConfirmationValue) {
    errors.password_confirmation = ["两次输入的密码不一致"];
  }
  return errors;
}

function applyRegistrationError(error: unknown, registrationCompleted: boolean): void {
  if (registrationCompleted && error instanceof AuthPersistenceError) {
    errorMessage.value = "用户已创建，但无法保存登录状态，请检查浏览器存储权限后返回登录";
    notice.error(errorMessage.value);
    return;
  }
  if (registrationCompleted) {
    errorMessage.value = "用户已创建，但自动登录失败，请返回登录页手动登录";
    notice.error(errorMessage.value);
    return;
  }

  if (error instanceof ApiError) {
    fieldErrors.value = error.fieldErrors;
    if (error.code === "invalid_access_token" || error.code === "permission_denied") {
      errorMessage.value = "当前服务已经存在用户，请返回登录页";
    } else if (error.code === "username_taken") {
      errorMessage.value = "该用户名已存在，请更换用户名或返回登录";
    } else {
      errorMessage.value =
        Object.keys(error.fieldErrors).length > 0 ? "请检查输入内容" : error.message;
    }
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

  errorMessage.value = "创建用户失败，请稍后重试";
  notice.error(errorMessage.value);
}
</script>
