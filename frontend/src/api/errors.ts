// 本文件拥有 frontend API client 的稳定错误类型和后端错误响应解析；它不决定页面提示文案。

/** 后端字段级校验错误。 */
export interface ApiValidationField {
  /** 后端 DTO 字段路径。 */
  path: string;
  /** 后端返回的安全校验提示。 */
  message: string;
}

/** 后端统一错误响应主体。 */
export interface ApiErrorBody {
  /** 稳定错误代码，页面逻辑应优先按此值分支。 */
  code: string;
  /** 后端提供的安全默认提示。 */
  message: string;
  /** 字段校验或其它结构化错误详情。 */
  details: unknown;
}

/** 后端统一错误响应外层结构。 */
export interface ApiErrorResponse {
  /** 统一错误主体。 */
  error: ApiErrorBody;
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
    super("无法连接到 WineStock 服务", { cause });
    this.name = "ApiNetworkError";
  }
}

/** 服务成功响应的内容无法按声明格式解析。 */
export class ApiResponseError extends Error {
  /** 发生解析错误的请求地址。 */
  readonly url: string;

  constructor(url: string, cause: unknown) {
    super("WineStock 服务返回了无法解析的响应", { cause });
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
  /** 按字段路径聚合后的校验提示。 */
  readonly fieldErrors: Readonly<Record<string, readonly string[]>>;

  constructor(status: number, body: ApiErrorBody, url: string) {
    super(body.message);
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
    messages.push(field.message);
    result[field.path] = messages;
  }

  return result;
}

function isValidationField(value: unknown): value is ApiValidationField {
  return isRecord(value) && typeof value.path === "string" && typeof value.message === "string";
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
