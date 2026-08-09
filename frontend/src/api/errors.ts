// 本文件拥有 frontend API client 的稳定错误类型和后端错误响应解析。
// 错误文案在后端稳定码与本地码命中语言包时统一在构造期本地化；页面只展示 error.message
// 与 fieldErrors，不自行解析后端文案。本地网络/解析错误的文案同样来自语言包。
import type { ApiResponse, ApiSchema } from "./contract";
import { translateMessageOrNull } from "../i18n";

/** 后端字段级校验错误；details 内部结构由解析函数兼容，不在 OpenAPI schema 中定义。 */
export interface ApiValidationField {
  /** 后端 DTO 字段路径。 */
  path: string;
  /** 后端返回的稳定校验码；旧服务端可能缺失，缺失时回退 message。 */
  code?: string;
  /** 后端返回的安全校验提示。 */
  message: string;
}

/** 后端统一错误响应主体。 */
export type ApiErrorBody = ApiResponse<ApiSchema<"ApiErrorBody">>;

/** 后端统一错误响应外层结构。 */
export type ApiErrorResponse = ApiResponse<ApiSchema<"ApiErrorResponse">>;

/** 按本地化消息键取文案；键缺失时原样返回键，避免暴露未翻译的原始文本。 */
function localMessage(key: string): string {
  return translateMessageOrNull(key) ?? key;
}

/** API 运行时配置缺失或无效。 */
export class ApiConfigurationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ApiConfigurationError";
  }
}

/** 浏览器未能建立连接或完成 HTTP 请求。 */
export class ApiNetworkError extends Error {
  constructor(cause: unknown) {
    super(localMessage("error.network_unavailable"), { cause });
    this.name = "ApiNetworkError";
  }
}

/** 服务成功响应的内容无法按声明格式解析。 */
export class ApiResponseError extends Error {
  /** 发生解析错误的请求地址。 */
  readonly url: string;

  constructor(url: string, cause: unknown) {
    super(localMessage("error.response_invalid"), { cause });
    this.name = "ApiResponseError";
    this.url = url;
  }
}

/** 非 2xx HTTP 响应。 */
export class ApiError extends Error {
  /** HTTP 状态码。 */
  readonly status: number;
  /** 后端稳定错误代码。 */
  readonly code: string;
  /** 后端结构化错误详情。 */
  readonly details: unknown;
  /** 发生错误的请求地址。 */
  readonly url: string;
  /** 按字段路径聚合后的本地化校验提示；message 仅兜底。 */
  readonly fieldErrors: Readonly<Record<string, readonly string[]>>;

  constructor(status: number, body: ApiErrorBody, url: string) {
    super(localizeApiErrorMessage(body));
    this.name = "ApiError";
    this.status = status;
    this.code = body.code;
    this.details = body.details;
    this.url = url;
    this.fieldErrors = collectFieldErrors(body.details);
  }
}

/** 判断未知 JSON 是否符合后端统一错误响应契约。 */
export function isApiErrorResponse(value: unknown): value is ApiErrorResponse {
  if (!isRecord(value) || !isRecord(value.error)) {
    return false;
  }

  return typeof value.error.code === "string" && typeof value.error.message === "string";
}

/** 顶层错误按 `error.<code>` 本地化；码未命中语言包时回退后端 message。 */
function localizeApiErrorMessage(body: ApiErrorBody): string {
  return translateMessageOrNull(`error.${body.code}`) ?? body.message;
}

function collectFieldErrors(details: unknown): Readonly<Record<string, readonly string[]>> {
  if (!isRecord(details) || details.kind !== "validation" || !Array.isArray(details.fields)) {
    return {};
  }

  const result: Record<string, string[]> = {};
  for (const field of details.fields) {
    if (!isValidationField(field)) {
      continue;
    }

    const messages = result[field.path] ?? [];
    // 字段级文案按 `validation.<code>` 本地化；无码或未命中时回退后端 message。
    const localized =
      typeof field.code === "string"
        ? (translateMessageOrNull(`validation.${field.code}`) ?? field.message)
        : field.message;
    messages.push(localized);
    result[field.path] = messages;
  }

  return result;
}

function isValidationField(value: unknown): value is ApiValidationField {
  return (
    isRecord(value) &&
    typeof value.path === "string" &&
    typeof value.message === "string" &&
    (value.code === undefined || typeof value.code === "string")
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
