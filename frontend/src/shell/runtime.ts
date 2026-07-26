// 本文件拥有 frontend 的 Shell 运行快照、配置应用和 API 根地址切换；它不实现平台传输或业务页面。
import { computed, readonly, ref, shallowRef } from "vue";
import { apiClient } from "../api/client";
import { resetAuthBootstrapStatus } from "../api/auth";
import { configureRuntimeApiBaseUrl } from "../api/runtime-config";
import { resetAuthSessionForRuntimeChange, setLocalAuthExchangeToken } from "../auth/session";
import {
  applyShellServiceStateSignal,
  resetServiceAvailabilityForRuntimeChange,
} from "../service/availability";
import { createShellBridge } from "./bridge";
import {
  assertCompleteShellBridge,
  assertApplyRuntimeConfigResult,
  assertCompatibleRuntimeSnapshot,
  assertNativeBackRequest,
  assertNativeBackResolutionAck,
  assertNativeBackShellBridgeExtension,
  cloneRuntimeSnapshot,
  type ApplyRuntimeConfigResult,
  type EditableRuntimeConfig,
  type RuntimeConfigValidationResult,
  type RuntimeSnapshot,
  type ShellBridge,
  type NativeBackRequest,
  type NativeBackResolution,
  type NativeBackResolutionAck,
  type NativeBackShellBridgeExtension,
  type StopShellSubscription,
  assertRuntimeConfigValidationResult,
} from "./contract";
import { isRuntimeSetupFinished as isRuntimeSetupFinishedSnapshot } from "./runtimeReadiness";

export {
  isRuntimeServiceReady,
  isRuntimeSetupFinished,
  shouldEnterSetupWizard,
} from "./runtimeReadiness";

/** 前端读取 Shell 初始快照的状态。 */
export type ShellRuntimeStatus = "idle" | "loading" | "ready" | "failed";

const mutableRuntimeStatus = ref<ShellRuntimeStatus>("idle");
const mutableRuntimeSnapshot = shallowRef<RuntimeSnapshot | null>(null);
const mutableRuntimeError = ref("");
let bridge: ShellBridge | null = null;
let initializationInFlight: Promise<RuntimeSnapshot> | null = null;
let stopRuntimeStateSubscription: StopShellSubscription | null = null;
let stopAppResumedSubscription: StopShellSubscription | null = null;

/** Shell 快照初始化状态。 */
export const shellRuntimeStatus = readonly(mutableRuntimeStatus);

/** 当前权威运行快照。 */
export const runtimeSnapshot = readonly(mutableRuntimeSnapshot);

/** Shell Bridge 无法初始化时的安全提示。 */
export const shellRuntimeError = readonly(mutableRuntimeError);

/** 当前可供 HTTP client 使用的 API 根地址。 */
export const activeApiBaseUrl = computed(() => mutableRuntimeSnapshot.value?.service.apiBaseUrl);

/**
 * 启动漏斗设置是否已确认完成（Shell initialized 且服务可 HTTP）。
 * initialized 由 Shell 根据成功应用的持久配置或平台显式配置发布。
 */
export const runtimeSetupFinished = computed(() =>
  isRuntimeSetupFinishedSnapshot(mutableRuntimeSnapshot.value),
);

/** 初始化一次 Shell Bridge，并在业务模块启动前写入有效 API 根地址。 */
export function initializeShellRuntime(): Promise<RuntimeSnapshot> {
  if (mutableRuntimeSnapshot.value) {
    return Promise.resolve(mutableRuntimeSnapshot.value);
  }
  if (initializationInFlight) {
    return initializationInFlight;
  }

  mutableRuntimeStatus.value = "loading";
  bridge = createShellBridge();
  const task = performInitialization().finally(() => {
    initializationInFlight = null;
  });
  initializationInFlight = task;
  return task;
}

/** 使用 Shell/shared 规则校验运行配置草稿。 */
export async function validateRuntimeConfig(
  config: EditableRuntimeConfig,
): Promise<RuntimeConfigValidationResult> {
  await initializeShellRuntime();
  const result = await requireBridge().validateRuntimeConfig(config);
  assertRuntimeConfigValidationResult(result);
  return result;
}

/** 保存并应用运行配置；API 地址变化时同步重置健康检查和内存会话。 */
export async function applyRuntimeConfig(
  config: EditableRuntimeConfig,
): Promise<ApplyRuntimeConfigResult> {
  await initializeShellRuntime();
  const previousApiBaseUrl = activeApiBaseUrl.value;
  const result = await requireBridge().applyRuntimeConfig(config);
  assertApplyRuntimeConfigResult(result);
  if (result.applied) {
    applySnapshot(result.snapshot, previousApiBaseUrl);
  }
  return result;
}

/** 通知平台前端首个稳定画面已经渲染。 */
export async function reportFrontendReady(): Promise<void> {
  await initializeShellRuntime();
  await requireBridge().frontendReady();
}

/**
 * 订阅 Android 原生返回请求。普通浏览器或 capability=false 时返回稳定 no-op，且不调用可选扩展。
 */
export async function subscribeNativeBackRequested(
  listener: (request: NativeBackRequest) => void,
): Promise<StopShellSubscription> {
  const snapshot = await initializeShellRuntime();
  if (!snapshot.capabilities.nativeBack) return () => undefined;
  const extension = requireNativeBackBridge();
  const stop = await extension.onNativeBackRequested((request) => {
    assertNativeBackRequest(request);
    listener(request);
  });
  if (typeof stop !== "function") {
    throw new Error("Shell Bridge 原生返回订阅没有返回取消函数");
  }
  return stop;
}

/** 向 Android 结算一次原生返回；仅在 capability=true 时允许调用。 */
export async function resolveNativeBack(
  resolution: NativeBackResolution,
): Promise<NativeBackResolutionAck> {
  const snapshot = await initializeShellRuntime();
  if (!snapshot.capabilities.nativeBack) {
    throw new Error("当前平台未启用原生返回协商");
  }
  const result = await requireNativeBackBridge().resolveNativeBack(resolution);
  assertNativeBackResolutionAck(result);
  return result;
}

/** 请求 Shell 启动本地服务；仅在 capability 允许时调用，端口可能随之变化。 */
export async function startLocalService(): Promise<RuntimeSnapshot> {
  const snapshot = await initializeShellRuntime();
  if (!snapshot.capabilities.startLocalService) {
    throw new Error("当前平台不支持启动本地服务");
  }
  const previousApiBaseUrl = activeApiBaseUrl.value;
  const result = await requireBridge().startLocalService();
  applySnapshot(result, previousApiBaseUrl);
  return result;
}

/** 请求 Shell 重启本地服务；本地服务异常且自动恢复失败后的手动兜底入口。 */
export async function restartLocalService(): Promise<RuntimeSnapshot> {
  const snapshot = await initializeShellRuntime();
  if (!snapshot.capabilities.restartLocalService) {
    throw new Error("当前平台不支持重启本地服务");
  }
  const previousApiBaseUrl = activeApiBaseUrl.value;
  const result = await requireBridge().restartLocalService();
  applySnapshot(result, previousApiBaseUrl);
  return result;
}

/** 通过当前平台的受控能力打开外部 HTTP/HTTPS 地址。 */
export async function openExternal(url: string): Promise<void> {
  const snapshot = await initializeShellRuntime();
  if (!snapshot.capabilities.openExternal) {
    throw new Error("当前平台不支持打开外部链接");
  }
  await requireBridge().openExternal(url);
}

async function performInitialization(): Promise<RuntimeSnapshot> {
  try {
    const initialSnapshot = await requireBridge().getRuntimeSnapshot();
    assertCompatibleRuntimeSnapshot(initialSnapshot);
    assertCompleteShellBridge(requireBridge());
    if (initialSnapshot.capabilities.nativeBack) {
      assertNativeBackShellBridgeExtension(requireBridge());
    }
    applySnapshot(initialSnapshot);
    stopRuntimeStateSubscription = await requireBridge().onRuntimeStateChanged((nextSnapshot) => {
      try {
        applySnapshot(nextSnapshot, activeApiBaseUrl.value);
      } catch (error) {
        mutableRuntimeStatus.value = "failed";
        mutableRuntimeError.value =
          error instanceof Error ? error.message : "Shell Bridge 发布了无效运行快照";
      }
    });
    stopAppResumedSubscription = await requireBridge().onAppResumed(() => {
      resetServiceAvailabilityForRuntimeChange();
    });
    if (
      typeof stopRuntimeStateSubscription !== "function" ||
      typeof stopAppResumedSubscription !== "function"
    ) {
      throw new Error("Shell Bridge 事件订阅没有返回取消函数");
    }
    mutableRuntimeStatus.value = "ready";
    mutableRuntimeError.value = "";
    return initialSnapshot;
  } catch (error) {
    mutableRuntimeStatus.value = "failed";
    mutableRuntimeError.value =
      error instanceof Error ? error.message : "无法读取 WineStock 运行配置";
    configureRuntimeApiBaseUrl(undefined);
    throw error;
  }
}

function applySnapshot(snapshot: RuntimeSnapshot, previousApiBaseUrl?: string): void {
  assertCompatibleRuntimeSnapshot(snapshot);
  const nextSnapshot = cloneRuntimeSnapshot(snapshot);
  const apiBaseUrlChanged =
    previousApiBaseUrl !== undefined && previousApiBaseUrl !== nextSnapshot.service.apiBaseUrl;

  if (apiBaseUrlChanged) {
    apiClient.cancelRequestsForRuntimeChange();
    resetAuthBootstrapStatus();
    resetAuthSessionForRuntimeChange();
  }

  configureRuntimeApiBaseUrl(nextSnapshot.service.apiBaseUrl);
  if (apiBaseUrlChanged) {
    resetServiceAvailabilityForRuntimeChange();
  }
  mutableRuntimeSnapshot.value = nextSnapshot;
  // 换取凭据仅在本地所有权快照下生效；切到远端或凭据缺失即清除，会话层退出静默模式。
  setLocalAuthExchangeToken(
    nextSnapshot.service.ownership === "local"
      ? nextSnapshot.service.localAuthExchangeToken
      : undefined,
  );
  applyShellServiceStateSignal(
    nextSnapshot.service.ownership === "local"
      ? { ownership: "local", phase: nextSnapshot.service.phase }
      : { ownership: "remote" },
  );
}

function requireBridge(): ShellBridge {
  if (!bridge) {
    throw new Error("Shell Bridge 尚未初始化");
  }
  return bridge;
}

function requireNativeBackBridge(): ShellBridge & NativeBackShellBridgeExtension {
  const current = requireBridge();
  assertNativeBackShellBridgeExtension(current);
  return current;
}

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    stopRuntimeStateSubscription?.();
    stopAppResumedSubscription?.();
    stopRuntimeStateSubscription = null;
    stopAppResumedSubscription = null;
  });
}
