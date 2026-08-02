// 本文件拥有 frontend 鉴权 HTTP 契约和请求函数；它不保存会话或决定登录后的页面导航。
// DTO 通过 contract.ts 别名映射到生成 schema，导出名保持稳定。
import { apiClient } from "./client";
import type { ApiResponse, ApiSchema } from "./contract";
import { resolveApiBaseUrl } from "./runtime-config";

const bootstrapStatusCache = new Map<string, AuthBootstrapStatus>();

/** 用户名密码登录请求。 */
export type AuthLoginRequest = ApiSchema<"AuthLoginRequest">;

/** 注册用户请求；首个用户允许在未登录状态下创建。 */
export type AuthRegisterRequest = ApiSchema<"AuthRegisterRequest">;

/** 未鉴权认证入口用于选择首用户注册或普通登录的状态。 */
export type AuthBootstrapStatus = ApiResponse<ApiSchema<"AuthBootstrapStatus">>;

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

/** self-hosted 本机静默会话换取请求；凭据来自壳内可信通道。 */
export type AuthLocalSessionRequest = ApiSchema<"AuthLocalSessionRequest">;

/** 本机静默会话状态；切换 server-mode 前判断是否需要先设置真实密码。 */
export type AuthLocalSessionStatus = ApiResponse<ApiSchema<"AuthLocalSessionStatus">>;

/** 用壳内下发的换取凭据建立本机静默会话；返回与登录相同的 token 包。 */
export function exchangeLocalSession(request: AuthLocalSessionRequest): Promise<AuthTokenResponse> {
  return apiClient.request<AuthTokenResponse>("/api/auth/local-session", {
    method: "POST",
    json: request,
    authenticated: false,
  });
}

/** 查询本机静默会话状态；仅标记用户密码仍为占位时 password_placeholder 为 true。 */
export function getLocalSessionStatus(): Promise<AuthLocalSessionStatus> {
  return apiClient.request<AuthLocalSessionStatus>("/api/auth/local-session/status");
}

/** refresh token 换取新 token 包的请求。 */
export type AuthRefreshRequest = ApiSchema<"AuthRefreshRequest">;

/** 吊销 refresh token 的登出请求。 */
export type AuthLogoutRequest = ApiSchema<"AuthLogoutRequest">;

/** 当前用户修改自己密码的请求；生成 schema 名为 UserPasswordChangeRequest。 */
export type AuthPasswordChangeRequest = ApiSchema<"UserPasswordChangeRequest">;

/** 登录响应中的当前用户摘要。 */
export type AuthUserResponse = ApiResponse<ApiSchema<"AuthUserResponse">>;

/** 登录成功后的 token 包。 */
export type AuthTokenResponse = ApiResponse<ApiSchema<"AuthTokenResponse">>;

/** 调用用户名密码登录接口。 */
export function login(request: AuthLoginRequest): Promise<AuthTokenResponse> {
  return apiClient.request<AuthTokenResponse>("/api/auth/login", {
    method: "POST",
    json: request,
    authenticated: false,
  });
}

/** 查询当前会话用户的最新身份和权限快照。 */
export function getCurrentUser(): Promise<AuthUserResponse> {
  return apiClient.request<AuthUserResponse>("/api/auth/me");
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
