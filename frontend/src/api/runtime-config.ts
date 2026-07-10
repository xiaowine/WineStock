// 本文件拥有 frontend API 地址和客户端元数据解析，属于运行时配置边界；它不启动或发现 Axum 服务。
import { ApiConfigurationError } from './errors'

/** 登录请求允许的客户端类型。 */
export type ApiClientKind = 'desktop' | 'android' | 'web'

/** 平台 shell 可在前端应用挂载前注入的运行时配置。 */
export interface FrontendRuntimeConfig {
  /** 可访问的 WineStock HTTP 服务根地址，不能使用 `0.0.0.0`。 */
  apiBaseUrl?: string
  /** 当前平台写入登录设备记录的客户端类型。 */
  clientKind?: ApiClientKind
  /** 当前设备的可识别名称。 */
  deviceName?: string
  /** 当前客户端版本号。 */
  appVersion?: string
}

/** 登录接口需要的客户端来源元数据。 */
export interface ApiClientMetadata {
  /** 当前平台客户端类型。 */
  clientKind: ApiClientKind
  /** 当前设备名称。 */
  deviceName: string
  /** 当前客户端版本号。 */
  appVersion: string
}

/**
 * 解析 API 根地址。
 * 平台注入值优先于 Vite 环境变量；缺失时明确失败，避免静默连接错误服务。
 */
export function resolveApiBaseUrl(): string {
  const configured = firstNonBlank(
    readInjectedConfig()?.apiBaseUrl,
    import.meta.env.VITE_API_BASE_URL,
  )

  if (!configured) {
    throw new ApiConfigurationError(
      '未配置 WineStock 服务地址，请由平台注入 apiBaseUrl 或设置 VITE_API_BASE_URL',
    )
  }

  let url: URL
  try {
    url = new URL(configured)
  } catch (error) {
    throw new ApiConfigurationError(`WineStock 服务地址无效：${String(error)}`)
  }

  if (url.protocol !== 'http:' && url.protocol !== 'https:') {
    throw new ApiConfigurationError('WineStock 服务地址必须使用 http 或 https')
  }
  if (url.hostname === '0.0.0.0') {
    throw new ApiConfigurationError('0.0.0.0 只能用于服务绑定，不能作为前端访问地址')
  }
  if (url.username || url.password || url.search || url.hash) {
    throw new ApiConfigurationError('WineStock 服务地址不能包含凭据、查询参数或 hash')
  }

  return url.toString().replace(/\/$/, '')
}

/** 解析登录请求使用的平台、设备名称和版本号。 */
export function resolveApiClientMetadata(): ApiClientMetadata {
  const injected = readInjectedConfig()
  const clientKind = normalizeClientKind(
    firstNonBlank(injected?.clientKind, import.meta.env.VITE_CLIENT_KIND),
  )

  return {
    clientKind,
    deviceName: firstNonBlank(injected?.deviceName, import.meta.env.VITE_DEVICE_NAME) ?? 'WineStock Web',
    appVersion: firstNonBlank(injected?.appVersion, import.meta.env.VITE_APP_VERSION) ?? 'development',
  }
}

function readInjectedConfig(): FrontendRuntimeConfig | undefined {
  return typeof window === 'undefined' ? undefined : window.__WINESTOCK_RUNTIME_CONFIG__
}

function normalizeClientKind(value: string | undefined): ApiClientKind {
  return value === 'desktop' || value === 'android' || value === 'web' ? value : 'web'
}

function firstNonBlank(...values: Array<string | undefined>): string | undefined {
  return values.map((value) => value?.trim()).find((value) => value)
}
