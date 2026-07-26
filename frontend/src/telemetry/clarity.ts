// 本文件拥有 Microsoft Clarity 的按需加载与采集操作封装：仅在用户同意后动态 import SDK
// 并初始化一次；业务代码只调用本文件的事件封装，未同意时全部为空操作、零网络请求。
// 它不拥有同意偏好的读写（见 consent.ts）。事件只传固定名字，不携带任何业务数据；
// 设计边界见 docs/implementation-notes/first-run-setup-wizard.md「数据收集偏好与 Clarity 边界」。
import { resolveApiClientMetadata } from "../api/runtime-config";
import { readTelemetryConsent } from "./consent";

/** WineStock 的 Clarity 项目 ID；随 tag 脚本公开分发，不是密钥。留空可整体停用采集。 */
const CLARITY_PROJECT_ID = "xsl053wgz1";

/** 问题事件的固定会话升级原因；Clarity 会优先完整保留升级会话的录制。 */
const ISSUE_UPGRADE_REASON = "issue";

let starting = false;
let started = false;
/** 本会话内曾启动后又被撤回；SDK 无法在同一会话内安全重启，再同意时留待下次启动生效。 */
let stoppedThisSession = false;

/**
 * 依据既有同意偏好启动采集；未同意、已启动或项目 ID 为空时为空操作。
 * 启动入口有二：应用装配时按持久化偏好补启动，向导勾选同意后立即启动。
 * 加载失败（离线、本机回环无外网）静默放弃，下次冷启动自动重试——文案「仅在联网时生效」即此。
 */
export function startTelemetryIfConsented(): void {
  if (started || starting || stoppedThisSession || !CLARITY_PROJECT_ID) return;
  if (readTelemetryConsent() !== true) return;
  starting = true;
  void import("@microsoft/clarity")
    .then((module) => {
      module.default.init(CLARITY_PROJECT_ID);
      started = true;
      // 会话标签复用 Shell 注入的客户端元数据，便于按端侧/版本筛选排查；
      // deviceName 可能含个人信息（如设备主人姓名），按承诺不上报。
      const metadata = resolveApiClientMetadata();
      module.default.setTag("platform", metadata.clientKind);
      module.default.setTag("appVersion", metadata.appVersion);
    })
    .catch(() => {
      // SDK chunk 加载失败按未启动处理；不重试轮询，避免离线场景反复请求。
    })
    .finally(() => {
      starting = false;
    });
}

/**
 * 撤回同意后尽力停止当前会话的采集；SDK 从未加载时本就没有采集，为空操作。
 * 停止后本会话不再重启（重复 init 会触发 Clarity 多实例告警），再同意自下次启动生效。
 */
export function stopTelemetry(): void {
  if (!started) return;
  started = false;
  stoppedThisSession = true;
  callClarity("stop");
}

/** 记录一次流程事件（如 inbound_submitted）；未同意或未启动时为空操作。 */
export function trackTelemetryEvent(name: string): void {
  if (!started) return;
  callClarity("event", name);
}

/**
 * 记录一次问题事件并升级当前会话优先级，让出问题的会话优先保留完整录制。
 * 未同意或未启动时为空操作。
 */
export function trackTelemetryIssue(name: string): void {
  if (!started) return;
  callClarity("event", name);
  callClarity("upgrade", ISSUE_UPGRADE_REASON);
}

/** 采集调用统一走 SDK 暴露的全局入口；异常吞掉，遥测永不影响业务流程。 */
function callClarity(method: string, ...args: string[]): void {
  try {
    (window as { clarity?: (method: string, ...args: string[]) => void }).clarity?.(
      method,
      ...args,
    );
  } catch {
    // 采集调用失败静默忽略。
  }
}
