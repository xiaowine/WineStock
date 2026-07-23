// 本文件拥有运行配置「服务可访问」与「设置流程已确认」的纯判定；它不读写 Shell Bridge、不持有响应式状态。
import type { RuntimeConfigStatus } from "./contract";

/**
 * 就绪/设置判定的最小快照形状。
 * 兼容 Vue readonly 快照；createdDefault 缺省视为 false（旧快照）。
 */
export interface RuntimeReadinessSnapshot {
  readonly configStatus: RuntimeConfigStatus;
  readonly createdDefault?: boolean;
  readonly service: {
    readonly apiBaseUrl?: string;
  };
}

/**
 * 服务是否可供 HTTP（configured 且存在有效 apiBaseUrl）。
 * Shell 自动默认（createdDefault）时也可以为 true，便于设置页测连与展示状态。
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
 * 启动漏斗「设置已完成」：服务可 HTTP，且不是 Shell 自动默认（createdDefault）。
 *
 * createdDefault 只应在用户点「保存设置」走 apply 后被 Shell 清为 false；
 * 自动起服不得落盘为正式配置，否则冷启动会误判为已确认。
 */
export function isRuntimeSetupFinished(
  snapshot: RuntimeReadinessSnapshot | null | undefined,
): boolean {
  if (!isRuntimeServiceReady(snapshot)) {
    return false;
  }
  return snapshot?.createdDefault !== true;
}
