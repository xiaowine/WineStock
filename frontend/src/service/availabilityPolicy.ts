// 本文件拥有服务可用性的纯决策规则（信号合成、轮询间隔、恢复窗口）；
// 它不持有响应式状态、不执行 HTTP 请求，也不接触 window/document。
// 设计决策见 docs/implementation-notes/shell-aware-service-availability.md。

/** 可用性状态机对外呈现的全部状态。 */
export type ServiceAvailabilityStatus =
  "checking" | "available" | "recovering" | "stopped" | "unavailable";

/** Shell 快照映射出的可用性输入信号；remote 下 HTTP 探测是唯一权威。 */
export type ShellServiceAvailabilitySignal =
  | { readonly ownership: "remote" }
  | {
      readonly ownership: "local";
      readonly phase: "stopped" | "starting" | "running" | "stopping" | "failed";
    };

/** 未收到任何快照时的保守默认：按 remote 语义仅依赖 HTTP 探测。 */
export const DEFAULT_SHELL_SIGNAL: ShellServiceAvailabilitySignal = { ownership: "remote" };

/** remote 可用时的轮询间隔。 */
export const REMOTE_AVAILABLE_CHECK_INTERVAL_MS = 15_000;
/** local 可用时的看门狗间隔；phase 推送是权威，HTTP 只兜底"进程活着但 HTTP 卡死"。 */
export const LOCAL_AVAILABLE_CHECK_INTERVAL_MS = 60_000;
/** 不可用时的恢复探测间隔（两种 ownership 一致）。 */
export const UNAVAILABLE_CHECK_INTERVAL_MS = 5_000;
/** phase=failed 后等待 Shell 自动重启的窗口；超时仍未 running 则升级为阻断错误。 */
export const LOCAL_RECOVERY_WINDOW_MS = 20_000;
/** 允许确认复查时，失败后到复查之间的短间隔。 */
export const CONFIRM_RECHECK_DELAY_MS = 1_000;

/**
 * 由 Shell 信号直接决定的状态；返回 null 表示交由 HTTP 探测决定
 * （remote 全部、local 且 phase=running）。
 */
export function deriveStatusFromShellSignal(
  signal: ShellServiceAvailabilitySignal,
): Extract<ServiceAvailabilityStatus, "checking" | "recovering" | "stopped"> | null {
  if (signal.ownership === "remote" || signal.phase === "running") {
    return null;
  }
  switch (signal.phase) {
    case "starting":
    case "stopping":
      return "checking";
    case "failed":
      return "recovering";
    case "stopped":
      return "stopped";
  }
}

/** 当前状态是否由 Shell phase 直接决定；此时暂停 HTTP 轮询，phase 变化即时驱动。 */
export function isShellDrivenStatus(status: ServiceAvailabilityStatus): boolean {
  return status === "recovering" || status === "stopped";
}

/** 按 ownership 与当前状态选择下一次 HTTP 探测间隔；null 表示不安排轮询。 */
export function pickNextCheckDelayMs(
  signal: ShellServiceAvailabilitySignal,
  status: ServiceAvailabilityStatus,
): number | null {
  if (isShellDrivenStatus(status)) {
    return null;
  }
  if (status === "available") {
    return signal.ownership === "local"
      ? LOCAL_AVAILABLE_CHECK_INTERVAL_MS
      : REMOTE_AVAILABLE_CHECK_INTERVAL_MS;
  }
  return UNAVAILABLE_CHECK_INTERVAL_MS;
}

/**
 * HTTP 探测失败时是否允许先做一次立即复查而不翻转状态。
 * 仅 local 且 Shell 认为 running 且当前仍 available 时允许一次，
 * 避免瞬时毛刺（如 handler panic 自愈）闪出全屏错误；remote 维持单次失败即翻转。
 */
export function shouldConfirmBeforeUnavailable(
  signal: ShellServiceAvailabilitySignal,
  status: ServiceAvailabilityStatus,
  confirmAlreadyUsed: boolean,
): boolean {
  return (
    signal.ownership === "local" &&
    signal.phase === "running" &&
    status === "available" &&
    !confirmAlreadyUsed
  );
}
