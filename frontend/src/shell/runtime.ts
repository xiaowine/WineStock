// 本文件拥有 frontend 的 Shell 运行快照、配置应用和 API 根地址切换；它不实现平台传输或业务页面。
import { computed, readonly, ref, shallowRef } from "vue";
import { apiClient } from "../api/client";
import { configureRuntimeApiBaseUrl } from "../api/runtime-config";
import { resetAuthSessionForRuntimeChange } from "../auth/session";
import { resetServiceAvailabilityForRuntimeChange } from "../service/availability";
import { createShellBridge } from "./bridge";
import {
  assertCompleteShellBridge,
  assertApplyRuntimeConfigResult,
  assertCompatibleRuntimeSnapshot,
  cloneRuntimeSnapshot,
  type ApplyRuntimeConfigResult,
  type EditableRuntimeConfig,
  type RuntimeConfigValidationResult,
  type RuntimeSnapshot,
  type ShellBridge,
  type StopShellSubscription,
  assertRuntimeConfigValidationResult,
} from "./contract";

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

/** 当前是否已经存在可供业务路由使用的 API 根地址。 */
export const hasConfiguredApiService = computed(() => Boolean(activeApiBaseUrl.value));

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

/** 启动当前生效配置的本地服务。 */
export async function startLocalService(): Promise<RuntimeSnapshot> {
  await initializeShellRuntime();
  return applyLifecycleSnapshot(await requireBridge().startLocalService());
}

/** 停止当前本地服务。 */
export async function stopLocalService(): Promise<RuntimeSnapshot> {
  await initializeShellRuntime();
  return applyLifecycleSnapshot(await requireBridge().stopLocalService());
}

/** 重启当前本地服务。 */
export async function restartLocalService(): Promise<RuntimeSnapshot> {
  await initializeShellRuntime();
  return applyLifecycleSnapshot(await requireBridge().restartLocalService());
}

/** 通知平台前端首个稳定画面已经渲染。 */
export async function reportFrontendReady(): Promise<void> {
  await initializeShellRuntime();
  await requireBridge().frontendReady();
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
    applySnapshot(initialSnapshot);
    assertCompleteShellBridge(requireBridge());
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

function applyLifecycleSnapshot(snapshot: RuntimeSnapshot): RuntimeSnapshot {
  applySnapshot(snapshot, activeApiBaseUrl.value);
  return snapshot;
}

function applySnapshot(snapshot: RuntimeSnapshot, previousApiBaseUrl?: string): void {
  assertCompatibleRuntimeSnapshot(snapshot);
  const nextSnapshot = cloneRuntimeSnapshot(snapshot);
  const apiBaseUrlChanged =
    previousApiBaseUrl !== undefined && previousApiBaseUrl !== nextSnapshot.service.apiBaseUrl;

  if (apiBaseUrlChanged) {
    apiClient.cancelRequestsForRuntimeChange();
    resetAuthSessionForRuntimeChange();
  }

  configureRuntimeApiBaseUrl(nextSnapshot.service.apiBaseUrl);
  if (apiBaseUrlChanged) {
    resetServiceAvailabilityForRuntimeChange();
  }
  mutableRuntimeSnapshot.value = nextSnapshot;
}

function requireBridge(): ShellBridge {
  if (!bridge) {
    throw new Error("Shell Bridge 尚未初始化");
  }
  return bridge;
}

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    stopRuntimeStateSubscription?.();
    stopAppResumedSubscription?.();
    stopRuntimeStateSubscription = null;
    stopAppResumedSubscription = null;
  });
}
