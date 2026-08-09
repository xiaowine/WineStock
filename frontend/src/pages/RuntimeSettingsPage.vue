<!--
  本文件拥有正式运行设置 Dialog：复用既有 ModalDialog、Shell 契约、runtime-settings
  纯模块与嵌套 Dialog；含本机静默会话切 server-mode 的强制设密门。
  `embedded` 为 true 时由 AppShell 原地打开；启动漏斗和恢复入口仍通过路由挂载同一组件。
  它不读写平台文件、不直接管理 core 生命周期，也不改变业务 API 契约。
-->
<template>
  <ModalDialog
    :open="true"
    :title="$t('runtime.runtimeMode')"
    :description="
      setupFinished
        ? $t('runtime.settingsDescriptionFinished')
        : $t('runtime.settingsDescriptionFirstTime')
    "
    :busy="applying"
    wide
    @close="leaveRuntimeSettings"
  >
    <section class="runtime-next" aria-labelledby="runtime-next-config-title">
      <section
        class="runtime-next-status"
        :class="`runtime-next-status--${statusTone}`"
        :aria-label="$t('runtime.serviceStatusAriaLabel')"
      >
        <span class="runtime-next-status__dot" aria-hidden="true"></span>
        <div class="runtime-next-status__copy">
          <strong>{{ statusTitle }}</strong>
          <span v-if="displayAddress">{{ displayAddress }}</span>
        </div>
        <button
          v-if="canRetryActiveService"
          class="text-button"
          type="button"
          :disabled="checkingActiveService"
          @click="retryActiveService"
        >
          {{ checkingActiveService ? $t("runtime.checking") : $t("runtime.retry") }}
        </button>
      </section>

      <form
        id="runtime-settings-form"
        class="runtime-next__form"
        novalidate
        @submit.prevent="requestApply"
      >
        <div>
          <fieldset class="runtime-next-tabs" :disabled="applying" :aria-label="$t('runtime.runtimeMode')">
            <label
              v-for="option in modeOptions"
              :key="option.value"
              class="runtime-next-tab"
              :class="{
                'runtime-next-tab--selected': option.selected,
                'runtime-next-tab--disabled': option.disabled,
              }"
              :title="option.disabled && option.disabledReason ? option.disabledReason : undefined"
            >
              <input
                type="radio"
                name="runtime_next_mode"
                :value="option.value"
                :checked="option.selected"
                :disabled="option.disabled"
                @change="changeMode(option.value)"
              />
              <span>{{ option.label }}</span>
            </label>
          </fieldset>
          <p v-if="isPureWebPlatform" class="runtime-next__tabs-note">
            {{ $t("runtime.webModeNote") }}
          </p>
        </div>

        <section class="runtime-next__config" aria-labelledby="runtime-next-config-title">
          <div class="runtime-next__config-heading">
            <h2 id="runtime-next-config-title">{{ modeTitle }}</h2>
            <span v-if="!setupFinished">{{ $t("runtime.saveToConfirmMode") }}</span>
            <span v-else-if="dirty">{{ $t("runtime.unsavedChanges") }}</span>
          </div>

          <template v-if="remoteMode">
            <FormInput
              v-model="draft.remoteBaseUrl"
              :label="$t('runtime.serverAddress')"
              validation-key="remoteBaseUrl"
              :error="fieldError('remoteBaseUrl')"
              :hint="$t('runtime.serverAddressHint')"
              name="runtime_next_remote_base_url"
              type="url"
              inputmode="url"
              autocomplete="off"
              placeholder="https://server.example.com:17890"
              :disabled="applying"
              required
            />
            <div v-if="usesInsecureRemoteHttp" class="form-warning" role="status">
              {{ $t("runtime.insecureHttpWarning") }}
            </div>
          </template>

          <template v-else>
            <p v-if="!serverMode" class="runtime-next__note">
              {{ $t("runtime.localAutoStartNote") }}
            </p>
            <button
              v-if="canRepairFirewall"
              class="secondary-button runtime-next__firewall-action"
              type="button"
              :disabled="applying"
              @click="repairFirewall"
            >
              {{ firewallRepairButtonLabel }}
            </button>
            <details v-if="serverMode" class="runtime-next__advanced">
              <summary>{{ $t("runtime.advancedSettings") }}</summary>
              <FormInput
                v-model="draft.port"
                :label="$t('runtime.servicePort')"
                validation-key="port"
                :error="fieldError('port')"
                :hint="$t('runtime.portHint')"
                name="runtime_next_port"
                type="number"
                inputmode="numeric"
                min="1"
                max="65535"
                :disabled="applying"
                required
              />
              <FormInput
                v-model="draft.bindHost"
                :label="$t('runtime.listenAddress')"
                validation-key="bindHost"
                :error="fieldError('bindHost')"
                :hint="bindHostHint"
                name="runtime_next_bind_host"
                type="text"
                autocomplete="off"
                :disabled="applying"
                required
              />
            </details>
            <div v-if="firewallStatusMessage" class="form-warning" role="status">
              {{ firewallStatusMessage }}
            </div>
          </template>

        </section>
      </form>
    </section>

    <template #actions>
      <button
        class="secondary-button runtime-next__cancel"
        type="button"
        @click="leaveRuntimeSettings"
      >
        {{ $t("runtime.cancel") }}
      </button>
      <button
        class="primary-button"
        type="submit"
        form="runtime-settings-form"
        :disabled="applying || testingRemote || !canSave"
      >
        {{
          testingRemote
            ? $t("runtime.testingConnection")
            : applying
              ? $t("runtime.saving")
              : remoteMode
                ? $t("runtime.testAndSave")
                : $t("runtime.saveSettings")
        }}
      </button>
    </template>

    <ModalDialog
      :open="confirmationOpen"
      :title="enablingLanAccess ? $t('runtime.enableLanTitle') : $t('runtime.switchModeTitle')"
      :description="confirmationDescription"
      :busy="applying"
      compact
      nested
      @close="confirmationOpen = false"
    >
      <p class="runtime-next__confirmation">{{ confirmationDetail }}</p>
      <template #actions>
        <button
          class="secondary-button"
          type="button"
          :disabled="applying"
          @click="confirmationOpen = false"
        >
          {{ $t("runtime.cancel") }}
        </button>
        <button class="primary-button" type="button" :disabled="applying" @click="applyConfirmed">
          {{ applying ? $t("runtime.saving") : $t("runtime.confirm") }}
        </button>
      </template>
    </ModalDialog>

    <ModalDialog
      :open="firewallRecoveryOpen"
      :title="firewallRecoveryTitle"
      :description="firewallRecoveryDescription"
      :busy="firewallRepairing"
      compact
      nested
      @close="firewallRecoveryOpen = false"
    >
      <p class="runtime-next__confirmation">{{ firewallRecoveryDetail }}</p>
      <template #actions>
        <button
          class="secondary-button"
          type="button"
          :disabled="firewallRepairing"
          @click="firewallRecoveryOpen = false"
        >
          {{ $t("runtime.continueAnyway") }}
        </button>
        <button
          class="primary-button"
          type="button"
          :disabled="firewallRepairing"
          @click="repairFirewall"
        >
          {{ firewallRepairing ? $t("runtime.repairing") : firewallRepairActionLabel }}
        </button>
      </template>
    </ModalDialog>

    <ModalDialog
      :open="passwordGateOpen"
      :title="$t('runtime.passwordGateTitle')"
      :description="$t('runtime.passwordGateDescription')"
      :busy="gateSubmitting"
      compact
      nested
      @close="closePasswordGate"
    >
      <FormField
        :label="$t('runtime.currentUserPassword')"
        control-id="runtime_next_gate_password"
        validation-key="gatePassword"
        :error="gateFieldError"
        :hint="$t('runtime.passwordMinHint')"
        required
        v-slot="{ describedBy, invalid }"
      >
        <PasswordInput
          id="runtime_next_gate_password"
          v-model="gatePassword"
          name="runtime_next_gate_password"
          minlength="8"
          maxlength="128"
          :disabled="gateSubmitting"
          :aria-invalid="invalid || undefined"
          :aria-describedby="describedBy"
        />
      </FormField>
      <FormField
        :label="$t('runtime.confirmPassword')"
        control-id="runtime_next_gate_password_confirm"
        validation-key="gatePasswordConfirm"
        :error="gateConfirmError"
        required
        v-slot="{ describedBy, invalid }"
      >
        <PasswordInput
          id="runtime_next_gate_password_confirm"
          v-model="gatePasswordConfirm"
          name="runtime_next_gate_password_confirm"
          maxlength="128"
          :disabled="gateSubmitting"
          :aria-invalid="invalid || undefined"
          :aria-describedby="describedBy"
        />
      </FormField>
      <template #actions>
        <button
          class="secondary-button"
          type="button"
          :disabled="gateSubmitting"
          @click="closePasswordGate"
        >
          {{ $t("runtime.cancel") }}
        </button>
        <button
          class="primary-button"
          type="button"
          :disabled="gateSubmitting"
          @click="submitPasswordGate"
        >
          {{ gateSubmitting ? $t("runtime.settingUp") : $t("runtime.setAndContinue") }}
        </button>
      </template>
    </ModalDialog>
  </ModalDialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { changeOwnPassword, getLocalSessionStatus } from "../api/auth";
import { authSession, authStatus, localSilentAuthActive } from "../auth/session";
import FormField from "../components/forms/FormField.vue";
import FormInput from "../components/forms/FormInput.vue";
import ModalDialog from "../components/ModalDialog.vue";
import PasswordInput from "../components/PasswordInput.vue";
import { useFormValidation } from "../composables/useFormValidation";
import { notice } from "../notices/notice";
import { getDefaultAppRouteName } from "../router/navigation";
import {
  applyRuntimeConfig,
  initializeShellRuntime,
  isRuntimeSetupFinished,
  repairFirewall as repairFirewallShell,
  runtimeSnapshot,
  shellRuntimeError,
  validateRuntimeConfig,
} from "../shell/runtime";
import {
  cloneRuntimeConfig,
  defaultRuntimeConfig,
  type EditableRuntimeConfig,
  type RuntimeConfigField,
  type RuntimeMode,
} from "../shell/contract";
import { localizeRuntimeFieldErrors } from "../shell/fieldErrors";
import {
  checkServiceAvailability,
  isCheckingServiceAvailability,
  serviceAvailabilityStatus,
} from "../service/availability";
import { isSafeInternalPath, resolveRuntimeSettingsLeave } from "./runtime-settings/leave";
import {
  applyRuntimeModeDefaults,
  isRemoteRuntimeMode,
  previewApiBaseUrl,
  sameRuntimeConfig,
} from "./runtime-settings/model";

type StatusTone = "neutral" | "success" | "warning" | "danger";
const props = withDefaults(defineProps<{ embedded?: boolean }>(), {
  embedded: false,
});
const emit = defineEmits<{ close: [] }>();

const router = useRouter();
const route = useRoute();
const { t } = useI18n();
const draft = ref<EditableRuntimeConfig>(cloneRuntimeConfig(defaultRuntimeConfig));
const fieldErrors = ref<Partial<Record<RuntimeConfigField, readonly string[]>>>({});
const applying = ref(false);
const testingRemote = ref(false);
const confirmationOpen = ref(false);
const firewallRecoveryOpen = ref(false);
const firewallRepairing = ref(false);
const passwordGateOpen = ref(false);
const gatePassword = ref("");
const gatePasswordConfirm = ref("");
const gateSubmitting = ref(false);
const gateFieldError = ref("");
const gateConfirmError = ref("");
useFormValidation(fieldErrors);

const snapshot = computed(() => runtimeSnapshot.value);
/** 纯网页端只有连接远端一种能力：本机自用/共享服务禁用，草稿被动纠正为远端。 */
const isPureWebPlatform = computed(() => snapshot.value?.platform === "web");
const remoteMode = computed(() => isRemoteRuntimeMode(draft.value.mode));
const serverMode = computed(() => draft.value.mode === "server-mode");
const activeAddress = computed(() => snapshot.value?.service.apiBaseUrl ?? "");
/** 本机服务的回环地址是实现细节，对用户无意义：状态卡只在远端模式展示服务器地址。 */
const displayAddress = computed(() =>
  snapshot.value?.service.ownership === "remote" ? activeAddress.value : "",
);
const previewAddress = computed(() => previewApiBaseUrl(draft.value));
const firewallStatus = computed(() => snapshot.value?.service.firewall?.status);
const firewallProviderName = computed(() =>
  snapshot.value?.platform === "desktop" && snapshot.value.capabilities.serverMode
    ? t("runtime.firewallProviderWindows")
    : t("runtime.firewallProviderSystem"),
);
const firewallStatusMessage = computed(() => {
  if (snapshot.value?.config.mode !== "server-mode" || !serverMode.value) return "";

  switch (firewallStatus.value) {
    case "ready":
      return t("runtime.firewallReady");
    case "requires-elevation":
      return t("runtime.firewallRequiresElevation", {
        provider: firewallProviderName.value,
      });
    case "blocked-by-policy":
      return t("runtime.firewallBlockedByPolicy", { provider: firewallProviderName.value });
    case "profile-unsupported":
      return t("runtime.firewallProfileUnsupported");
    case "disabled":
      return t("runtime.firewallDisabled", { provider: firewallProviderName.value });
    case "cleanup-pending":
      return t("runtime.firewallCleanupPending");
    case "error":
      return t("runtime.firewallError", { provider: firewallProviderName.value });
    case "not-required":
      return t("runtime.firewallNotRequired");
    default:
      return "";
  }
});
const canRepairFirewall = computed(() =>
  ["requires-elevation", "error", "cleanup-pending"].includes(firewallStatus.value ?? ""),
);
const firewallRecoveryRequired = computed(() => canRepairFirewall.value);
const firewallRecoveryTitle = computed(() =>
  firewallStatus.value === "cleanup-pending"
    ? t("runtime.firewallCleanupTitle")
    : t("runtime.firewallConfigIncompleteTitle", { provider: firewallProviderName.value }),
);
const firewallRecoveryDescription = computed(() =>
  firewallStatus.value === "cleanup-pending"
    ? t("runtime.firewallCleanupDescription")
    : t("runtime.firewallConfigIncompleteDescription"),
);
const firewallRecoveryDetail = computed(() =>
  firewallStatus.value === "cleanup-pending"
    ? t("runtime.firewallCleanupDetail", { provider: firewallProviderName.value })
    : t("runtime.firewallConfigIncompleteDetail", { provider: firewallProviderName.value }),
);
const firewallRepairActionLabel = computed(() =>
  firewallStatus.value === "cleanup-pending"
    ? t("runtime.firewallRetryCleanup")
    : t("runtime.firewallRetryAuthorize"),
);
const firewallRepairButtonLabel = computed(() =>
  firewallStatus.value === "cleanup-pending"
    ? t("runtime.firewallRetryCleanup")
    : t("runtime.firewallRetryButton"),
);
const dirty = computed(
  () =>
    !snapshot.value ||
    snapshot.value.configStatus !== "configured" ||
    !sameRuntimeConfig(draft.value, snapshot.value.config),
);
const endpointChanging = computed(() =>
  Boolean(activeAddress.value && previewAddress.value !== activeAddress.value),
);
const modeChanging = computed(
  () =>
    snapshot.value?.configStatus === "configured" &&
    snapshot.value.config.mode !== draft.value.mode,
);
const serverPortChanging = computed(
  () =>
    snapshot.value?.configStatus === "configured" &&
    snapshot.value.config.mode === "server-mode" &&
    draft.value.mode === "server-mode" &&
    snapshot.value.config.port !== draft.value.port,
);
const runtimeChangeClearsSession = computed(
  () => (endpointChanging.value || modeChanging.value) && !serverPortChanging.value,
);
const enablingLanAccess = computed(
  () =>
    serverMode.value &&
    (snapshot.value?.config.mode !== "server-mode" ||
      snapshot.value.config.bindHost !== draft.value.bindHost),
);
const checkingActiveService = computed(() => isCheckingServiceAvailability.value);
const canRetryActiveService = computed(() => Boolean(activeAddress.value));
/** 用户已通过「保存设置」确认（Shell 已发布 initialized=true）。 */
const setupFinished = computed(() => isRuntimeSetupFinished(snapshot.value));
/** 未初始化时即使表单与草稿一致也允许保存，把确认权收在保存路径上。 */
const canSave = computed(() => dirty.value || !setupFinished.value);
const modeTitle = computed(() =>
  remoteMode.value
    ? t("runtime.remoteModeLabel")
    : serverMode.value
      ? t("runtime.serverModeLabel")
      : t("runtime.localModeLabel"),
);
const bindHostHint = computed(() =>
  serverMode.value ? t("runtime.listenAddressHintDefault") : t("runtime.listenAddressHintLocal"),
);
const serverModeDisabledReason = computed(() => {
  if (isPureWebPlatform.value) {
    return t("runtime.serverModeDisabledWeb");
  }
  if (snapshot.value?.capabilities.serverMode) return "";
  if (snapshot.value?.platform === "android") {
    return t("runtime.serverModeDisabledAndroid");
  }
  return t("runtime.firewallManualConfig");
});
const usesInsecureRemoteHttp = computed(() => {
  if (!remoteMode.value) return false;
  try {
    const url = new URL(draft.value.remoteBaseUrl);
    return (
      url.protocol === "http:" &&
      !["localhost", "127.0.0.1", "[::1]"].includes(url.hostname.toLowerCase())
    );
  } catch {
    return false;
  }
});
const modeOptions = computed(() => [
  {
    value: "self-hosted" as const,
    label: t("runtime.localModeLabel"),
    description: t("runtime.localModeDescription"),
    selected: draft.value.mode === "self-hosted",
    disabled: isPureWebPlatform.value,
    disabledReason: isPureWebPlatform.value ? t("runtime.webConnectRemoteHint") : "",
  },
  {
    value: "client-only" as const,
    label: t("runtime.remoteModeLabel"),
    description: t("runtime.remoteModeDescription"),
    selected: remoteMode.value,
    disabled: false,
    disabledReason: "",
  },
  {
    value: "server-mode" as const,
    label: t("runtime.serverModeLabel"),
    description: t("runtime.serverModeDescription"),
    selected: serverMode.value,
    disabled: isPureWebPlatform.value || !(snapshot.value?.capabilities.serverMode ?? false),
    disabledReason: serverModeDisabledReason.value,
  },
]);
const statusTone = computed<StatusTone>(() => {
  if (["blocked-by-policy", "error", "cleanup-pending"].includes(firewallStatus.value ?? ""))
    return "danger";
  if (
    ["requires-elevation", "profile-unsupported", "disabled"].includes(firewallStatus.value ?? "")
  )
    return "warning";
  if (
    snapshot.value?.service.phase === "failed" ||
    serviceAvailabilityStatus.value === "unavailable"
  )
    return "danger";
  if (["starting", "stopping"].includes(snapshot.value?.service.phase ?? "")) return "warning";
  if (activeAddress.value && serviceAvailabilityStatus.value === "available") return "success";
  return "neutral";
});
const statusTitle = computed(() => {
  const phase = snapshot.value?.service.phase;
  if (firewallStatus.value === "requires-elevation") return t("runtime.statusNeedsFirewallAuth");
  if (firewallStatus.value === "blocked-by-policy") return t("runtime.statusFirewallBlocked");
  if (firewallStatus.value === "profile-unsupported") return t("runtime.statusProfileUnsupported");
  if (firewallStatus.value === "disabled") return t("runtime.statusFirewallDisabled");
  if (firewallStatus.value === "cleanup-pending") return t("runtime.statusCleanupPending");
  if (firewallStatus.value === "error") return t("runtime.statusFirewallError");
  if (phase === "starting") return t("runtime.statusStarting");
  if (phase === "stopping") return t("runtime.statusStopping");
  if (phase === "failed") return t("runtime.statusLocalFailed");
  if (!activeAddress.value) return t("runtime.statusNotConnected");
  if (serviceAvailabilityStatus.value === "available") return t("runtime.statusAvailable");
  if (serviceAvailabilityStatus.value === "unavailable") return t("runtime.statusUnavailable");
  return t("runtime.statusChecking");
});
const confirmationDescription = computed(() => {
  if (serverPortChanging.value) return t("runtime.confirmationPortChange");
  if (!enablingLanAccess.value) return t("runtime.confirmationAddressChange");
  return t("runtime.confirmationLanEnable");
});
const confirmationDetail = computed(() =>
  runtimeChangeClearsSession.value
    ? t("runtime.confirmationSessionCleared")
    : t("runtime.confirmationContinue"),
);

watch(
  runtimeSnapshot,
  (next) => {
    if (next && !applying.value)
      draft.value = coerceDraftForPlatform(cloneRuntimeConfig(next.config));
  },
  { immediate: true },
);
watch(shellRuntimeError, (error) => {
  if (error) notice.error(t("runtime.runtimeInitFailed"), { detail: error });
});
void initializeShellRuntime()
  .then((initial) => {
    if (isFirewallRecoveryStatus(initial.service.firewall?.status)) {
      firewallRecoveryOpen.value = true;
    }
  })
  .catch(() => undefined);

function fieldError(field: RuntimeConfigField): string {
  return fieldErrors.value[field]?.[0] ?? "";
}

function changeMode(mode: RuntimeMode): void {
  draft.value = applyRuntimeModeDefaults(draft.value, mode);
  fieldErrors.value = {};
}

/** 纯网页端把本机类草稿纠正为远端；平台 shell 内原样返回。 */
function coerceDraftForPlatform(config: EditableRuntimeConfig): EditableRuntimeConfig {
  if (isPureWebPlatform.value && !isRemoteRuntimeMode(config.mode)) {
    const coerced = applyRuntimeModeDefaults(config, "client-only");
    // 本机类配置没有远端地址：用当前实际生效的服务地址预填，重进页面不丢"原来的地址"。
    if (!coerced.remoteBaseUrl && activeAddress.value) {
      return { ...coerced, remoteBaseUrl: activeAddress.value };
    }
    return coerced;
  }
  return config;
}

async function requestApply(): Promise<void> {
  const validation = await validateRuntimeConfig(draft.value);
  fieldErrors.value = localizeRuntimeFieldErrors(validation.fieldErrors);
  if (!validation.valid) {
    notice.warning(t("runtime.checkRuntimeMode"), {
      detail: Object.values(fieldErrors.value)[0]?.[0] ?? t("runtime.checkInput"),
    });
    return;
  }
  if (remoteMode.value && !(await testRemoteConnection())) return;
  const gate = await resolveLocalUserPasswordGate();
  if (gate === "blocked") return;
  if (gate === "required") {
    openPasswordGate();
    return;
  }
  if (endpointChanging.value || modeChanging.value || enablingLanAccess.value) {
    confirmationOpen.value = true;
    return;
  }
  await executeApply();
}

/**
 * 本机静默免登录切到 server-mode 前的强制设密门：
 * 当前用户密码仍为自动开通的随机占位值时，先设真实密码，否则局域网端无人能登录。
 * 状态查询失败时阻止提交并提示，避免带着占位密码开放局域网。
 */
async function resolveLocalUserPasswordGate(): Promise<"pass" | "required" | "blocked"> {
  if (
    draft.value.mode !== "server-mode" ||
    !localSilentAuthActive.value ||
    authStatus.value !== "authenticated"
  ) {
    return "pass";
  }
  try {
    return (await getLocalSessionStatus()).password_placeholder ? "required" : "pass";
  } catch {
    notice.error(t("runtime.passwordStatusFailed"), { detail: t("runtime.retryLater") });
    return "blocked";
  }
}

function openPasswordGate(): void {
  gatePassword.value = "";
  gatePasswordConfirm.value = "";
  gateFieldError.value = "";
  gateConfirmError.value = "";
  passwordGateOpen.value = true;
}

function closePasswordGate(): void {
  if (gateSubmitting.value) return;
  passwordGateOpen.value = false;
}

/** 占位态免旧密码设置真实密码；成功后回到正常的确认与保存流程。 */
async function submitPasswordGate(): Promise<void> {
  gateFieldError.value = gatePassword.value.length < 8 ? t("runtime.passwordTooShort") : "";
  gateConfirmError.value =
    gatePassword.value === gatePasswordConfirm.value ? "" : t("runtime.passwordMismatch");
  if (gateFieldError.value || gateConfirmError.value) {
    notice.warning(t("runtime.checkUserAccount"), {
      detail: gateFieldError.value || gateConfirmError.value,
    });
    return;
  }

  gateSubmitting.value = true;
  try {
    await changeOwnPassword({
      current_password: "",
      new_password: gatePassword.value,
    });
    passwordGateOpen.value = false;
    notice.success(t("runtime.accountPasswordSet"));
    confirmationOpen.value = true;
  } catch (error) {
    notice.error(t("runtime.setPasswordFailed"), {
      detail: error instanceof Error ? error.message : t("runtime.opFailedRetry"),
    });
  } finally {
    gateSubmitting.value = false;
  }
}

async function applyConfirmed(): Promise<void> {
  confirmationOpen.value = false;
  await executeApply();
}

async function executeApply(): Promise<void> {
  applying.value = true;
  const wasSetupFinished = setupFinished.value;
  try {
    const result = await applyRuntimeConfig(draft.value);
    fieldErrors.value = localizeRuntimeFieldErrors(result.fieldErrors);
    if (!result.applied) {
      notice.error(t("runtime.saveFailed"), {
        detail:
          result.error?.message ??
          Object.values(fieldErrors.value)[0]?.[0] ??
          t("runtime.saveNotApplied"),
      });
      return;
    }
    draft.value = cloneRuntimeConfig(result.snapshot.config);
    if (firewallRecoveryRequired.value) {
      firewallRecoveryOpen.value = true;
      notice.warning(t("runtime.modeSavedFirewallPending"), {
        detail: t("runtime.firewallPendingDetail"),
      });
    } else {
      notice.success(t("runtime.modeSaved"));
    }
    // 设置从「未完成」变为「已确认」且仍匿名时，自动进入认证入口。
    if (
      !wasSetupFinished &&
      isRuntimeSetupFinished(result.snapshot) &&
      authStatus.value !== "authenticated"
    ) {
      await navigateAfterSetup(true);
    }
  } catch (error) {
    notice.error(t("runtime.saveFailed"), {
      detail: error instanceof Error ? error.message : t("runtime.retryLater"),
    });
  } finally {
    applying.value = false;
  }
}

async function testRemoteConnection(): Promise<boolean> {
  testingRemote.value = true;
  const controller = new AbortController();
  const timeout = window.setTimeout(() => controller.abort(), 4_000);
  try {
    const response = await fetch(`${previewAddress.value}/api/health`, {
      headers: { accept: "application/json" },
      credentials: "omit",
      signal: controller.signal,
    });
    const payload = (await response.json()) as unknown;
    if (!response.ok || !isHealthPayload(payload)) throw new Error("invalid health response");
    return true;
  } catch (error) {
    notice.error(t("runtime.remoteTestFailed"), {
      detail:
        error instanceof DOMException && error.name === "AbortError"
          ? t("runtime.remoteTestTimeout")
          : t("runtime.remoteTestUnreachable"),
    });
    return false;
  } finally {
    window.clearTimeout(timeout);
    testingRemote.value = false;
  }
}

async function retryActiveService(): Promise<void> {
  await checkServiceAvailability();
}

async function repairFirewall(): Promise<void> {
  if (!canRepairFirewall.value) return;
  const previousFirewallStatus = firewallStatus.value;
  firewallRepairing.value = true;
  try {
    await repairFirewallShell();
    firewallRecoveryOpen.value = false;
    notice.success(
      previousFirewallStatus === "cleanup-pending"
        ? t("runtime.firewallCleaned")
        : t("runtime.firewallConfigured"),
    );
  } catch (error) {
    notice.error(t("runtime.firewallOpFailed"), {
      detail: error instanceof Error ? error.message : t("runtime.opFailedRetry"),
    });
  } finally {
    firewallRepairing.value = false;
  }
}

function isFirewallRecoveryStatus(status: string | undefined): boolean {
  return ["requires-elevation", "error", "cleanup-pending"].includes(status ?? "");
}

/**
 * 配置阶段结束：已登录回业务，匿名统一进入 /auth。
 * 不在此处 apply：initialized 只能由 Shell 在「保存设置」成功后发布。
 */
async function leaveRuntimeSettings(): Promise<void> {
  if (authStatus.value !== "authenticated" && !setupFinished.value) {
    notice.warning(t("runtime.saveModeFirst"), {
      detail: t("runtime.saveModeFirstDetail"),
    });
    return;
  }
  if (props.embedded) {
    emit("close");
    return;
  }
  await navigateAfterSetup();
}

/** 在设置已确认（或已登录）的前提下执行离开导航。 */
async function navigateAfterSetup(setupFinishedOverride?: boolean): Promise<void> {
  const returnTo = typeof route.query.returnTo === "string" ? route.query.returnTo : undefined;
  const finished =
    setupFinishedOverride === true || setupFinished.value || authStatus.value === "authenticated";
  const target = resolveRuntimeSettingsLeave({
    returnTo,
    setupFinished: finished,
    authenticated: authStatus.value === "authenticated",
    returnToRouteValid: isReturnToRouteValid(returnTo),
  });

  if (target.kind === "stay") {
    return;
  }
  if (target.kind === "path") {
    const resolved = router.resolve(target.path);
    await router.replace({ path: resolved.fullPath });
    return;
  }
  if (target.kind === "default-app") {
    await router.replace({
      name: getDefaultAppRouteName(authSession.value?.user.permissions),
    });
    return;
  }
  await router.replace({
    name: "auth-entry",
    query: target.redirect ? { redirect: target.redirect } : undefined,
  });
}

function isReturnToRouteValid(returnTo: string | undefined): boolean {
  if (!returnTo || !isSafeInternalPath(returnTo)) {
    return false;
  }
  try {
    const resolved = router.resolve(returnTo);
    return (
      resolved.matched.length > 0 &&
      !["runtime-settings", "home-fallback"].includes(String(resolved.name))
    );
  } catch {
    return false;
  }
}

function isHealthPayload(value: unknown): value is { status: "OK" } {
  return typeof value === "object" && value !== null && "status" in value && value.status === "OK";
}
</script>

<style scoped lang="scss" src="./RuntimeSettingsPage.scss"></style>
