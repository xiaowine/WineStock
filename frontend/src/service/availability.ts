// 本文件拥有 frontend 服务可用性探测与恢复调度；它不启动 Axum、不管理鉴权 token，也不决定具体页面布局。
// ownership=local 时 Shell phase 推送是权威信号，HTTP 探测降为看门狗；
// 纯决策规则在 availabilityPolicy.ts，设计见 docs/implementation-notes/shell-aware-service-availability.md。
import { readonly, ref, watch } from "vue";
import { checkHealth } from "../api/health";
import { trackTelemetryIssue } from "../telemetry/clarity";
import {
  CONFIRM_RECHECK_DELAY_MS,
  DEFAULT_SHELL_SIGNAL,
  LOCAL_RECOVERY_WINDOW_MS,
  UNAVAILABLE_CHECK_INTERVAL_MS,
  deriveStatusFromShellSignal,
  isShellDrivenStatus,
  pickNextCheckDelayMs,
  shouldConfirmBeforeUnavailable,
  type ServiceAvailabilityStatus,
  type ShellServiceAvailabilitySignal,
} from "./availabilityPolicy";

export type { ServiceAvailabilityStatus, ShellServiceAvailabilitySignal };

const HEALTH_CHECK_TIMEOUT_MS = 4_000;

const mutableStatus = ref<ServiceAvailabilityStatus>("checking");
const mutableIsChecking = ref(false);
const mutableSuccessfulCheckSequence = ref(0);
let monitorStarted = false;
let scheduledCheck: number | null = null;
let checkInFlight: Promise<void> | null = null;
let activeHealthController: AbortController | null = null;
let runtimeGeneration = 0;
let shellSignal: ShellServiceAvailabilitySignal = DEFAULT_SHELL_SIGNAL;
let recoveryDeadline: number | null = null;
let confirmRetryUsed = false;
let confirmRecheckPending = false;

/** 只读服务可用性；根应用据此决定是否阻断业务页面。 */
export const serviceAvailabilityStatus = readonly(mutableStatus);

// 进入断连状态记一次排查事件（含会话升级）；按状态翻转去重，不随重复探测反复上报。
watch(mutableStatus, (status, previous) => {
  if (status === "unavailable" && previous !== "unavailable") {
    trackTelemetryIssue("service_unavailable");
  }
});

/** 只读探测进行状态；用于禁用重复手动重试。 */
export const isCheckingServiceAvailability = readonly(mutableIsChecking);

/** 每次健康检查成功后递增，供会话层在服务恢复时执行一次恢复。 */
export const successfulServiceCheckSequence = readonly(mutableSuccessfulCheckSequence);

/**
 * 业务 API 已确认网络连接失败时的去抖入口：不再单次失败立即翻转，
 * 而是立即触发一次健康检查确认，由确认结果决定是否进入断连状态。
 * Shell phase 已直接决定状态时（本地 failed/stopped 等）忽略请求层报告。
 */
export function reportServiceUnavailable(): void {
  if (deriveStatusFromShellSignal(shellSignal) !== null) {
    return;
  }
  if (mutableStatus.value === "unavailable") {
    clearScheduledCheck();
    scheduleNextCheck();
    return;
  }
  void checkServiceAvailability();
}

/**
 * 应用 Shell 快照映射出的可用性信号（每次快照推送都会调用）。
 * local 非 running phase 直接决定状态；回到 running 或切为 remote 时立即确认一次 HTTP。
 */
export function applyShellServiceStateSignal(signal: ShellServiceAvailabilitySignal): void {
  shellSignal = signal;
  const derived = deriveStatusFromShellSignal(signal);

  if (derived === null) {
    clearRecoveryDeadline();
    confirmRetryUsed = false;
    if (isShellDrivenStatus(mutableStatus.value) || mutableStatus.value === "checking") {
      mutableStatus.value = "checking";
      if (monitorStarted) {
        void checkServiceAvailability();
      }
    }
    return;
  }

  if (derived === "recovering") {
    mutableStatus.value = "recovering";
    clearScheduledCheck();
    armRecoveryDeadline();
    return;
  }

  clearRecoveryDeadline();
  mutableStatus.value = derived;
  if (derived === "stopped") {
    clearScheduledCheck();
  }
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

/** API 根地址改变或应用回前台时取消旧探测，按保留的 Shell 信号重新推导状态。 */
export function resetServiceAvailabilityForRuntimeChange(): void {
  runtimeGeneration += 1;
  activeHealthController?.abort();
  activeHealthController = null;
  clearScheduledCheck();
  clearRecoveryDeadline();
  checkInFlight = null;
  mutableIsChecking.value = false;
  confirmRetryUsed = false;
  confirmRecheckPending = false;

  const derived = deriveStatusFromShellSignal(shellSignal);
  if (derived !== null) {
    mutableStatus.value = derived;
    if (derived === "recovering") {
      armRecoveryDeadline();
    }
    return;
  }
  mutableStatus.value = "checking";
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
    confirmRetryUsed = false;
    mutableStatus.value = "available";
    mutableSuccessfulCheckSequence.value += 1;
  } catch {
    if (generation !== runtimeGeneration) {
      return;
    }
    const derived = deriveStatusFromShellSignal(shellSignal);
    if (derived !== null) {
      // Shell phase 是权威：starting/stopping/failed/stopped 期间的 HTTP 失败不额外翻转。
      mutableStatus.value = derived;
      return;
    }
    if (shouldConfirmBeforeUnavailable(shellSignal, mutableStatus.value, confirmRetryUsed)) {
      confirmRetryUsed = true;
      confirmRecheckPending = true;
      return;
    }
    mutableStatus.value = "unavailable";
  } finally {
    window.clearTimeout(timeout);
    if (activeHealthController === controller) {
      activeHealthController = null;
    }
  }
}

function scheduleNextCheck(): void {
  if (document.visibilityState === "hidden") {
    confirmRecheckPending = false;
    return;
  }

  let delay: number | null;
  if (confirmRecheckPending) {
    confirmRecheckPending = false;
    delay = CONFIRM_RECHECK_DELAY_MS;
  } else {
    delay = pickNextCheckDelayMs(shellSignal, mutableStatus.value);
  }
  if (delay === null) {
    return;
  }
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

function armRecoveryDeadline(): void {
  if (recoveryDeadline !== null || typeof window === "undefined") {
    return;
  }
  recoveryDeadline = window.setTimeout(() => {
    recoveryDeadline = null;
    if (shellSignal.ownership !== "local") {
      return;
    }
    if (shellSignal.phase === "failed") {
      // Shell 自动重启窗口耗尽，升级为阻断错误并恢复低频探测。
      mutableStatus.value = "unavailable";
      clearScheduledCheck();
      scheduleNextCheck();
      return;
    }
    if (shellSignal.phase === "starting") {
      // 重启尝试仍在进行，短暂顺延而不是无限重置整个窗口。
      recoveryDeadline = window.setTimeout(() => {
        recoveryDeadline = null;
        if (shellSignal.ownership === "local" && shellSignal.phase === "failed") {
          mutableStatus.value = "unavailable";
          clearScheduledCheck();
          scheduleNextCheck();
        }
      }, UNAVAILABLE_CHECK_INTERVAL_MS);
    }
  }, LOCAL_RECOVERY_WINDOW_MS);
}

function clearRecoveryDeadline(): void {
  if (recoveryDeadline === null) {
    return;
  }
  window.clearTimeout(recoveryDeadline);
  recoveryDeadline = null;
}

function handleServiceWake(): void {
  if (isShellDrivenStatus(mutableStatus.value)) {
    return;
  }
  void checkServiceAvailability();
}

function handleVisibilityChange(): void {
  if (document.visibilityState === "visible") {
    handleServiceWake();
    return;
  }
  clearScheduledCheck();
}
