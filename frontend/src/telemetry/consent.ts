// 本文件拥有匿名使用数据收集偏好的读写；它不加载任何采集 SDK，也不进 Shell 运行配置。
// 偏好为前端自有持久化，键值带版本号：将来收集范围或文案变更时提升版本重新征询。
// 设计见 docs/implementation-notes/first-run-setup-wizard.md。

const STORAGE_KEY = "winestock.telemetry.consent";
const CONSENT_VERSION = 1;

/** 采集由 Microsoft Clarity 提供；数据处理遵循 Microsoft 隐私声明，供各同意入口展示链接。 */
export const TELEMETRY_PROVIDER_NAME = "Microsoft Clarity";
export const TELEMETRY_POLICY_URL = "https://privacy.microsoft.com/zh-cn/privacystatement";

interface StoredTelemetryConsent {
  readonly version: number;
  readonly granted: boolean;
}

/**
 * 读取当前版本的收集偏好；未作答、版本过期或存储不可用时返回 null（视为未同意）。
 */
export function readTelemetryConsent(): boolean | null {
  let serialized: string | null;
  try {
    serialized = window.localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
  if (!serialized) {
    return null;
  }
  try {
    const value = JSON.parse(serialized) as unknown;
    if (
      typeof value === "object" &&
      value !== null &&
      (value as StoredTelemetryConsent).version === CONSENT_VERSION &&
      typeof (value as StoredTelemetryConsent).granted === "boolean"
    ) {
      return (value as StoredTelemetryConsent).granted;
    }
  } catch {
    // JSON 损坏按未作答处理。
  }
  return null;
}

/** 持久化收集偏好；存储不可用时静默失败（等价于未同意，不阻断初始化流程）。 */
export function saveTelemetryConsent(granted: boolean): void {
  const payload: StoredTelemetryConsent = { version: CONSENT_VERSION, granted };
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(payload));
  } catch {
    // 与读取一致：不可用时按未同意运行。
  }
}
