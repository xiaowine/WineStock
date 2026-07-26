// 本文件拥有运行配置「服务可访问」与「设置流程已确认」的纯判定；它不读写 Shell Bridge、不持有响应式状态。
import type { RuntimeConfigStatus } from "./contract";

/** 就绪/设置判定所需的最小 Shell 快照形状。 */
export interface RuntimeReadinessSnapshot {
  readonly configStatus: RuntimeConfigStatus;
  /** Shell 对“配置是否已初始化”的权威判断。 */
  readonly initialized: boolean;
  readonly service: {
    readonly apiBaseUrl?: string;
  };
}

/**
 * 服务是否可供 HTTP（configured 且存在有效 apiBaseUrl）。
 * 该判定只描述连接状态，不代替 Shell 的 initialized 标记。
 */
export function isRuntimeServiceReady(
  snapshot: RuntimeReadinessSnapshot | null | undefined,
): boolean {
  if (!snapshot) {
    return false;
  }
  if (snapshot.configStatus !== "configured") {
    return false;
  }
  return Boolean(snapshot.service.apiBaseUrl);
}

/**
 * 启动漏斗「设置已完成」：Shell 已确认配置初始化，且服务可通过 HTTP 访问。
 *
 * initialized 由 Shell 根据持久化/成功应用结果发布，前端不再从地址或配置状态推断。
 */
export function isRuntimeSetupFinished(
  snapshot: RuntimeReadinessSnapshot | null | undefined,
): boolean {
  if (!snapshot || !snapshot.initialized) {
    return false;
  }
  return isRuntimeServiceReady(snapshot);
}

/**
 * 首次未配置状态是否应进入初始化向导。
 * 仅 `unconfigured` 且未初始化走向导；`invalid`（配置损坏）、已配置但服务未就绪
 * 以及快照缺失（Shell Bridge 初始化失败）都维持运行设置页修复路径。
 * 设计见 docs/implementation-notes/first-run-setup-wizard.md。
 */
export function shouldEnterSetupWizard(
  snapshot: RuntimeReadinessSnapshot | null | undefined,
): boolean {
  return Boolean(snapshot && snapshot.configStatus === "unconfigured" && !snapshot.initialized);
}
