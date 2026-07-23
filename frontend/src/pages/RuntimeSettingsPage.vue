<!--
  本文件拥有 UI 平台运行设置、Shell 状态呈现和配置应用编排，属于 frontend Shell 界面。
  它不读写平台文件、不管理 core 生命周期、不代理库存或鉴权业务 API。
-->
<template>
  <main ref="pageRoot" class="runtime-settings-page">
    <header ref="pageHeader" class="runtime-settings-header">
      <div class="runtime-settings-header__content">
        <div class="brand-lockup">
          <span class="brand-mark">W</span>
          <span class="brand-name">WineStock</span>
        </div>
        <div class="runtime-settings-header__title">
          <button
            v-if="showReturnAction"
            class="text-button"
            type="button"
            @click="returnToApplication"
          >
            ← 返回应用
          </button>
          <div>
            <p class="eyebrow">当前设备</p>
            <h1>本机运行设置</h1>
          </div>
        </div>
        <span class="runtime-platform-badge">{{ platformLabel }}</span>
      </div>
    </header>

    <div class="runtime-settings-workspace">
      <section
        class="runtime-service-summary"
        :class="`runtime-service-summary--${statusTone}`"
        aria-labelledby="runtime-service-title"
      >
        <span class="runtime-service-summary__indicator" aria-hidden="true"></span>
        <div class="runtime-service-summary__copy">
          <p class="eyebrow">当前服务</p>
          <h2 id="runtime-service-title">{{ statusTitle }}</h2>
          <p>{{ statusDetail }}</p>
          <code v-if="activeAddress">{{ activeAddress }}</code>
        </div>
        <div
          v-if="lanAccessUrls.length || canRetryActiveService"
          class="runtime-service-summary__actions"
        >
          <button
            v-if="lanAccessUrls.length"
            class="secondary-button"
            type="button"
            @click="lanAccessDialogOpen = true"
          >
            本机局域网地址
          </button>
          <button
            v-if="canRetryActiveService"
            class="secondary-button"
            type="button"
            :disabled="checkingActiveService"
            @click="retryActiveService"
          >
            {{ checkingActiveService ? "正在检查…" : "重新检查" }}
          </button>
        </div>
      </section>

      <div v-if="shellRuntimeError" class="form-error" role="alert">
        {{ shellRuntimeError }}
      </div>

      <form class="runtime-settings-form" novalidate @submit.prevent="requestApply">
        <aside class="runtime-settings-form__modes">
          <RuntimeModeSelector
            :model-value="draft.mode"
            :server-mode-available="snapshot?.capabilities.serverMode ?? false"
            :disabled="applying"
            @update:model-value="changeMode"
          />
        </aside>

        <section
          class="runtime-settings-form__configuration"
          aria-labelledby="runtime-config-title"
        >
          <header class="runtime-config-header">
            <div>
              <p class="eyebrow">配置</p>
              <h2 id="runtime-config-title">{{ modeTitle }}</h2>
            </div>
            <span v-if="dirty" class="runtime-unsaved-badge">未保存</span>
          </header>

          <div class="runtime-config-fields">
            <template v-if="remoteMode">
              <FormInput
                v-model="draft.remoteBaseUrl"
                label="API 地址"
                validation-key="remoteBaseUrl"
                :error="fieldError('remoteBaseUrl')"
                hint="必须包含 http:// 或 https://，不能使用 0.0.0.0。"
                name="runtime_remote_base_url"
                type="url"
                inputmode="url"
                autocomplete="url"
                placeholder="https://server.example.com:17890"
                :disabled="applying"
                required
              />

              <div class="runtime-connection-test">
                <button
                  class="secondary-button"
                  type="button"
                  :disabled="applying || testingRemote"
                  @click="testRemoteConnection"
                >
                  {{ testingRemote ? "正在测试…" : "测试连接" }}
                </button>
                <p
                  v-if="remoteTestMessage"
                  :class="`runtime-connection-test__result runtime-connection-test__result--${remoteTestTone}`"
                  role="status"
                >
                  {{ remoteTestMessage }}
                </p>
              </div>

              <div v-if="usesInsecureRemoteHttp" class="form-warning" role="status">
                此连接未使用 HTTPS，局域网中的通信可能被其他设备观察。
              </div>
            </template>

            <template v-else>
              <FormInput
                v-model="draft.port"
                label="服务端口"
                validation-key="port"
                :error="fieldError('port')"
                hint="允许使用 1 到 65535 之间的端口。"
                name="runtime_port"
                type="number"
                inputmode="numeric"
                min="1"
                max="65535"
                :disabled="applying"
                required
              />

              <p class="runtime-fixed-behavior">应用启动时会自动运行本地服务。</p>

              <div v-if="serverMode" class="form-warning" role="status">
                局域网中的其他设备可以访问此服务，请确保管理员账户使用安全密码。
              </div>

              <details class="runtime-advanced-settings">
                <summary>高级设置</summary>
                <div class="runtime-advanced-settings__body">
                  <FormInput
                    v-model="draft.bindHost"
                    label="监听地址"
                    validation-key="bindHost"
                    :error="fieldError('bindHost')"
                    :hint="bindHostHint"
                    name="runtime_bind_host"
                    type="text"
                    autocomplete="off"
                    :disabled="applying || draft.mode === 'self-hosted'"
                    required
                  />
                </div>
              </details>
            </template>

            <div class="runtime-address-preview">
              <span>应用后的访问地址</span>
              <code>{{ previewAddress || "等待有效配置" }}</code>
            </div>

            <div v-if="pageError" class="form-error" role="alert">
              {{ pageError }}
            </div>
          </div>

          <footer class="runtime-settings-actions">
            <p>{{ dirty ? "当前草稿尚未应用。" : "当前表单与生效配置一致。" }}</p>
            <div>
              <button
                class="secondary-button"
                type="button"
                :disabled="applying || !dirty"
                @click="restoreDraft"
              >
                取消更改
              </button>
              <button class="primary-button" type="submit" :disabled="applying || !dirty">
                {{ applying ? "正在应用…" : applyButtonLabel }}
              </button>
            </div>
          </footer>
        </section>
      </form>
    </div>

    <ModalDialog
      :open="confirmationOpen"
      :title="confirmationTitle"
      :description="confirmationDescription"
      :busy="applying"
      compact
      @close="confirmationOpen = false"
    >
      <div class="runtime-confirmation">
        <div v-if="activeAddress" class="runtime-confirmation__change">
          <span>当前</span>
          <code>{{ activeAddress }}</code>
          <span>应用后</span>
          <code>{{ previewAddress }}</code>
        </div>
        <p v-if="endpointChanging">
          当前登录会话会从内存中清除，并在新服务上重新恢复或登录。本地业务草稿不得跨服务混用。
        </p>
        <p v-if="enablingLanAccess">
          应用将在配置的网络接口上监听连接，局域网设备可以尝试访问 WineStock 服务。
        </p>
      </div>
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
          {{ applying ? "正在应用…" : "确认并应用" }}
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
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { authSession, authStatus } from "../auth/session";
import ModalDialog from "../components/ModalDialog.vue";
import FormInput from "../components/forms/FormInput.vue";
import RuntimeModeSelector from "../components/runtime/RuntimeModeSelector.vue";
import LanAccessDialog from "../components/runtime/LanAccessDialog.vue";
import { useFormValidation } from "../composables/useFormValidation";
import { notice } from "../notices/notice";
import { getDefaultAppRouteName } from "../router/navigation";
import {
  activeApiBaseUrl,
  applyRuntimeConfig,
  initializeShellRuntime,
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
import {
  applyRuntimeModeDefaults,
  isRemoteRuntimeMode,
  previewApiBaseUrl,
  sameRuntimeConfig,
} from "./runtime-settings/model";

type StatusTone = "neutral" | "success" | "warning" | "danger";
type RemoteTestTone = "success" | "warning" | "danger";

const router = useRouter();
const route = useRoute();
const pageRoot = ref<HTMLElement | null>(null);
const pageHeader = ref<HTMLElement | null>(null);
const draft = ref<EditableRuntimeConfig>(cloneRuntimeConfig(defaultRuntimeConfig));
const fieldErrors = ref<Partial<Record<RuntimeConfigField, readonly string[]>>>({});
const pageError = ref("");
const applying = ref(false);
const confirmationOpen = ref(false);
const lanAccessDialogOpen = ref(false);
const testingRemote = ref(false);
const remoteTestMessage = ref("");
const remoteTestTone = ref<RemoteTestTone>("warning");
useFormValidation(fieldErrors);

/** 同步页头高度给移动端粘滞模式 tab，避免安全区或返回按钮把 tab 顶偏。 */
let stickyHeaderObserver: ResizeObserver | null = null;

function syncStickyHeaderOffset(): void {
  const root = pageRoot.value;
  const header = pageHeader.value;
  if (!root || !header) return;
  root.style.setProperty(
    "--runtime-settings-sticky-header-height",
    `${Math.ceil(header.getBoundingClientRect().height)}px`,
  );
}

const snapshot = computed(() => runtimeSnapshot.value);
const remoteMode = computed(() => isRemoteRuntimeMode(draft.value.mode));
const serverMode = computed(() => draft.value.mode === "server-mode");
const previewAddress = computed(() => previewApiBaseUrl(draft.value));
const activeAddress = computed(() => snapshot.value?.service.apiBaseUrl ?? "");
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
const localServiceStopped = computed(
  () =>
    snapshot.value?.configStatus === "configured" &&
    snapshot.value.service.ownership === "local" &&
    snapshot.value.service.phase === "stopped",
);
const checkingActiveService = computed(() => isCheckingServiceAvailability.value);
const canRetryActiveService = computed(() => Boolean(activeAddress.value));
const showReturnAction = computed(
  () => Boolean(activeApiBaseUrl.value) || authStatus.value === "authenticated",
);
const usesInsecureRemoteHttp = computed(() => {
  if (!remoteMode.value) return false;
  try {
    const url = new URL(draft.value.remoteBaseUrl);
    const hostname = url.hostname.toLowerCase();
    const loopback = hostname === "localhost" || hostname === "127.0.0.1" || hostname === "[::1]";
    return url.protocol === "http:" && !loopback;
  } catch {
    return false;
  }
});
const modeTitle = computed(() =>
  remoteMode.value ? "连接远程服务" : serverMode.value ? "局域网服务器" : "本机运行",
);
const bindHostHint = computed(() =>
  draft.value.mode === "self-hosted"
    ? "本机运行固定使用 127.0.0.1，避免无意开放局域网访问。"
    : "0.0.0.0 表示监听所有 IPv4 接口，但前端仍使用 127.0.0.1 访问。",
);
const platformLabel = computed(() => {
  switch (snapshot.value?.platform) {
    case "desktop":
      return "Desktop";
    case "android":
      return "Android";
    default:
      return "Web";
  }
});
const applyButtonLabel = computed(() => {
  if (remoteMode.value) return "保存并连接";
  return "保存配置";
});
const confirmationTitle = computed(() =>
  enablingLanAccess.value ? "允许局域网设备访问？" : "切换运行服务？",
);
const confirmationDescription = computed(() =>
  enablingLanAccess.value
    ? "保存后 Shell 会按新监听地址重新启动本地服务。"
    : "保存后当前请求会停止，并使用新的 API 地址重新建立连接。",
);
const statusTone = computed<StatusTone>(() => {
  if (snapshot.value?.service.phase === "failed") return "danger";
  if (
    snapshot.value?.service.phase === "starting" ||
    snapshot.value?.service.phase === "stopping"
  ) {
    return "warning";
  }
  if (activeAddress.value && serviceAvailabilityStatus.value === "available") return "success";
  if (activeAddress.value && serviceAvailabilityStatus.value === "unavailable") return "danger";
  return "neutral";
});
const statusTitle = computed(() => {
  const phase = snapshot.value?.service.phase;
  if (phase === "starting") return "正在启动本地服务";
  if (phase === "stopping") return "正在停止本地服务";
  if (phase === "failed") return "本机服务启动失败";
  if (localServiceStopped.value) return "本机服务已停止";
  if (!activeAddress.value) return "尚未配置运行服务";
  if (serviceAvailabilityStatus.value === "available") {
    return snapshot.value?.service.ownership === "remote" ? "远程服务可用" : "本机服务运行中";
  }
  if (serviceAvailabilityStatus.value === "unavailable") {
    return snapshot.value?.service.ownership === "remote" ? "远程服务不可用" : "本机服务暂不可用";
  }
  return "正在检查服务";
});
const statusDetail = computed(() => {
  if (snapshot.value?.service.error) return snapshot.value.service.error.message;
  if (localServiceStopped.value) return "配置仍然保留，本机服务生命周期由应用 Shell 管理。";
  if (!activeAddress.value) return "选择运行方式并保存后，业务页面才会连接 WineStock API。";
  if (serviceAvailabilityStatus.value === "available")
    return "当前地址已经通过 WineStock 健康检查。";
  if (serviceAvailabilityStatus.value === "unavailable") {
    return "配置仍然保留，系统会自动重试；你也可以修改地址或端口。";
  }
  return "正在请求 /api/health，请稍候。";
});

watch(
  runtimeSnapshot,
  (next) => {
    if (next && !applying.value) {
      draft.value = cloneRuntimeConfig(next.config);
    }
  },
  { immediate: true },
);

watch(
  () => draft.value.remoteBaseUrl,
  () => {
    remoteTestMessage.value = "";
  },
);

watch(lanAccessUrls, (urls) => {
  if (!urls.length) lanAccessDialogOpen.value = false;
});

onMounted(() => {
  void nextTick(() => {
    syncStickyHeaderOffset();
    if (typeof ResizeObserver !== "undefined" && pageHeader.value) {
      stickyHeaderObserver = new ResizeObserver(() => {
        syncStickyHeaderOffset();
      });
      stickyHeaderObserver.observe(pageHeader.value);
    }
    window.addEventListener("resize", syncStickyHeaderOffset, { passive: true });
  });
  void initializeShellRuntime().catch(() => undefined);
});

onBeforeUnmount(() => {
  stickyHeaderObserver?.disconnect();
  stickyHeaderObserver = null;
  window.removeEventListener("resize", syncStickyHeaderOffset);
});

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
  draft.value = cloneRuntimeConfig(snapshot.value.config);
  fieldErrors.value = {};
  pageError.value = "";
  remoteTestMessage.value = "";
}

async function requestApply(): Promise<void> {
  pageError.value = "";
  const validation = await validateRuntimeConfig(draft.value);
  fieldErrors.value = validation.fieldErrors;
  if (!validation.valid) return;

  if (endpointChanging.value || modeChanging.value || enablingLanAccess.value) {
    confirmationOpen.value = true;
    return;
  }
  await executeApply();
}

async function applyConfirmed(): Promise<void> {
  confirmationOpen.value = false;
  await executeApply();
}

async function executeApply(): Promise<void> {
  applying.value = true;
  pageError.value = "";
  try {
    const result = await applyRuntimeConfig(draft.value);
    fieldErrors.value = result.fieldErrors;
    if (!result.applied) {
      pageError.value = result.error?.message ?? "运行配置未能应用，请检查输入后重试";
      notice.error("运行配置应用失败", { detail: pageError.value });
      return;
    }
    draft.value = cloneRuntimeConfig(result.snapshot.config);
    notice.success("运行配置已应用", {
      detail: result.snapshot.service.apiBaseUrl ?? "本地服务当前未启动",
    });
  } catch (error) {
    pageError.value = error instanceof Error ? error.message : "运行配置应用失败";
    notice.error("运行配置应用失败", { detail: pageError.value });
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
    if (!response.ok || !isHealthPayload(payload)) {
      throw new Error("健康检查响应无效");
    }
    remoteTestTone.value = "success";
    remoteTestMessage.value = "WineStock 服务可用。";
  } catch (error) {
    remoteTestTone.value =
      error instanceof DOMException && error.name === "AbortError" ? "warning" : "danger";
    remoteTestMessage.value =
      error instanceof DOMException && error.name === "AbortError"
        ? "连接超时；地址格式有效，仍可保存后等待服务恢复。"
        : "当前无法连接；地址格式有效，仍可保存。";
  } finally {
    window.clearTimeout(timeout);
    testingRemote.value = false;
  }
}

async function retryActiveService(): Promise<void> {
  await checkServiceAvailability();
}

async function returnToApplication(): Promise<void> {
  const returnTo = typeof route.query.returnTo === "string" ? route.query.returnTo : "";
  if (returnTo.startsWith("/") && !returnTo.startsWith("//") && !returnTo.includes("\\")) {
    const resolved = router.resolve(returnTo);
    if (
      resolved.matched.length > 0 &&
      resolved.name !== "runtime-settings" &&
      resolved.name !== "home-fallback"
    ) {
      await router.replace({ path: resolved.fullPath });
      return;
    }
  }

  if (authStatus.value === "authenticated") {
    await router.replace({
      name: getDefaultAppRouteName(authSession.value?.user.permissions),
    });
    return;
  }
  if (activeApiBaseUrl.value) {
    await router.replace({ name: "login" });
  }
}

function isHealthPayload(value: unknown): value is { status: "OK" } {
  return typeof value === "object" && value !== null && "status" in value && value.status === "OK";
}
</script>

<style lang="scss" src="./RuntimeSettingsPage.scss"></style>
