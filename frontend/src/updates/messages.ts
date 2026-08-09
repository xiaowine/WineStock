// 本模块把 Shell Bridge 更新错误码映射为前端安全文案，不解析底层异常字符串。
import { translateMessageOrNull } from "../i18n";

function updateErrorCode(error: unknown): unknown {
  return error instanceof Error ? (error as Error & { code?: unknown }).code : undefined;
}

function localizedUpdateCode(error: unknown): string | null {
  const code = updateErrorCode(error);
  if (typeof code !== "string") return null;
  return translateMessageOrNull(`update.${code}`);
}

export function updateCheckErrorMessage(error: unknown): string {
  return (
    localizedUpdateCode(error) ??
    translateMessageOrNull("update.update_check_retry_later") ??
    "Check your network, or retry later from preferences."
  );
}

export function updateInstallErrorMessage(error: unknown): string {
  return (
    localizedUpdateCode(error) ??
    translateMessageOrNull("update.update_check_unavailable") ??
    "Cannot reach the update service, please retry later."
  );
}
