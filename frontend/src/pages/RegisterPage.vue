<!--
  本文件拥有首个用户注册页面，属于 frontend 鉴权页面层。
  它在桌面和移动视口共用同一注册流程，不负责后续用户管理或平台安全存储。
-->
<template>
  <main v-overlay-scrollbar class="auth-page">
    <section class="auth-panel" aria-labelledby="register-title">
      <header class="auth-header">
        <div class="brand-lockup">
          <BrandMark />
          <span class="brand-name">WineStock</span>
        </div>
        <div>
          <h1 id="register-title">
            {{ localBootstrapMode ? $t('auth.registerLocalAccount') : $title($route.meta.title) }}
          </h1>
          <p>
            {{
              localBootstrapMode
                ? $t('auth.registerLocalUsernameHint')
                : $t('auth.registerFirstUserHint')
            }}
          </p>
        </div>
      </header>

      <p v-if="checkingBootstrapStatus" class="auth-runtime-note" role="status">
        {{ $t('auth.checkingService') }}
      </p>

      <form v-else class="auth-form" novalidate @submit.prevent="submitRegistration">
        <FormInput
          v-model="username"
          :label="$t('auth.username')"
          validation-key="username"
          :error="usernameError"
          name="username"
          type="text"
          autocomplete="off"
          maxlength="64"
          autofocus
          :disabled="isSubmitting"
        />

        <FormField
          v-if="!localBootstrapMode"
          :label="$t('auth.password')"
          control-id="register-password"
          validation-key="password"
          :error="passwordError"
          v-slot="{ describedBy, invalid }"
        >
          <PasswordInput
            id="register-password"
            v-model="password"
            name="password"
            autocomplete="off"
            maxlength="256"
            :disabled="isSubmitting"
            :aria-invalid="invalid || undefined"
            :aria-describedby="describedBy"
          />
        </FormField>

        <FormField
          v-if="!localBootstrapMode"
          :label="$t('auth.confirmPassword')"
          control-id="register-password-confirmation"
          validation-key="password_confirmation"
          :error="passwordConfirmationError"
          v-slot="{ describedBy, invalid }"
        >
          <PasswordInput
            id="register-password-confirmation"
            v-model="passwordConfirmation"
            name="password_confirmation"
            autocomplete="off"
            maxlength="256"
            :disabled="isSubmitting"
            :aria-invalid="invalid || undefined"
            :aria-describedby="describedBy"
          />
        </FormField>

        <button class="primary-button primary-button--full" type="submit" :disabled="isSubmitting">
          {{ isSubmitting ? $t('auth.creatingAccount') : $t('auth.createAndEnter') }}
        </button>
      </form>

      <p v-if="!checkingBootstrapStatus" class="auth-runtime-note">
        {{
          localBootstrapMode
            ? $t('auth.registerLocalSuccess')
            : $t('auth.registerAutoLoginHint')
        }}
      </p>
    </section>
  </main>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import {
  getAuthBootstrapStatus,
  login,
  markAuthBootstrapInitialized,
  registerInitialUser,
} from "../api/auth";
import { ApiConfigurationError, ApiError, ApiNetworkError, ApiResponseError } from "../api/errors";
import { resolveApiClientMetadata } from "../api/runtime-config";
import {
  establishAuthSession,
  establishLocalInitialUser,
  localSilentAuthActive,
} from "../auth/session";
import { AuthPersistenceError } from "../auth/storage";
import BrandMark from "../components/BrandMark.vue";
import { translateMessageOrNull } from "../i18n";
import { notice } from "../notices/notice";
import PasswordInput from "../components/PasswordInput.vue";
import FormField from "../components/forms/FormField.vue";
import FormInput from "../components/forms/FormInput.vue";
import { useFormValidation } from "../composables/useFormValidation";
import { runtimeSnapshot } from "../shell/runtime";

const router = useRouter();
const route = useRoute();
const { t } = useI18n();
const username = ref("");
const password = ref("");
const passwordConfirmation = ref("");
const isSubmitting = ref(false);
const checkingBootstrapStatus = ref(true);
const errorMessage = ref("");
const fieldErrors = ref<Readonly<Record<string, readonly string[]>>>({});
useFormValidation(fieldErrors);

const localBootstrapMode = computed(
  () =>
    runtimeSnapshot.value?.platform !== "web" &&
    runtimeSnapshot.value?.config.mode === "self-hosted" &&
    runtimeSnapshot.value?.service.ownership === "local" &&
    localSilentAuthActive.value,
);

const usernameError = computed(() => fieldErrors.value.username?.[0]);
const passwordError = computed(() => fieldErrors.value.password?.[0]);
const passwordConfirmationError = computed(() => fieldErrors.value.password_confirmation?.[0]);

onMounted(async () => {
  try {
    const status = await getAuthBootstrapStatus();
    if (!status.requires_initial_user) {
      await router.replace({ name: "login", query: route.query });
    }
  } catch {
    await router.replace({ name: "auth-entry", query: route.query });
  } finally {
    checkingBootstrapStatus.value = false;
  }
});

/** 创建首个用户后立即登录；注册成功但登录失败时提示改用登录页，避免重复注册。 */
async function submitRegistration(): Promise<void> {
  errorMessage.value = "";
  fieldErrors.value = validateRegistrationInput(
    username.value,
    localBootstrapMode.value ? "" : password.value,
    localBootstrapMode.value ? "" : passwordConfirmation.value,
    localBootstrapMode.value,
  );
  if (Object.keys(fieldErrors.value).length > 0) {
    notice.warning(t("auth.checkRegistrationInfo"), {
      detail: Object.values(fieldErrors.value)[0]?.[0] ?? t("auth.checkRegistrationInfo"),
    });
    return;
  }

  isSubmitting.value = true;
  let registrationCompleted = false;
  try {
    const normalizedUsername = username.value.trim();
    if (localBootstrapMode.value) {
      await establishLocalInitialUser(normalizedUsername);
      markAuthBootstrapInitialized();
      await router.replace({ name: "dashboard" });
      notice.success(t("auth.localAccountCreated"));
      return;
    }
    await registerInitialUser({
      username: normalizedUsername,
      password: password.value,
    });
    markAuthBootstrapInitialized();
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
    notice.success(t("auth.firstUserCreated"));
  } catch (error) {
    await applyRegistrationError(error, registrationCompleted);
  } finally {
    isSubmitting.value = false;
  }
}

function validateRegistrationInput(
  usernameValue: string,
  passwordValue: string,
  passwordConfirmationValue: string,
  localMode: boolean,
): Readonly<Record<string, readonly string[]>> {
  const errors: Record<string, string[]> = {};
  if (!usernameValue.trim()) {
    errors.username = [t("auth.usernameRequired")];
  }
  if (!localMode && !passwordValue) {
    errors.password = [t("auth.passwordRequired")];
  }
  if (!localMode && !passwordConfirmationValue) {
    errors.password_confirmation = [t("auth.passwordConfirmationRequired")];
  } else if (passwordValue !== passwordConfirmationValue) {
    errors.password_confirmation = [t("auth.passwordMismatch")];
  }
  return errors;
}

async function applyRegistrationError(
  error: unknown,
  registrationCompleted: boolean,
): Promise<void> {
  if (registrationCompleted && error instanceof AuthPersistenceError) {
    errorMessage.value = t("auth.registrationPersistenceFailed");
    notice.error(errorMessage.value);
    return;
  }
  if (registrationCompleted) {
    errorMessage.value = t("auth.registrationAutoLoginFailed");
    notice.error(errorMessage.value);
    return;
  }

  if (error instanceof ApiError) {
    fieldErrors.value = error.fieldErrors;
    if (
      error.code === "initial_user_already_exists" ||
      error.code === "invalid_access_token" ||
      error.code === "permission_denied"
    ) {
      errorMessage.value = translateMessageOrNull(`error.${error.code}`) ?? error.message;
      notice.error(errorMessage.value);
      await router.replace({ name: "login", query: route.query });
      return;
    }
    const hasFieldErrors = Object.keys(error.fieldErrors).length > 0;
    errorMessage.value = hasFieldErrors ? t("auth.checkInput") : error.message;
    notice.error(errorMessage.value, {
      detail: Object.values(error.fieldErrors)[0]?.[0],
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

  errorMessage.value = t("auth.registrationFailed");
  notice.error(errorMessage.value);
}
</script>
