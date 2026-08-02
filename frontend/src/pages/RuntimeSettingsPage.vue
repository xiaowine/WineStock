<!--
  本文件拥有正式运行设置 Dialog：复用既有 ModalDialog、Shell 契约、runtime-settings
  纯模块与嵌套 Dialog；含本机静默会话切 server-mode 的强制设密门。
  `embedded` 为 true 时由 AppShell 原地打开；启动漏斗和恢复入口仍通过路由挂载同一组件。
  它不读写平台文件、不直接管理 core 生命周期，也不改变业务 API 契约。
-->
<template>
  <ModalDialog
    :open="true"
    title="运行设置"
    :description="
      setupFinished
        ? '调整这台设备连接 WineStock 的方式。'
        : '先确认这台设备的使用方式，保存后继续。'
    "
    :busy="applying"
    wide
    @close="leaveRuntimeSettings"
  >
    <section class="runtime-next" aria-labelledby="runtime-next-config-title">
      <section
        class="runtime-next-status"
        :class="`runtime-next-status--${statusTone}`"
        aria-label="当前服务状态"
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
          {{ checkingActiveService ? "检查中…" : "重试" }}
        </button>
      </section>

      <form
        id="runtime-settings-form"
        class="runtime-next__form"
        novalidate
        @submit.prevent="requestApply"
      >
        <div>
          <fieldset class="runtime-next-tabs" :disabled="applying" aria-label="运行方式">
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
            浏览器无法在本机启动服务，仅支持连接已有服务器。
          </p>
        </div>

        <section class="runtime-next__config" aria-labelledby="runtime-next-config-title">
          <div class="runtime-next__config-heading">
            <h2 id="runtime-next-config-title">{{ modeTitle }}</h2>
            <span v-if="!setupFinished">请保存以确认运行方式</span>
            <span v-else-if="dirty">有未保存的更改</span>
          </div>

          <template v-if="remoteMode">
            <FormInput
              v-model="draft.remoteBaseUrl"
              label="服务器地址"
              validation-key="remoteBaseUrl"
              :error="fieldError('remoteBaseUrl')"
              hint="例如：https://server.example.com:17890"
              name="runtime_next_remote_base_url"
              type="url"
              inputmode="url"
              autocomplete="off"
              placeholder="https://server.example.com:17890"
              :disabled="applying"
              required
            />
            <div class="runtime-next__test-row">
              <button
                class="secondary-button"
                type="button"
                :disabled="applying || testingRemote"
                @click="testRemoteConnection"
              >
                {{ testingRemote ? "正在测试…" : "测试连接" }}
              </button>
              <span
                v-if="remoteTestMessage"
                class="runtime-next__test-result"
                :class="`runtime-next__test-result--${remoteTestTone}`"
                role="status"
              >
                {{ remoteTestMessage }}
              </span>
            </div>
            <div v-if="usesInsecureRemoteHttp" class="form-warning" role="status">
              这个地址没有使用 HTTPS，请只在可信网络中使用。
            </div>
          </template>

          <template v-else>
            <p v-if="!serverMode" class="runtime-next__note">
              打开应用时，本机服务会自动启动并选择可用端口。
            </p>
            <div v-if="firewallStatusMessage" class="form-warning" role="status">
              {{ firewallStatusMessage }}
            </div>
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
              <summary>高级设置</summary>
              <FormInput
                v-model="draft.port"
                label="服务端口"
                validation-key="port"
                :error="fieldError('port')"
                hint="一般无需修改。"
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
                label="监听地址"
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
          </template>

          <div v-if="lanAccessUnavailable" class="form-warning" role="status">
            当前设备没有可用的局域网地址，请检查网络适配器或监听地址。
          </div>
        </section>
      </form>
    </section>

    <template #actions>
      <button
        class="secondary-button runtime-next__cancel"
        type="button"
        @click="leaveRuntimeSettings"
      >
        取消
      </button>
      <button
        class="primary-button"
        type="submit"
        form="runtime-settings-form"
        :disabled="applying || !canSave"
      >
        {{ applying ? "正在保存…" : remoteMode ? "连接服务器" : "保存设置" }}
      </button>
    </template>

    <ModalDialog
      :open="confirmationOpen"
      :title="enablingLanAccess ? '允许其他设备连接？' : '切换运行服务？'"
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
          取消
        </button>
        <button class="primary-button" type="button" :disabled="applying" @click="applyConfirmed">
          {{ applying ? "正在保存…" : "确认" }}
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
          继续使用
        </button>
        <button
          class="primary-button"
          type="button"
          :disabled="firewallRepairing"
          @click="repairFirewall"
        >
          {{ firewallRepairing ? "正在重试…" : firewallRepairActionLabel }}
        </button>
      </template>
    </ModalDialog>

    <ModalDialog
      :open="passwordGateOpen"
      title="先设置当前用户密码"
      description="开放给其他设备连接前，需要为当前用户设置登录用户名和真实密码；其他设备将用它登录。"
      :busy="gateSubmitting"
      compact
      nested
      @close="closePasswordGate"
    >
      <FormInput
        v-model="gateUsername"
        label="当前用户名"
        validation-key="gateUsername"
        :error="gateUsernameError"
        name="runtime_next_gate_username"
        type="text"
        autocomplete="off"
        maxlength="64"
        :disabled="gateSubmitting"
        required
      />
      <FormInput
        v-model="gatePassword"
        label="当前用户密码"
        validation-key="gatePassword"
        :error="gateFieldError"
        hint="至少 8 个字符。"
        name="runtime_next_gate_password"
        type="password"
        autocomplete="off"
        :disabled="gateSubmitting"
        required
      />
      <FormInput
        v-model="gatePasswordConfirm"
        label="确认密码"
        validation-key="gatePasswordConfirm"
        :error="gateConfirmError"
        name="runtime_next_gate_password_confirm"
        type="password"
        autocomplete="off"
        :disabled="gateSubmitting"
        required
      />
      <template #actions>
        <button
          class="secondary-button"
          type="button"
          :disabled="gateSubmitting"
          @click="closePasswordGate"
        >
          取消
        </button>
        <button
          class="primary-button"
          type="button"
          :disabled="gateSubmitting"
          @click="submitPasswordGate"
        >
          {{ gateSubmitting ? "正在设置…" : "设置并继续" }}
        </button>
      </template>
    </ModalDialog>
  </ModalDialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { changeOwnPassword, getCurrentUser, getLocalSessionStatus } from "../api/auth";
import {
  authSession,
  authStatus,
  localSilentAuthActive,
  replaceCurrentSessionUser,
} from "../auth/session";
import FormInput from "../components/forms/FormInput.vue";
import ModalDialog from "../components/ModalDialog.vue";
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
import { getUsableLanAccessUrls } from "../shell/lanAccess";
import {
  cloneRuntimeConfig,
  defaultRuntimeConfig,
  type EditableRuntimeConfig,
  type RuntimeConfigField,
  type RuntimeMode,
} from "../shell/contract";
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
type TestTone = "success" | "warning" | "danger";

const props = withDefaults(defineProps<{ embedded?: boolean }>(), {
  embedded: false,
});
const emit = defineEmits<{ close: [] }>();

const router = useRouter();
const route = useRoute();
const draft = ref<EditableRuntimeConfig>(cloneRuntimeConfig(defaultRuntimeConfig));
const fieldErrors = ref<Partial<Record<RuntimeConfigField, readonly string[]>>>({});
const applying = ref(false);
const testingRemote = ref(false);
const remoteTestMessage = ref("");
const remoteTestTone = ref<TestTone>("warning");
const confirmationOpen = ref(false);
const firewallRecoveryOpen = ref(false);
const firewallRepairing = ref(false);
const passwordGateOpen = ref(false);
const gateUsername = ref("");
const gatePassword = ref("");
const gatePasswordConfirm = ref("");
const gateSubmitting = ref(false);
const gateUsernameError = ref("");
const gateFieldError = ref("");
const gateConfirmError = ref("");
useFormValidation(fieldErrors);

const snapshot = computed(() => runtimeSnapshot.value);
/** 纯网页端只有连接远程服务一种能力：本机/局域网服务器模式禁用，草稿被动纠正为远端。 */
const isPureWebPlatform = computed(() => snapshot.value?.platform === "web");
const remoteMode = computed(() => isRemoteRuntimeMode(draft.value.mode));
const serverMode = computed(() => draft.value.mode === "server-mode");
const activeAddress = computed(() => snapshot.value?.service.apiBaseUrl ?? "");
/** 本机服务的回环地址是实现细节，对用户无意义：状态卡只在远端模式展示服务器地址。 */
const displayAddress = computed(() =>
  snapshot.value?.service.ownership === "remote" ? activeAddress.value : "",
);
const previewAddress = computed(() => previewApiBaseUrl(draft.value));
const lanAccessUrls = computed(() => getUsableLanAccessUrls(snapshot.value));
const lanAccessUnavailable = computed(
  () =>
    snapshot.value?.config.mode === "server-mode" &&
    snapshot.value.service.ownership === "local" &&
    snapshot.value.service.phase === "running" &&
    snapshot.value.capabilities.serverMode &&
    lanAccessUrls.value.length === 0,
);
const firewallStatus = computed(() => snapshot.value?.service.firewall?.status);
const firewallProviderName = computed(() =>
  snapshot.value?.platform === "desktop" && snapshot.value.capabilities.serverMode
    ? "Windows 防火墙"
    : "系统防火墙",
);
const firewallStatusMessage = computed(() => {
  switch (firewallStatus.value) {
    case "ready":
      return "当前端口已允许局域网访问。";
    case "requires-elevation":
      return `尚未完成${firewallProviderName.value}授权，其他设备可能无法连接。`;
    case "blocked-by-policy":
      return `系统策略阻止配置${firewallProviderName.value}，其他设备可能无法连接。`;
    case "profile-unsupported":
      return "当前网络属于公用网络，未自动开放局域网端口。";
    case "disabled":
      return `${firewallProviderName.value}未运行，无法确认局域网访问状态。`;
    case "cleanup-pending":
      return "旧的 Windows 防火墙规则尚未清理完成，可能仍保留局域网访问。";
    case "error":
      return `${firewallProviderName.value}状态无法确认，请重试或检查系统设置。`;
    case "not-required":
      return "当前平台不需要自动配置防火墙。";
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
    ? "防火墙规则清理未完成"
    : `${firewallProviderName.value}未完成配置`,
);
const firewallRecoveryDescription = computed(() =>
  firewallStatus.value === "cleanup-pending"
    ? "当前运行方式已经切换，但旧的局域网放行规则还没有删除。"
    : "局域网设备可能无法连接当前服务。",
);
const firewallRecoveryDetail = computed(() =>
  firewallStatus.value === "cleanup-pending"
    ? `可以继续使用当前运行方式，也可以再次确认${firewallProviderName.value}系统权限并重试清理。`
    : `可以继续使用当前服务，也可以再次确认${firewallProviderName.value}系统权限并重试配置。`,
);
const firewallRepairActionLabel = computed(() =>
  firewallStatus.value === "cleanup-pending" ? "重试清理" : "重试授权",
);
const firewallRepairButtonLabel = computed(() =>
  firewallStatus.value === "cleanup-pending" ? "重试清理" : "重试防火墙设置",
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
  remoteMode.value ? "连接已有服务器" : serverMode.value ? "允许其他设备连接" : "在本机使用",
);
const bindHostHint = computed(() =>
  serverMode.value ? "默认值适用于大多数局域网环境。" : "本机模式固定为 127.0.0.1。",
);
const serverModeDisabledReason = computed(() => {
  if (isPureWebPlatform.value) {
    return "浏览器无法在本机启动服务，不能作为局域网服务器。";
  }
  if (snapshot.value?.capabilities.serverMode) return "";
  if (snapshot.value?.platform === "android") {
    return "Android 当前只支持本机 127.0.0.1，自身不能作为局域网服务器。";
  }
  return "当前平台暂不支持自动配置防火墙，请手动配置。";
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
    label: "在本机使用",
    description: "数据保存在这台设备上，适合只在这台设备使用。",
    selected: draft.value.mode === "self-hosted",
    disabled: isPureWebPlatform.value,
    disabledReason: isPureWebPlatform.value ? "浏览器无法在本机启动服务，请连接已有服务器。" : "",
  },
  {
    value: "client-only" as const,
    label: "连接已有服务器",
    description: "多台设备共享同一台服务器上的数据。",
    selected: remoteMode.value,
    disabled: false,
    disabledReason: "",
  },
  {
    value: "server-mode" as const,
    label: "允许其他设备连接",
    description: "让同一网络中的设备使用这台设备的数据。",
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
  if (firewallStatus.value === "requires-elevation") return "需要防火墙授权";
  if (firewallStatus.value === "blocked-by-policy") return "防火墙规则被系统策略阻止";
  if (firewallStatus.value === "profile-unsupported") return "当前网络配置文件不支持自动放行";
  if (firewallStatus.value === "disabled") return "防火墙未启用，局域网保护状态未知";
  if (firewallStatus.value === "cleanup-pending") return "旧防火墙规则尚未清理";
  if (firewallStatus.value === "error") return "防火墙规则更新失败";
  if (phase === "starting") return "正在启动";
  if (phase === "stopping") return "正在停止";
  if (phase === "failed") return "本机服务启动失败";
  if (!activeAddress.value) return "尚未连接服务";
  if (serviceAvailabilityStatus.value === "available") return "服务连接正常";
  if (serviceAvailabilityStatus.value === "unavailable") return "暂时无法连接服务";
  return "正在检查连接";
});
const confirmationDescription = computed(() => {
  if (serverPortChanging.value) return "保存后，应用会使用新的服务端口。";
  if (!enablingLanAccess.value) return "保存后，应用会改用新的服务地址。";
  return "保存后，同一网络中的设备可以连接 WineStock。";
});
const confirmationDetail = computed(() =>
  runtimeChangeClearsSession.value
    ? "切换服务后，当前登录状态会被清除，可能需要重新登录。"
    : "请确认继续保存这项设置。",
);

watch(
  runtimeSnapshot,
  (next) => {
    if (next && !applying.value)
      draft.value = coerceDraftForPlatform(cloneRuntimeConfig(next.config));
  },
  { immediate: true },
);
watch(
  () => draft.value.remoteBaseUrl,
  () => (remoteTestMessage.value = ""),
);
watch(shellRuntimeError, (error) => {
  if (error) notice.error("运行环境初始化失败", { detail: error });
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
  remoteTestMessage.value = "";
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
  fieldErrors.value = validation.fieldErrors;
  if (!validation.valid) {
    notice.warning("请检查运行设置", {
      detail: Object.values(validation.fieldErrors)[0]?.[0] ?? "请检查输入内容",
    });
    return;
  }
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
    notice.error("无法确认当前用户密码状态", { detail: "请稍后重试。" });
    return "blocked";
  }
}

function openPasswordGate(): void {
  gateUsername.value = authSession.value?.user.username ?? "";
  gatePassword.value = "";
  gatePasswordConfirm.value = "";
  gateUsernameError.value = "";
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
  const normalizedUsername = gateUsername.value.trim();
  gateUsernameError.value = !normalizedUsername
    ? "请输入当前用户名"
    : normalizedUsername.length > 64
      ? "用户名不能超过 64 个字符"
      : "";
  gateFieldError.value = gatePassword.value.length < 8 ? "密码至少需要 8 个字符" : "";
  gateConfirmError.value =
    gatePassword.value === gatePasswordConfirm.value ? "" : "两次输入的密码不一致";
  if (gateUsernameError.value || gateFieldError.value || gateConfirmError.value) {
    notice.warning("请检查当前用户账号", {
      detail: gateUsernameError.value || gateFieldError.value || gateConfirmError.value,
    });
    return;
  }

  gateSubmitting.value = true;
  try {
    await changeOwnPassword({
      username: normalizedUsername,
      current_password: "",
      new_password: gatePassword.value,
    });
    replaceCurrentSessionUser(await getCurrentUser());
    passwordGateOpen.value = false;
    notice.success("当前用户账号已设置");
    confirmationOpen.value = true;
  } catch (error) {
    notice.error("设置当前用户密码失败", {
      detail: error instanceof Error ? error.message : "请重试。",
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
    fieldErrors.value = result.fieldErrors;
    if (!result.applied) {
      notice.error("设置保存失败", {
        detail:
          result.error?.message ??
          Object.values(result.fieldErrors)[0]?.[0] ??
          "设置没有保存，请检查后重试",
      });
      return;
    }
    draft.value = cloneRuntimeConfig(result.snapshot.config);
    if (firewallRecoveryRequired.value) {
      firewallRecoveryOpen.value = true;
      notice.warning("运行设置已保存，但防火墙未完成", {
        detail: "可以继续使用，或在此重试防火墙操作。",
      });
    } else {
      notice.success("运行设置已保存");
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
    notice.error("设置保存失败", {
      detail: error instanceof Error ? error.message : "请稍后重试。",
    });
  } finally {
    applying.value = false;
  }
}

async function testRemoteConnection(): Promise<void> {
  const validation = await validateRuntimeConfig(draft.value);
  fieldErrors.value = validation.fieldErrors;
  if (!validation.valid) {
    notice.warning("请检查远程连接设置", {
      detail: Object.values(validation.fieldErrors)[0]?.[0] ?? "请检查服务器地址",
    });
    return;
  }
  if (!remoteMode.value) return;
  testingRemote.value = true;
  remoteTestMessage.value = "";
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
    remoteTestTone.value = "success";
    remoteTestMessage.value = "连接成功";
  } catch (error) {
    remoteTestTone.value =
      error instanceof DOMException && error.name === "AbortError" ? "warning" : "danger";
    remoteTestMessage.value =
      error instanceof DOMException && error.name === "AbortError" ? "连接超时" : "暂时无法连接";
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
      previousFirewallStatus === "cleanup-pending" ? "防火墙规则已清理" : "防火墙设置已完成",
    );
  } catch (error) {
    notice.error("防火墙操作失败", {
      detail: error instanceof Error ? error.message : "请重试。",
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
    notice.warning("请先保存运行设置，再继续。", { detail: "保存成功后才能离开此页面。" });
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
