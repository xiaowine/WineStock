// 本文件拥有入库草稿页的错误文案与轻量格式化，属于 frontend 展示层；它不修改草稿状态。
import {
  ApiConfigurationError,
  ApiError,
  ApiNetworkError,
  ApiResponseError,
} from "../../api/errors";

export function inboundSubmitErrorMessage(error: unknown): { title: string; detail?: string } {
  if (error instanceof ApiError) {
    const messages: Record<string, string> = {
      permission_denied: "当前账号没有创建入库单的权限",
      item_not_found: "某条明细的物品已失效，请移除后重新选择",
      location_not_found: "某条明细的库位已失效，请重新选择",
      invalid_request: "入库单字段不符合服务端校验规则",
    };
    return {
      title: messages[error.code] ?? "提交入库单失败",
      detail: messages[error.code] ? error.message : `${error.message}（${error.code}）`,
    };
  }
  if (error instanceof ApiNetworkError)
    return { title: "无法连接到 WineStock 服务", detail: "草稿仍保存在本机，请恢复连接后重试。" };
  if (error instanceof ApiResponseError)
    return { title: "服务响应版本不匹配", detail: "请确认前后端版本一致后重试。" };
  if (error instanceof ApiConfigurationError)
    return { title: "服务地址配置无效", detail: error.message };
  return { title: "提交入库单失败", detail: "草稿仍保存在本机，请稍后重试。" };
}

export function itemErrorMessage(error: unknown, fallback = "加载物品失败"): string {
  if (error instanceof ApiError)
    return error.code === "permission_denied" ? "当前账号没有执行此操作的权限" : error.message;
  if (error instanceof ApiConfigurationError) return error.message;
  if (error instanceof ApiNetworkError) return "无法连接到 WineStock 服务";
  if (error instanceof ApiResponseError) return "服务响应格式无效，请检查前后端版本";
  return fallback;
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
