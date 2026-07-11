// 本文件拥有 frontend 通用 HTTP 请求执行，属于 API 边界；它不实现具体业务接口或 token 刷新策略。
import {
  ApiConfigurationError,
  ApiError,
  ApiNetworkError,
  ApiResponseError,
  isApiErrorResponse,
  type ApiErrorBody,
} from './errors'
import { resolveApiBaseUrl } from './runtime-config'

/** API 查询参数基础值。 */
export type ApiQueryPrimitive = string | number | boolean

/** API 查询参数值；数组会重复写入同一个 query key。 */
export type ApiQueryValue = ApiQueryPrimitive | readonly ApiQueryPrimitive[] | null | undefined

/** API 查询参数集合。 */
export type ApiQuery = Readonly<Record<string, ApiQueryValue>>

/** API client 支持的 HTTP 方法。 */
export type ApiMethod = 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE'

/** 单次 API 请求选项。 */
export interface ApiRequestOptions {
  /** HTTP 方法，默认使用 `GET`。 */
  method?: ApiMethod
  /** URL 查询参数。 */
  query?: ApiQuery
  /** 需要 JSON 序列化的请求体。 */
  json?: unknown
  /** 是否尝试附加 access token，默认启用。 */
  authenticated?: boolean
  /** 调用方补充的请求头。 */
  headers?: HeadersInit
  /** 调用方控制取消请求的信号。 */
  signal?: AbortSignal
}

/** access token 提供函数；forceRefresh 为 true 时必须尝试 refresh token 轮换。 */
export type AccessTokenProvider = (forceRefresh?: boolean) => string | null | Promise<string | null>

/** 浏览器请求发生网络连接失败时的全局通知函数。 */
export type NetworkErrorHandler = () => void

/** 共享 HTTP API client。 */
export class ApiClient {
  private accessTokenProvider: AccessTokenProvider = () => null
  private networkErrorHandler: NetworkErrorHandler = () => undefined

  /** 注册 access token 提供函数；client 不负责持久化 token。 */
  setAccessTokenProvider(provider: AccessTokenProvider): void {
    this.accessTokenProvider = provider
  }

  /** 注册网络连接失败通知；主动取消请求不会触发。 */
  setNetworkErrorHandler(handler: NetworkErrorHandler): void {
    this.networkErrorHandler = handler
  }

  /**
   * 向当前运行时配置的 WineStock 服务发送请求。
   * 网络失败、响应解析失败和非 2xx 状态分别转换为稳定错误类型。
   */
  async request<TResult>(path: string, options: ApiRequestOptions = {}): Promise<TResult> {
    const method = options.method ?? 'GET'
    if (!path.startsWith('/')) {
      throw new ApiConfigurationError('API 请求路径必须以 / 开头')
    }
    if ((method === 'GET' || method === 'DELETE') && options.json !== undefined) {
      throw new ApiConfigurationError(`${method} 请求不能携带 JSON 请求体`)
    }

    const url = buildRequestUrl(resolveApiBaseUrl(), path, options.query)
    const baseHeaders = new Headers(options.headers)
    if (!baseHeaders.has('accept')) {
      baseHeaders.set('accept', 'application/json')
    }

    let body: string | undefined
    if (options.json !== undefined) {
      baseHeaders.set('content-type', 'application/json')
      body = JSON.stringify(options.json)
    }

    const usesManagedAccessToken =
      options.authenticated !== false && !baseHeaders.has('authorization')
    let accessToken = usesManagedAccessToken ? await this.accessTokenProvider(false) : null

    for (let attempt = 0; attempt < 2; attempt += 1) {
      const headers = new Headers(baseHeaders)
      if (accessToken) {
        headers.set('authorization', `Bearer ${accessToken}`)
      }

      let response: Response
      try {
        response = await fetch(url, {
          method,
          headers,
          body,
          signal: options.signal,
          credentials: 'omit',
        })
      } catch (error) {
        if (error instanceof DOMException && error.name === 'AbortError') {
          throw error
        }
        this.networkErrorHandler()
        throw new ApiNetworkError(error)
      }

      const payload = await readResponsePayload(response)
      if (response.ok) {
        return payload as TResult
      }

      const errorBody: ApiErrorBody = isApiErrorResponse(payload)
        ? payload.error
        : {
            code: 'http_error',
            message: `请求失败（HTTP ${response.status}）`,
            details: payload ?? null,
          }

      if (
        attempt === 0 &&
        usesManagedAccessToken &&
        errorBody.code === 'invalid_access_token'
      ) {
        accessToken = await this.accessTokenProvider(true)
        if (accessToken) {
          continue
        }
      }

      throw new ApiError(response.status, errorBody, url.toString())
    }

    throw new ApiResponseError(url.toString(), new Error('API 请求重试状态无效'))
  }
}

/** 默认共享 API client 实例。 */
export const apiClient = new ApiClient()

/** 根据运行时根地址、API 路径和查询参数构造请求 URL，避免 token 被发送到外部绝对地址。 */
function buildRequestUrl(baseUrl: string, path: string, query: ApiQuery | undefined): URL {
  const url = new URL(path.replace(/^\/+/, ''), `${baseUrl.replace(/\/+$/, '')}/`)
  if (!query) {
    return url
  }

  for (const [key, value] of Object.entries(query)) {
    if (value === null || value === undefined) {
      continue
    }

    const values = Array.isArray(value) ? value : [value]
    for (const item of values) {
      url.searchParams.append(key, String(item))
    }
  }

  return url
}

/** 读取 JSON、文本或空响应；声明为 JSON 却无法解析时明确报错。 */
async function readResponsePayload(response: Response): Promise<unknown> {
  if (response.status === 204) {
    return undefined
  }

  const text = await response.text()
  if (!text) {
    return undefined
  }

  const contentType = response.headers.get('content-type') ?? ''
  if (!contentType.includes('application/json') && !contentType.includes('+json')) {
    return text
  }

  try {
    return JSON.parse(text) as unknown
  } catch (error) {
    if (response.ok) {
      throw new ApiResponseError(response.url, error)
    }
    return text
  }
}
