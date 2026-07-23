// 本文件拥有 frontend 鉴权 HTTP 契约和请求函数；它不保存会话或决定登录后的页面导航。
import { apiClient } from "./client";
import { resolveApiBaseUrl, type ApiClientKind } from "./runtime-config";

const bootstrapStatusCache = new Map<string, AuthBootstrapStatus>();

/** 用户名密码登录请求。 */
export interface AuthLoginRequest {
  /** 登录用户名。 */
  username: string;
  /** 明文密码，只允许发送到登录接口。 */
  password: string;
  /** refresh token 来源设备名称。 */
  device_name: string;
  /** 登录客户端类型。 */
  client_kind: ApiClientKind;
  /** 客户端版本号。 */
  version: string;
}

/** 注册用户请求；首个用户允许在未登录状态下创建。 */
export interface AuthRegisterRequest {
  /** 登录用户名。 */
  username: string;
  /** 明文密码，只允许发送到注册接口。 */
  password: string;
}

/** 未鉴权认证入口用于选择首用户注册或普通登录的状态。 */
export interface AuthBootstrapStatus {
  requires_initial_user: boolean;
}

/** 查询当前服务是否仍需要创建首个用户。 */
export function getAuthBootstrapStatus(): Promise<AuthBootstrapStatus> {
  let apiBaseUrl: string;
  try {
    apiBaseUrl = resolveApiBaseUrl();
  } catch (error) {
    return Promise.reject(error);
  }
  const cached = bootstrapStatusCache.get(apiBaseUrl);
  if (cached) return Promise.resolve(cached);
  return apiClient
    .request<AuthBootstrapStatus>("/api/auth/bootstrap-status", { authenticated: false })
    .then((status) => {
      if (typeof status.requires_initial_user !== "boolean") {
        throw new Error("bootstrap status response is invalid");
      }
      bootstrapStatusCache.set(apiBaseUrl, status);
      return status;
    });
}

/** 运行地址切换后清除旧服务的初始化判断；首用户注册成功后可直接标记已初始化。 */
export function resetAuthBootstrapStatus(): void {
  bootstrapStatusCache.clear();
}

export function markAuthBootstrapInitialized(): void {
  try {
    bootstrapStatusCache.set(resolveApiBaseUrl(), { requires_initial_user: false });
  } catch {
    // 未配置服务地址时没有可写入的缓存键。
  }
}

/** refresh token 换取新 token 包的请求。 */
export interface AuthRefreshRequest {
  /** 当前持有的 opaque refresh token。 */
  refresh_token: string;
}

/** 吊销 refresh token 的登出请求。 */
export interface AuthLogoutRequest {
  /** 当前持有的 opaque refresh token。 */
  refresh_token: string;
}

/** 当前用户修改自己密码的请求。 */
export interface AuthPasswordChangeRequest {
  /** 当前明文密码，用于确认操作者仍掌握原凭据。 */
  current_password: string;
  /** 新明文密码，服务端要求 8 至 128 个字符。 */
  new_password: string;
}

/** 登录响应中的当前用户摘要。 */
export interface AuthUserResponse {
  /** 字符串形式的用户 ID。 */
  id: string;
  /** 登录用户名。 */
  username: string;
  /** 当前用户权限代码。 */
  permissions: string[];
  /** 是否必须先完成临时密码修改。 */
  password_change_required: boolean;
}

/** 登录成功后的 token 包。 */
export interface AuthTokenResponse {
  /** 用于 Bearer 请求头的 JWT access token。 */
  access_token: string;
  /** 只在登录或刷新响应中返回一次的 opaque refresh token。 */
  refresh_token: string;
  /** access token 剩余有效期，单位秒。 */
  expires_in: number;
  /** 当前登录用户摘要。 */
  user: AuthUserResponse;
}

/** 调用用户名密码登录接口。 */
export function login(request: AuthLoginRequest): Promise<AuthTokenResponse> {
  return apiClient.request<AuthTokenResponse>("/api/auth/login", {
    method: "POST",
    json: request,
    authenticated: false,
  });
}

/** 未携带 access token 调用注册接口，只用于初始化空服务的首个用户。 */
export function registerInitialUser(request: AuthRegisterRequest): Promise<AuthUserResponse> {
  return apiClient.request<AuthUserResponse>("/api/auth/register", {
    method: "POST",
    json: request,
    authenticated: false,
  });
}

/** 轮换 refresh token 并获取新的 access token。 */
export function refresh(request: AuthRefreshRequest): Promise<AuthTokenResponse> {
  return apiClient.request<AuthTokenResponse>("/api/auth/refresh", {
    method: "POST",
    json: request,
    authenticated: false,
  });
}

/** 吊销当前 refresh token；接口不依赖 Bearer access token。 */
export function logout(request: AuthLogoutRequest): Promise<void> {
  return apiClient.request<void>("/api/auth/logout", {
    method: "POST",
    json: request,
    authenticated: false,
  });
}

/** 当前登录用户修改自己的密码；成功响应为 204。 */
export function changeOwnPassword(request: AuthPasswordChangeRequest): Promise<void> {
  return apiClient.request<void>("/api/auth/me/password", {
    method: "POST",
    json: request,
  });
}
