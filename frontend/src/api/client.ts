// 本文件拥有 frontend 通用 HTTP 请求执行，属于 API 边界；它不实现具体业务接口或 token 刷新策略。
import {
  ApiConfigurationError,
  ApiError,
  ApiNetworkError,
  ApiResponseError,
  isApiErrorResponse,
  type ApiErrorBody,
} from "./errors";
import { resolveApiBaseUrl } from "./runtime-config";

/** API 查询参数基础值。 */
export type ApiQueryPrimitive = string | number | boolean;

/** API 查询参数值；数组会重复写入同一个 query key。 */
export type ApiQueryValue = ApiQueryPrimitive | readonly ApiQueryPrimitive[] | null | undefined;

/** API 查询参数集合。 */
export type ApiQuery = Readonly<Record<string, ApiQueryValue>>;

/** API client 支持的 HTTP 方法。 */
export type ApiMethod = "GET" | "POST" | "PUT" | "PATCH" | "DELETE";

/** 单次 API 请求选项。 */
export interface ApiRequestOptions {
  /** HTTP 方法，默认使用 `GET`。 */
  method?: ApiMethod;
  /** URL 查询参数。 */
  query?: ApiQuery;
  /** 需要 JSON 序列化的请求体。 */
  json?: unknown;
  /** 是否尝试附加 access token，默认启用。 */
  authenticated?: boolean;
  /** 调用方补充的请求头。 */
  headers?: HeadersInit;
  /** 调用方控制取消请求的信号。 */
  signal?: AbortSignal;
  /** 成功响应读取方式；文件读取使用 blob，其余默认解析 JSON/文本。 */
  responseType?: "default" | "blob";
}

/** XMLHttpRequest 上传进度快照。 */
export interface ApiUploadProgress {
  /** 已发送字节数。 */
  loaded: number;
  /** 浏览器可知的请求体总字节数。 */
  total: number;
  /** 0 到 100 的上传百分比；无法计算时为空。 */
  percent: number | null;
}

/** multipart 上传选项；XHR 仅用于提供浏览器原生上传进度。 */
export interface ApiUploadOptions {
  /** multipart 请求体。 */
  formData: FormData;
  /** 调用方控制取消请求的信号。 */
  signal?: AbortSignal;
  /** 上传进度回调。 */
  onProgress?: (progress: ApiUploadProgress) => void;
}

/** access token 提供函数；forceRefresh 为 true 时必须尝试 refresh token 轮换。 */
export type AccessTokenProvider = (
  forceRefresh?: boolean,
) => string | null | Promise<string | null>;

/** 浏览器请求发生网络连接失败时的全局通知函数。 */
export type NetworkErrorHandler = () => void;

/** 共享 HTTP API client。 */
export class ApiClient {
  private accessTokenProvider: AccessTokenProvider = () => null;
  private networkErrorHandler: NetworkErrorHandler = () => undefined;
  private runtimeAbortController = new AbortController();

  /** 注册 access token 提供函数；client 不负责持久化 token。 */
  setAccessTokenProvider(provider: AccessTokenProvider): void {
    this.accessTokenProvider = provider;
  }

  /** 注册网络连接失败通知；主动取消请求不会触发。 */
  setNetworkErrorHandler(handler: NetworkErrorHandler): void {
    this.networkErrorHandler = handler;
  }

  /** API 根地址切换前取消全部仍指向旧服务的 fetch 和上传请求。 */
  cancelRequestsForRuntimeChange(): void {
    this.runtimeAbortController.abort();
    this.runtimeAbortController = new AbortController();
  }

  /**
   * 向当前运行时配置的 WineStock 服务发送请求。
   * 网络失败、响应解析失败和非 2xx 状态分别转换为稳定错误类型。
   */
  async request<TResult>(path: string, options: ApiRequestOptions = {}): Promise<TResult> {
    const method = options.method ?? "GET";
    if (!path.startsWith("/")) {
      throw new ApiConfigurationError("API 请求路径必须以 / 开头");
    }
    if ((method === "GET" || method === "DELETE") && options.json !== undefined) {
      throw new ApiConfigurationError(`${method} 请求不能携带 JSON 请求体`);
    }

    const requestSignal = combineAbortSignals(options.signal, this.runtimeAbortController.signal);
    const url = buildRequestUrl(resolveApiBaseUrl(), path, options.query);
    const baseHeaders = new Headers(options.headers);
    if (!baseHeaders.has("accept")) {
      baseHeaders.set("accept", "application/json");
    }

    let body: string | undefined;
    if (options.json !== undefined) {
      baseHeaders.set("content-type", "application/json");
      body = JSON.stringify(options.json);
    }

    const usesManagedAccessToken =
      options.authenticated !== false && !baseHeaders.has("authorization");
    let accessToken = usesManagedAccessToken ? await this.accessTokenProvider(false) : null;

    for (let attempt = 0; attempt < 2; attempt += 1) {
      const headers = new Headers(baseHeaders);
      if (accessToken) {
        headers.set("authorization", `Bearer ${accessToken}`);
      }

      let response: Response;
      try {
        response = await fetch(url, {
          method,
          headers,
          body,
          signal: requestSignal,
          credentials: "omit",
        });
      } catch (error) {
        if (error instanceof DOMException && error.name === "AbortError") {
          throw error;
        }
        this.networkErrorHandler();
        throw new ApiNetworkError(error);
      }

      const payload =
        response.ok && options.responseType === "blob"
          ? await response.blob()
          : await readResponsePayload(response);
      if (response.ok) {
        return payload as TResult;
      }

      const errorBody: ApiErrorBody = isApiErrorResponse(payload)
        ? payload.error
        : {
            code: "http_error",
            message: `请求失败（HTTP ${response.status}）`,
            details: payload ?? null,
          };

      if (attempt === 0 && usesManagedAccessToken && errorBody.code === "invalid_access_token") {
        accessToken = await this.accessTokenProvider(true);
        if (accessToken) {
          continue;
        }
      }

      throw new ApiError(response.status, errorBody, url.toString());
    }

    throw new ApiResponseError(url.toString(), new Error("API 请求重试状态无效"));
  }

  /**
   * 上传 multipart 表单并报告真实上传进度。
   * 该入口复用受管理 access token 和一次强制 refresh，页面不直接读取会话状态。
   */
  async upload<TResult>(path: string, options: ApiUploadOptions): Promise<TResult> {
    if (!path.startsWith("/")) {
      throw new ApiConfigurationError("API 请求路径必须以 / 开头");
    }
    const requestSignal = combineAbortSignals(options.signal, this.runtimeAbortController.signal);
    const url = buildRequestUrl(resolveApiBaseUrl(), path, undefined);
    let accessToken = await this.accessTokenProvider(false);
    for (let attempt = 0; attempt < 2; attempt += 1) {
      const result = await sendMultipartRequest(
        url,
        { ...options, signal: requestSignal },
        accessToken,
        this.networkErrorHandler,
      );
      if (result.ok) {
        return result.payload as TResult;
      }
      const errorBody: ApiErrorBody = isApiErrorResponse(result.payload)
        ? result.payload.error
        : {
            code: "http_error",
            message: `请求失败（HTTP ${result.status}）`,
            details: result.payload ?? null,
          };
      if (attempt === 0 && errorBody.code === "invalid_access_token") {
        accessToken = await this.accessTokenProvider(true);
        if (accessToken) continue;
      }
      throw new ApiError(result.status, errorBody, url.toString());
    }
    throw new ApiResponseError(url.toString(), new Error("API 上传重试状态无效"));
  }
}

/** 默认共享 API client 实例。 */
export const apiClient = new ApiClient();

function combineAbortSignals(
  callerSignal: AbortSignal | undefined,
  runtimeSignal: AbortSignal,
): AbortSignal {
  return callerSignal ? AbortSignal.any([callerSignal, runtimeSignal]) : runtimeSignal;
}

/** 根据运行时根地址、API 路径和查询参数构造请求 URL，避免 token 被发送到外部绝对地址。 */
function buildRequestUrl(baseUrl: string, path: string, query: ApiQuery | undefined): URL {
  const url = new URL(path.replace(/^\/+/, ""), `${baseUrl.replace(/\/+$/, "")}/`);
  if (!query) {
    return url;
  }

  for (const [key, value] of Object.entries(query)) {
    if (value === null || value === undefined) {
      continue;
    }

    const values = Array.isArray(value) ? value : [value];
    for (const item of values) {
      url.searchParams.append(key, String(item));
    }
  }

  return url;
}

/** 读取 JSON、文本或空响应；声明为 JSON 却无法解析时明确报错。 */
async function readResponsePayload(response: Response): Promise<unknown> {
  if (response.status === 204) {
    return undefined;
  }

  const text = await response.text();
  if (!text) {
    return undefined;
  }

  const contentType = response.headers.get("content-type") ?? "";
  if (!contentType.includes("application/json") && !contentType.includes("+json")) {
    return text;
  }

  try {
    return JSON.parse(text) as unknown;
  } catch (error) {
    if (response.ok) {
      throw new ApiResponseError(response.url, error);
    }
    return text;
  }
}

interface MultipartResult {
  ok: boolean;
  status: number;
  payload: unknown;
}

function sendMultipartRequest(
  url: URL,
  options: ApiUploadOptions,
  accessToken: string | null,
  networkErrorHandler: NetworkErrorHandler,
): Promise<MultipartResult> {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    let abortedByCaller = false;
    const abort = () => {
      abortedByCaller = true;
      xhr.abort();
    };
    if (options.signal?.aborted) {
      reject(new DOMException("请求已取消", "AbortError"));
      return;
    }
    options.signal?.addEventListener("abort", abort, { once: true });
    xhr.open("POST", url);
    xhr.setRequestHeader("accept", "application/json");
    if (accessToken) xhr.setRequestHeader("authorization", `Bearer ${accessToken}`);
    xhr.upload.onprogress = (event) => {
      options.onProgress?.({
        loaded: event.loaded,
        total: event.lengthComputable ? event.total : 0,
        percent:
          event.lengthComputable && event.total > 0
            ? Math.min(100, Math.round((event.loaded / event.total) * 100))
            : null,
      });
    };
    xhr.onload = () => {
      options.signal?.removeEventListener("abort", abort);
      try {
        resolve({
          ok: xhr.status >= 200 && xhr.status < 300,
          status: xhr.status,
          payload: parseXhrPayload(xhr.responseText, xhr.getResponseHeader("content-type") ?? ""),
        });
      } catch (error) {
        reject(error);
      }
    };
    xhr.onerror = () => {
      options.signal?.removeEventListener("abort", abort);
      networkErrorHandler();
      reject(new ApiNetworkError(new Error("XMLHttpRequest network error")));
    };
    xhr.onabort = () => {
      options.signal?.removeEventListener("abort", abort);
      reject(
        abortedByCaller
          ? new DOMException("请求已取消", "AbortError")
          : new ApiNetworkError(new Error("XMLHttpRequest aborted")),
      );
    };
    xhr.send(options.formData);
  });
}

function parseXhrPayload(text: string, contentType: string): unknown {
  if (!text) return undefined;
  if (!contentType.includes("application/json") && !contentType.includes("+json")) return text;
  try {
    return JSON.parse(text) as unknown;
  } catch (error) {
    throw new ApiResponseError("multipart upload", error);
  }
}
