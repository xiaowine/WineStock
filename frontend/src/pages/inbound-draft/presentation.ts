// 本文件拥有入库草稿页的错误文案与轻量格式化，属于 frontend 展示层；它不修改草稿状态。
import {
  ApiConfigurationError,
  ApiError,
  ApiNetworkError,
  ApiResponseError,
} from "../../api/errors";
import { translateMessageOrNull } from "../../i18n";

export function inboundSubmitErrorMessage(error: unknown): { title: string; detail?: string } {
  if (error instanceof ApiError) {
    // 后端稳定码优先走 error.<code> 语言包；未命中时回退已本地化的 error.message。
    const title = translateMessageOrNull(`error.${error.code}`) ?? error.message;
    return { title, detail: error.message };
  }
  if (error instanceof ApiNetworkError)
    return {
      title: translateMessageOrNull("error.network_unavailable") ?? "error.network_unavailable",
      detail:
        translateMessageOrNull("stockDraft.draftSavedRetryConnect") ??
        "stockDraft.draftSavedRetryConnect",
    };
  if (error instanceof ApiResponseError)
    return {
      title:
        translateMessageOrNull("stockDraft.responseVersionMismatch") ??
        "stockDraft.responseVersionMismatch",
      detail:
        translateMessageOrNull("stockDraft.checkVersionConsistency") ??
        "stockDraft.checkVersionConsistency",
    };
  if (error instanceof ApiConfigurationError)
    return {
      title:
        translateMessageOrNull("stockDraft.serviceConfigInvalid") ??
        "stockDraft.serviceConfigInvalid",
      detail: error.message,
    };
  return {
    title: translateMessageOrNull("stockDraft.submitInboundFailed") ?? "stockDraft.submitInboundFailed",
    detail:
      translateMessageOrNull("stockDraft.draftSavedRetryLater") ?? "stockDraft.draftSavedRetryLater",
  };
}

export function itemErrorMessage(error: unknown, fallback = "stockDraft.loadItemsFailed"): string {
  if (error instanceof ApiError)
    return translateMessageOrNull(`error.${error.code}`) ?? error.message;
  if (error instanceof ApiConfigurationError) return error.message;
  if (error instanceof ApiNetworkError)
    return translateMessageOrNull("error.network_unavailable") ?? translateMessageOrNull(fallback) ?? fallback;
  if (error instanceof ApiResponseError)
    return (
      translateMessageOrNull("stockDraft.responseInvalidCheckVersions") ??
      translateMessageOrNull(fallback) ??
      fallback
    );
  return translateMessageOrNull(fallback) ?? fallback;
}

export function formatMoney(value: number): string {
  return new Intl.NumberFormat("zh-CN", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(value);
}

export function formatQuantity(value: number): string {
  return new Intl.NumberFormat("zh-CN", { maximumFractionDigits: 2 }).format(value);
}

export function isAbortError(error: unknown): boolean {
  return error instanceof DOMException && error.name === "AbortError";
}
