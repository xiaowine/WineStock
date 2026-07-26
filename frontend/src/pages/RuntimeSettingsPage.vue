<!--
  本文件拥有正式运行设置界面，复用既有 Shell 契约并不直接管理 core 生命周期。
  它不读写平台文件、不直接管理 core 生命周期，也不改变业务 API 契约。
-->
<template>
  <main class="simple-runtime-page">
    <header class="simple-runtime-header">
      <div class="simple-runtime-header__inner">
        <div class="simple-runtime-header__side simple-runtime-header__side--start">
          <button
            v-if="showLeaveAction"
            class="text-button"
            type="button"
            @click="leaveRuntimeSettings"
          >
            {{ leaveActionLabel }}
          </button>
        </div>
        <div class="simple-runtime-header__title"><h1>运行设置</h1></div>
        <div
          class="simple-runtime-header__side simple-runtime-header__side--end"
          aria-hidden="true"
        ></div>
      </div>
    </header>

    <div class="simple-runtime-content">
      <section class="simple-runtime-status" :class="`simple-runtime-status--${statusTone}`">
        <span class="simple-runtime-status__indicator" aria-hidden="true"></span>
        <div>
          <strong>{{ statusTitle }}</strong>
          <span v-if="activeAddress">{{ activeAddress }}</span>
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

      <div v-if="shellRuntimeError" class="form-error" role="alert">{{ shellRuntimeError }}</div>

      <form class="simple-runtime-form" novalidate @submit.prevent="requestApply">
        <fieldset class="simple-runtime-modes" :disabled="applying">
          <legend>这台设备怎么使用 WineStock？</legend>
          <label
            v-for="option in modeOptions"
            :key="option.value"
            class="simple-runtime-mode"
            :class="{
              'simple-runtime-mode--selected': option.selected,
              'simple-runtime-mode--disabled': option.disabled,
            }"
          >
            <input
              type="radio"
              name="simple_runtime_mode"
              :value="option.value"
              :checked="option.selected"
              :disabled="option.disabled"
              @change="changeMode(option.value)"
            />
            <span class="simple-runtime-mode__radio" aria-hidden="true"></span>
            <span>
              <strong>{{ option.label }}</strong>
              <small>{{ option.description }}</small>
              <em v-if="option.disabled">{{ option.disabledReason }}</em>
            </span>
          </label>
        </fieldset>

        <section class="simple-runtime-settings" aria-labelledby="simple-runtime-settings-title">
          <div class="simple-runtime-section-heading">
            <h2 id="simple-runtime-settings-title">{{ modeTitle }}</h2>
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
              name="simple_runtime_remote_base_url"
              type="url"
              inputmode="url"
              autocomplete="url"
              placeholder="https://server.example.com:17890"
              :disabled="applying"
              required
            />
            <div class="simple-runtime-test">
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
                :class="`simple-runtime-test__result--${remoteTestTone}`"
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
            <FormInput
              v-if="serverMode"
              v-model="draft.port"
              label="服务端口"
              validation-key="port"
              :error="fieldError('port')"
              hint="一般无需修改。"
              name="simple_runtime_port"
              type="number"
              inputmode="numeric"
              min="1"
              max="65535"
              :disabled="applying"
              required
            />
            <p v-if="!serverMode" class="simple-runtime-note">
              打开应用时，本机服务会自动启动并选择可用端口。
            </p>
            <div v-else class="form-warning" role="status">
              同一网络中的其他设备将能够连接此服务。
            </div>
            <details v-if="serverMode" class="simple-runtime-advanced">
              <summary>高级设置</summary>
              <FormInput
                v-model="draft.bindHost"
                label="监听地址"
                validation-key="bindHost"
                :error="fieldError('bindHost')"
                :hint="bindHostHint"
                name="simple_runtime_bind_host"
                type="text"
                autocomplete="off"
                :disabled="applying"
                required
              />
            </details>
          </template>

          <button
            v-if="lanAccessUrls.length"
            class="secondary-button simple-runtime-lan-action"
            type="button"
            @click="lanAccessDialogOpen = true"
          >
            查看局域网访问地址
          </button>
          <div v-if="pageError" class="form-error" role="alert">{{ pageError }}</div>
        </section>

        <footer class="simple-runtime-actions">
          <button
            class="secondary-button"
            type="button"
            :disabled="applying || !dirty"
            @click="restoreDraft"
          >
            取消
          </button>
          <button class="primary-button" type="submit" :disabled="applying || !canSave">
            {{ applying ? "正在保存…" : remoteMode ? "连接服务器" : "保存设置" }}
          </button>
        </footer>
      </form>
    </div>

    <ModalDialog
      :open="confirmationOpen"
      :title="enablingLanAccess ? '允许其他设备连接？' : '切换运行服务？'"
      :description="confirmationDescription"
      :busy="applying"
      compact
      @close="confirmationOpen = false"
    >
      <p class="simple-runtime-confirmation">{{ confirmationDetail }}</p>
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
      :open="passwordGateOpen"
      title="先设置管理员密码"
      description="开放给其他设备连接前，需要为本机管理员 admin 设置一个真实密码；其他设备将用它登录。"
      :busy="gateSubmitting"
      compact
      @close="closePasswordGate"
    >
      <FormInput
        v-model="gatePassword"
        label="管理员密码"
        validation-key="gatePassword"
        :error="gateFieldError"
        hint="至少 8 个字符。"
        name="simple_runtime_gate_password"
        type="password"
        autocomplete="new-password"
        :disabled="gateSubmitting"
        required
      />
      <FormInput
        v-model="gatePasswordConfirm"
        label="确认密码"
        validation-key="gatePasswordConfirm"
        :error="gateConfirmError"
        name="simple_runtime_gate_password_confirm"
        type="password"
        autocomplete="new-password"
        :disabled="gateSubmitting"
        required
      />
      <p v-if="gateError" class="form-error" role="alert">{{ gateError }}</p>
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

    <LanAccessDialog
      :open="lanAccessDialogOpen"
      :urls="lanAccessUrls"
      @close="lanAccessDialogOpen = false"
    />
  </main>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { changeOwnPassword, getLocalSessionStatus } from "../api/auth";
import { authSession, authStatus, localSilentAuthActive } from "../auth/session";
import FormInput from "../components/forms/FormInput.vue";
import LanAccessDialog from "../components/runtime/LanAccessDialog.vue";
import ModalDialog from "../components/ModalDialog.vue";
import { useFormValidation } from "../composables/useFormValidation";
import { notice } from "../notices/notice";
import { getDefaultAppRouteName } from "../router/navigation";
import {
  applyRuntimeConfig,
  initializeShellRuntime,
  isRuntimeSetupFinished,
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

const router = useRouter();
const route = useRoute();
const draft = ref<EditableRuntimeConfig>(cloneRuntimeConfig(defaultRuntimeConfig));
const fieldErrors = ref<Partial<Record<RuntimeConfigField, readonly string[]>>>({});
const pageError = ref("");
const applying = ref(false);
const testingRemote = ref(false);
const remoteTestMessage = ref("");
const remoteTestTone = ref<TestTone>("warning");
const confirmationOpen = ref(false);
const lanAccessDialogOpen = ref(false);
const passwordGateOpen = ref(false);
const gatePassword = ref("");
const gatePasswordConfirm = ref("");
const gateSubmitting = ref(false);
const gateError = ref("");
const gateFieldError = ref("");
const gateConfirmError = ref("");
useFormValidation(fieldErrors);

const snapshot = computed(() => runtimeSnapshot.value);
/** 纯网页端只有连接远程服务一种能力：本机/局域网服务器模式禁用，草稿被动纠正为远端。 */
const isPureWebPlatform = computed(() => snapshot.value?.platform === "web");
const remoteMode = computed(() => isRemoteRuntimeMode(draft.value.mode));
const serverMode = computed(() => draft.value.mode === "server-mode");
const activeAddress = computed(() => snapshot.value?.service.apiBaseUrl ?? "");
const previewAddress = computed(() => previewApiBaseUrl(draft.value));
const lanAccessUrls = computed(() => getUsableLanAccessUrls(snapshot.value));
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
/**
 * 未初始化时即使表单与草稿一致也允许保存，
 * 由保存按钮触发 apply，把确认权收在保存路径上。
 */
const canSave = computed(() => dirty.value || !setupFinished.value);
/** 仅设置已确认后才展示离开；未确认必须先「保存设置」。 */
const showLeaveAction = computed(() => setupFinished.value || authStatus.value === "authenticated");
const leaveActionLabel = computed(() =>
  authStatus.value === "authenticated" ? "← 返回应用" : "继续",
);
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
  return "当前平台暂不支持持续提供局域网服务。";
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
    description: "适合只在这台设备上使用。",
    selected: draft.value.mode === "self-hosted",
    disabled: isPureWebPlatform.value,
    disabledReason: isPureWebPlatform.value ? "浏览器无法在本机启动服务，请连接已有服务器。" : "",
  },
  {
    value: "client-only" as const,
    label: "连接已有服务器",
    description: "输入另一台 WineStock 服务器的地址。",
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
  if (phase === "starting") return "正在启动";
  if (phase === "stopping") return "正在停止";
  if (phase === "failed") return "本机服务启动失败";
  if (!activeAddress.value) return "尚未连接服务";
  if (serviceAvailabilityStatus.value === "available") return "服务连接正常";
  if (serviceAvailabilityStatus.value === "unavailable") return "暂时无法连接服务";
  return "正在检查连接";
});
const confirmationDescription = computed(() =>
  enablingLanAccess.value
    ? "保存后，同一网络中的设备可以连接 WineStock。"
    : "保存后，应用会改用新的服务地址。",
);
const confirmationDetail = computed(() =>
  endpointChanging.value || modeChanging.value
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
watch(lanAccessUrls, (urls) => {
  if (!urls.length) lanAccessDialogOpen.value = false;
});
void initializeShellRuntime().catch(() => undefined);

function fieldError(field: RuntimeConfigField): string {
  return fieldErrors.value[field]?.[0] ?? "";
}

function changeMode(mode: RuntimeMode): void {
  draft.value = applyRuntimeModeDefaults(draft.value, mode);
  fieldErrors.value = {};
  pageError.value = "";
  remoteTestMessage.value = "";
}

function restoreDraft(): void {
  if (!snapshot.value) return;
  draft.value = coerceDraftForPlatform(cloneRuntimeConfig(snapshot.value.config));
  fieldErrors.value = {};
  pageError.value = "";
}

/** 纯网页端把本机类草稿纠正为远端；平台 shell 内原样返回。 */
function coerceDraftForPlatform(config: EditableRuntimeConfig): EditableRuntimeConfig {
  if (isPureWebPlatform.value && !isRemoteRuntimeMode(config.mode)) {
    return applyRuntimeModeDefaults(config, "client-only");
  }
  return config;
}

async function requestApply(): Promise<void> {
  pageError.value = "";
  const validation = await validateRuntimeConfig(draft.value);
  fieldErrors.value = validation.fieldErrors;
  if (!validation.valid) return;
  const gate = await resolveLocalAdminPasswordGate();
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
 * 管理员密码仍为自动开通的随机占位值时，先设真实密码，否则局域网端无人能登录。
 * 状态查询失败时阻止提交并提示，避免带着占位密码开放局域网。
 */
async function resolveLocalAdminPasswordGate(): Promise<"pass" | "required" | "blocked"> {
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
    pageError.value = "无法确认本机管理员密码状态，请稍后重试";
    notice.error("设置保存失败", { detail: pageError.value });
    return "blocked";
  }
}

function openPasswordGate(): void {
  gatePassword.value = "";
  gatePasswordConfirm.value = "";
  gateError.value = "";
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
  gateFieldError.value = gatePassword.value.length < 8 ? "密码至少需要 8 个字符" : "";
  gateConfirmError.value =
    gatePassword.value === gatePasswordConfirm.value ? "" : "两次输入的密码不一致";
  if (gateFieldError.value || gateConfirmError.value) return;

  gateSubmitting.value = true;
  gateError.value = "";
  try {
    await changeOwnPassword({ current_password: "", new_password: gatePassword.value });
    passwordGateOpen.value = false;
    notice.success("管理员密码已设置");
    confirmationOpen.value = true;
  } catch (error) {
    gateError.value = error instanceof Error ? error.message : "密码设置失败，请重试";
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
  pageError.value = "";
  const wasSetupFinished = setupFinished.value;
  try {
    const result = await applyRuntimeConfig(draft.value);
    fieldErrors.value = result.fieldErrors;
    if (!result.applied) {
      pageError.value = result.error?.message ?? "设置没有保存，请检查后重试";
      notice.error("设置保存失败", { detail: pageError.value });
      return;
    }
    draft.value = cloneRuntimeConfig(result.snapshot.config);
    notice.success("运行设置已保存");
    // 设置从「未完成」变为「已确认」且仍匿名时，自动进入认证入口。
    if (
      !wasSetupFinished &&
      isRuntimeSetupFinished(result.snapshot) &&
      authStatus.value !== "authenticated"
    ) {
      await navigateAfterSetup(true);
    }
  } catch (error) {
    pageError.value = error instanceof Error ? error.message : "设置保存失败";
    notice.error("设置保存失败", { detail: pageError.value });
  } finally {
    applying.value = false;
  }
}

async function testRemoteConnection(): Promise<void> {
  const validation = await validateRuntimeConfig(draft.value);
  fieldErrors.value = validation.fieldErrors;
  if (!validation.valid || !remoteMode.value) return;
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

/**
 * 配置阶段结束：已登录回业务，匿名统一进入 /auth。
 * 不在此处 apply：initialized 只能由 Shell 在「保存设置」成功后发布。
 */
async function leaveRuntimeSettings(): Promise<void> {
  if (authStatus.value !== "authenticated" && !setupFinished.value) {
    pageError.value = "请先保存运行设置，再继续。";
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

<style lang="scss" src="./RuntimeSettingsPage.scss"></style>
