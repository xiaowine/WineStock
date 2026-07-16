// 本文件拥有 frontend 服务可用性探测与恢复调度；它不启动 Axum、不管理鉴权 token，也不决定具体页面布局。
import { readonly, ref } from "vue";
import { checkHealth } from "../api/health";

const AVAILABLE_CHECK_INTERVAL_MS = 15_000;
const UNAVAILABLE_CHECK_INTERVAL_MS = 5_000;
const HEALTH_CHECK_TIMEOUT_MS = 4_000;

/** 当前配置服务的可用性状态。 */
export type ServiceAvailabilityStatus = "checking" | "available" | "unavailable";

const mutableStatus = ref<ServiceAvailabilityStatus>("checking");
const mutableIsChecking = ref(false);
const mutableSuccessfulCheckSequence = ref(0);
let monitorStarted = false;
let scheduledCheck: number | null = null;
let checkInFlight: Promise<void> | null = null;
let activeHealthController: AbortController | null = null;
let runtimeGeneration = 0;

/** 只读服务可用性；根应用据此决定是否阻断业务页面。 */
export const serviceAvailabilityStatus = readonly(mutableStatus);

/** 只读探测进行状态；用于禁用重复手动重试。 */
export const isCheckingServiceAvailability = readonly(mutableIsChecking);

/** 每次健康检查成功后递增，供会话层在服务恢复时执行一次恢复。 */
export const successfulServiceCheckSequence = readonly(mutableSuccessfulCheckSequence);

/** 业务 API 已确认网络连接失败时立即进入全屏断连状态，并安排下一次健康检查。 */
export function reportServiceUnavailable(): void {
  mutableStatus.value = "unavailable";
  clearScheduledCheck();
  scheduleNextCheck();
}

/**
 * 启动全局服务可用性监控。
 * 它会立即检查，并在前台按可用性选择轮询间隔；窗口唤醒或恢复联网时会立即补检。
 */
export function startServiceAvailabilityMonitor(): void {
  if (monitorStarted || typeof window === "undefined" || typeof document === "undefined") {
    return;
  }

  monitorStarted = true;
  window.addEventListener("focus", handleServiceWake);
  window.addEventListener("online", handleServiceWake);
  document.addEventListener("visibilitychange", handleVisibilityChange);
  void checkServiceAvailability();
}

/** 立即检查服务；并发触发会复用同一个请求。 */
export function checkServiceAvailability(): Promise<void> {
  if (checkInFlight) {
    return checkInFlight;
  }

  clearScheduledCheck();
  mutableIsChecking.value = true;
  const generation = runtimeGeneration;
  const task = performHealthCheck(generation).finally(() => {
    if (generation !== runtimeGeneration) {
      return;
    }
    mutableIsChecking.value = false;
    checkInFlight = null;
    scheduleNextCheck();
  });
  checkInFlight = task;
  return task;
}

/** API 根地址改变时取消旧探测，并从 checking 状态重新检查新服务。 */
export function resetServiceAvailabilityForRuntimeChange(): void {
  runtimeGeneration += 1;
  activeHealthController?.abort();
  activeHealthController = null;
  clearScheduledCheck();
  checkInFlight = null;
  mutableStatus.value = "checking";
  mutableIsChecking.value = false;
  if (monitorStarted) {
    void checkServiceAvailability();
  }
}

async function performHealthCheck(generation: number): Promise<void> {
  const controller = new AbortController();
  activeHealthController = controller;
  const timeout = window.setTimeout(() => controller.abort(), HEALTH_CHECK_TIMEOUT_MS);

  try {
    await checkHealth(controller.signal);
    if (generation !== runtimeGeneration) {
      return;
    }
    mutableStatus.value = "available";
    mutableSuccessfulCheckSequence.value += 1;
  } catch {
    if (generation === runtimeGeneration) {
      mutableStatus.value = "unavailable";
    }
  } finally {
    window.clearTimeout(timeout);
    if (activeHealthController === controller) {
      activeHealthController = null;
    }
  }
}

function scheduleNextCheck(): void {
  if (document.visibilityState === "hidden") {
    return;
  }

  const delay =
    mutableStatus.value === "available"
      ? AVAILABLE_CHECK_INTERVAL_MS
      : UNAVAILABLE_CHECK_INTERVAL_MS;
  scheduledCheck = window.setTimeout(() => {
    scheduledCheck = null;
    void checkServiceAvailability();
  }, delay);
}

function clearScheduledCheck(): void {
  if (scheduledCheck === null) {
    return;
  }
  window.clearTimeout(scheduledCheck);
  scheduledCheck = null;
}

function handleServiceWake(): void {
  void checkServiceAvailability();
}

function handleVisibilityChange(): void {
  if (document.visibilityState === "visible") {
    handleServiceWake();
    return;
  }
  clearScheduledCheck();
}
