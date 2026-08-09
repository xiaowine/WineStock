<!--
  本文件拥有当前用户修改密码页面，属于 frontend 鉴权页面层。
  它处理强制改密和主动改密，不拥有管理员重置密码或平台凭据存储。
-->
<template>
  <main v-overlay-scrollbar class="auth-page">
    <section class="auth-panel" aria-labelledby="change-password-title">
      <header class="auth-header">
        <div class="brand-lockup">
          <BrandMark />
          <span class="brand-name">WineStock</span>
        </div>
        <div>
          <h1 id="change-password-title">{{ $title($route.meta.title) }}</h1>
          <p>
            {{
              passwordChangeRequired
                ? $t('auth.tempPasswordNotice')
                : $t('auth.changePasswordInstructions')
            }}
          </p>
        </div>
      </header>

      <p class="auth-account-context">
        {{ $t('auth.currentAccount') }}<strong>{{ username }}</strong>
      </p>

      <form class="auth-form" novalidate @submit.prevent="submitPasswordChange">
        <FormField
          :label="$t('auth.currentPassword')"
          control-id="change-current-password"
          validation-key="current_password"
          :error="currentPasswordError"
          v-slot="{ describedBy, invalid }"
        >
          <PasswordInput
            id="change-current-password"
            v-model="currentPassword"
            name="current_password"
            autocomplete="off"
            maxlength="256"
            autofocus
            :disabled="isSubmitting"
            :aria-invalid="invalid || undefined"
            :aria-describedby="describedBy"
          />
        </FormField>

        <FormField
          :label="$t('auth.newPassword')"
          control-id="change-new-password"
          validation-key="new_password"
          :error="newPasswordError"
          :hint="$t('auth.passwordMinLengthHint')"
          v-slot="{ describedBy, invalid }"
        >
          <PasswordInput
            id="change-new-password"
            v-model="newPassword"
            name="new_password"
            autocomplete="off"
            minlength="8"
            maxlength="128"
            :disabled="isSubmitting"
            :aria-invalid="invalid || undefined"
            :aria-describedby="describedBy"
          />
        </FormField>

        <FormField
          :label="$t('auth.confirmNewPassword')"
          control-id="change-new-password-confirmation"
          validation-key="new_password_confirmation"
          :error="newPasswordConfirmationError"
          v-slot="{ describedBy, invalid }"
        >
          <PasswordInput
            id="change-new-password-confirmation"
            v-model="newPasswordConfirmation"
            name="new_password_confirmation"
            autocomplete="off"
            maxlength="128"
            :disabled="isSubmitting"
            :aria-invalid="invalid || undefined"
            :aria-describedby="describedBy"
          />
        </FormField>

        <button class="primary-button primary-button--full" type="submit" :disabled="isSubmitting">
          {{ isSubmitting ? $t('auth.changingPassword') : $t('auth.changePassword') }}
        </button>
      </form>

      <div class="auth-page-actions">
        <RouterLink
          v-if="!passwordChangeRequired"
          class="secondary-button"
          :to="{ name: 'dashboard' }"
        >
          {{ $t('auth.backToOverview') }}
        </RouterLink>
        <button
          class="secondary-button"
          type="button"
          :disabled="isSubmitting || isLoggingOut"
          @click="handleLogout"
        >
          {{ isLoggingOut ? $t('auth.loggingOut') : $t('auth.logout') }}
        </button>
      </div>
    </section>
  </main>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
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
const { t } = useI18n();
const currentPassword = ref("");
const newPassword = ref("");
const newPasswordConfirmation = ref("");
const isSubmitting = ref(false);
const errorMessage = ref("");
const fieldErrors = ref<Readonly<Record<string, readonly string[]>>>({});
useFormValidation(fieldErrors);

const username = computed(() => authSession.value?.user.username ?? "");
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
    notice.warning(t("auth.checkPasswordInfo"), {
      detail: Object.values(fieldErrors.value)[0]?.[0] ?? t("auth.checkPasswordInfo"),
    });
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
    notice.success(t("auth.passwordChanged"));
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
        ? t("auth.logoutPersistenceFailed")
        : t("auth.logoutFailed");
    return;
  }

  await router.replace({
    name: "auth-entry",
    query: result === "local_only" ? { logout: "local_only" } : undefined,
  });
  if (result === "local_only") {
    notice.warning(t("auth.logoutLocalOnlyWarning"));
  } else {
    notice.success(t("auth.loggedOut"));
  }
}

function validatePasswordChange(
  currentPasswordValue: string,
  newPasswordValue: string,
  confirmationValue: string,
): Readonly<Record<string, readonly string[]>> {
  const errors: Record<string, string[]> = {};
  if (!currentPasswordValue.trim()) {
    errors.current_password = [t("auth.currentPasswordRequired")];
  }
  if (!newPasswordValue.trim()) {
    errors.new_password = [t("auth.newPasswordRequired")];
  } else if (newPasswordValue.length < 8) {
    errors.new_password = [t("auth.newPasswordMinLength")];
  } else if (newPasswordValue === currentPasswordValue) {
    errors.new_password = [t("auth.newPasswordSameAsCurrent")];
  }
  if (!confirmationValue) {
    errors.new_password_confirmation = [t("auth.newPasswordConfirmationRequired")];
  } else if (confirmationValue !== newPasswordValue) {
    errors.new_password_confirmation = [t("auth.newPasswordMismatch")];
  }
  return errors;
}

function applyPasswordChangeError(error: unknown): void {
  if (error instanceof ApiError) {
    fieldErrors.value = error.fieldErrors;
    const hasFieldErrors = Object.keys(error.fieldErrors).length > 0;
    errorMessage.value =
      error.code === "invalid_credentials"
        ? t("error.invalid_credentials")
        : hasFieldErrors
          ? t("auth.checkInput")
          : error.message;
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

  errorMessage.value = t("auth.passwordChangeFailed");
  notice.error(errorMessage.value);
}
</script>
