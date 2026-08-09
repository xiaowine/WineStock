<!--
  本文件拥有用户名密码登录页面，属于 frontend 鉴权页面层。
  它在桌面和移动视口共用同一登录流程，不实现路由守卫或平台生命周期。
-->
<template>
  <main v-overlay-scrollbar class="auth-page">
    <section class="auth-panel" aria-labelledby="login-title">
      <header class="auth-header">
        <div class="brand-lockup">
          <BrandMark />
          <span class="brand-name">WineStock</span>
        </div>
        <div>
          <h1 id="login-title">{{ $title($route.meta.title) }}</h1>
          <p>{{ $t("auth.loginSubtitle") }}</p>
        </div>
      </header>

      <p v-if="checkingBootstrapStatus" class="auth-runtime-note" role="status">
        {{ $t("auth.checkingService") }}
      </p>

      <form v-else class="auth-form" novalidate @submit.prevent="submitLogin">
        <div v-if="logoutWarning" class="form-warning" role="status">
          {{ logoutWarning }}
        </div>

        <FormInput
          v-model="username"
          :label="$t('auth.username')"
          validation-key="username"
          :error="usernameError"
          name="username"
          type="text"
          autocomplete="off"
          maxlength="64"
          :disabled="isSubmitting"
        />

        <FormField
          :label="$t('auth.password')"
          control-id="login-password"
          validation-key="password"
          :error="passwordError"
          v-slot="{ describedBy, invalid }"
        >
          <PasswordInput
            id="login-password"
            v-model="password"
            name="password"
            autocomplete="off"
            maxlength="256"
            :disabled="isSubmitting"
            :aria-invalid="invalid || undefined"
            :aria-describedby="describedBy"
          />
        </FormField>

        <button class="primary-button primary-button--full" type="submit" :disabled="isSubmitting">
          {{ isSubmitting ? $t('auth.loggingIn') : $t('auth.login') }}
        </button>
      </form>

      <p v-if="!checkingBootstrapStatus" class="auth-runtime-note">
        {{ $t('auth.currentService') }}<code>{{ activeApiBaseUrl ?? $t('auth.notConfigured') }}</code>
        ·
        <RouterLink :to="{ name: 'runtime-settings', query: { returnTo: route.fullPath } }">
          {{ $t('auth.change') }}
        </RouterLink>
      </p>
    </section>
  </main>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { getAuthBootstrapStatus, login } from "../api/auth";
import { ApiConfigurationError, ApiError, ApiNetworkError, ApiResponseError } from "../api/errors";
import { resolveApiClientMetadata } from "../api/runtime-config";
import { establishAuthSession } from "../auth/session";
import { AuthPersistenceError } from "../auth/storage";
import BrandMark from "../components/BrandMark.vue";
import { notice } from "../notices/notice";
import { resolvePostLoginLocation } from "../router/guards";
import PasswordInput from "../components/PasswordInput.vue";
import FormField from "../components/forms/FormField.vue";
import FormInput from "../components/forms/FormInput.vue";
import { useFormValidation } from "../composables/useFormValidation";
import { activeApiBaseUrl } from "../shell/runtime";

const router = useRouter();
const route = useRoute();
const { t } = useI18n();
const username = ref("");
const password = ref("");
const isSubmitting = ref(false);
const checkingBootstrapStatus = ref(true);
const errorMessage = ref("");
const fieldErrors = ref<Readonly<Record<string, readonly string[]>>>({});
useFormValidation(fieldErrors);

const usernameError = computed(() => fieldErrors.value.username?.[0]);
const passwordError = computed(() => fieldErrors.value.password?.[0]);
const logoutWarning = computed(() =>
  route.query.logout === "local_only" ? t("auth.logoutLocalOnlyWarning") : "",
);

onMounted(async () => {
  try {
    const status = await getAuthBootstrapStatus();
    if (status.requires_initial_user && route.name === "login") {
      await router.replace({ name: "register", query: route.query });
      return;
    }
  } catch {
    await router.replace({ name: "auth-entry", query: route.query });
  } finally {
    checkingBootstrapStatus.value = false;
  }
});

/** 校验并提交登录表单；失败时保留输入并映射统一 API 错误。 */
async function submitLogin(): Promise<void> {
  errorMessage.value = "";
  fieldErrors.value = validateLoginInput(username.value, password.value);
  if (Object.keys(fieldErrors.value).length > 0) {
    notice.warning(t("auth.checkLoginInfo"), {
      detail: Object.values(fieldErrors.value)[0]?.[0] ?? t("auth.checkUsernameAndPassword"),
    });
    return;
  }

  isSubmitting.value = true;
  try {
    const metadata = resolveApiClientMetadata();
    const response = await login({
      username: username.value.trim(),
      password: password.value,
      device_name: metadata.deviceName,
      client_kind: metadata.clientKind,
      version: metadata.appVersion,
    });

    establishAuthSession(response);
    await router.replace(resolvePostLoginLocation(router, route.query.redirect));
    notice.success(t("auth.loginSuccess"));
  } catch (error) {
    applyLoginError(error);
  } finally {
    isSubmitting.value = false;
  }
}

function validateLoginInput(
  usernameValue: string,
  passwordValue: string,
): Readonly<Record<string, readonly string[]>> {
  const errors: Record<string, string[]> = {};
  if (!usernameValue.trim()) {
    errors.username = [t("auth.usernameRequired")];
  }
  if (!passwordValue) {
    errors.password = [t("auth.passwordRequired")];
  }
  return errors;
}

function applyLoginError(error: unknown): void {
  if (error instanceof AuthPersistenceError) {
    errorMessage.value = t("auth.loginPersistenceFailed");
    notice.error(errorMessage.value);
    return;
  }
  if (error instanceof ApiError) {
    if (error.code === "invalid_credentials") {
      fieldErrors.value = {};
      errorMessage.value = "";
      notice.error(t("error.invalid_credentials"));
      return;
    }

    fieldErrors.value = error.fieldErrors;
    const hasFieldErrors = Object.keys(error.fieldErrors).length > 0;
    errorMessage.value = hasFieldErrors ? t("auth.checkInput") : error.message;
    notice.error(errorMessage.value, {
      detail: hasFieldErrors ? Object.values(error.fieldErrors)[0]?.[0] : undefined,
    });
    return;
  }
  if (error instanceof ApiConfigurationError) {
    errorMessage.value = error.message;
    notice.error(errorMessage.value);
    return;
  }
  if (error instanceof ApiNetworkError) {
    errorMessage.value = t("auth.networkConnectFailed");
    notice.error(errorMessage.value);
    return;
  }
  if (error instanceof ApiResponseError) {
    errorMessage.value = t("auth.responseInvalidFormat");
    notice.error(errorMessage.value);
    return;
  }

  errorMessage.value = t("auth.loginFailed");
  notice.error(errorMessage.value);
}
</script>
