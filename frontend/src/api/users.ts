// 本文件拥有 frontend 用户管理 HTTP 契约和请求函数；它不保存会话或决定页面权限展示。
// DTO 通过 contract.ts 别名映射到生成 schema，导出名保持稳定；查询参数模型仍手写。
import type { AuthRegisterRequest, AuthUserResponse } from "./auth";
import { apiClient } from "./client";
import type { ApiResponse, ApiSchema } from "./contract";
import type { PaginatedResponse } from "./pagination";

/** 用户账号状态。 */
export type UserStatus = ApiSchema<"UserStatus">;

/** 用户管理接口返回的用户详情。 */
export type UserAdminResponse = ApiResponse<ApiSchema<"UserAdminResponse">>;

/** 用户列表查询参数。 */
export interface UserListQuery {
  /** 页码，从 1 开始。 */
  page: number;
  /** 每页数量，服务端最大允许 200。 */
  page_size: number;
  /** 可选用户名搜索文本。 */
  search?: string;
  /** 可选账号状态筛选。 */
  status?: UserStatus;
}

/** 权限定义。 */
export type PermissionResponse = ApiResponse<ApiSchema<"PermissionResponse">>;

/** 用户状态更新请求。 */
export type UserStatusUpdateRequest = ApiSchema<"UserStatusUpdateRequest">;

/** 修改用户登录用户名请求。 */
export type UserUsernameUpdateRequest = ApiSchema<"UserUsernameUpdateRequest">;

/** 用户权限整体替换请求。 */
export type UserPermissionsUpdateRequest = ApiSchema<"UserPermissionsUpdateRequest">;

/** 管理员设置临时密码请求。 */
export type UserPasswordResetRequest = ApiSchema<"UserPasswordResetRequest">;

/** 分页查询用户。 */
export function listUsers(
  query: UserListQuery,
  signal?: AbortSignal,
): Promise<PaginatedResponse<UserAdminResponse>> {
  return apiClient.request<PaginatedResponse<UserAdminResponse>>("/api/users", {
    query: { ...query },
    signal,
  });
}

/** 使用当前会话权限创建后续用户；新用户默认没有权限。 */
export function registerUser(request: AuthRegisterRequest): Promise<AuthUserResponse> {
  return apiClient.request<AuthUserResponse>("/api/auth/register", {
    method: "POST",
    json: request,
  });
}

/** 更新目标用户状态。 */
export function updateUserStatus(
  userId: number,
  request: UserStatusUpdateRequest,
): Promise<UserAdminResponse> {
  return apiClient.request<UserAdminResponse>(`/api/users/${userId}/status`, {
    method: "PATCH",
    json: request,
  });
}

/** 修改目标用户登录用户名；用户 ID、权限和现有会话保持不变。 */
export function updateUserUsername(
  userId: number,
  request: UserUsernameUpdateRequest,
): Promise<UserAdminResponse> {
  return apiClient.request<UserAdminResponse>(`/api/users/${userId}/username`, {
    method: "PATCH",
    json: request,
  });
}

/** 软删除目标用户并使其现有会话失效；成功响应为 204。 */
export function deleteUser(userId: number): Promise<void> {
  return apiClient.request<void>(`/api/users/${userId}`, {
    method: "DELETE",
  });
}

/** 整体替换目标用户权限。 */
export function updateUserPermissions(
  userId: number,
  request: UserPermissionsUpdateRequest,
): Promise<UserAdminResponse> {
  return apiClient.request<UserAdminResponse>(`/api/users/${userId}/permissions`, {
    method: "PUT",
    json: request,
  });
}

/** 为目标用户设置临时密码；成功响应为 204。 */
export function resetUserPassword(
  userId: number,
  request: UserPasswordResetRequest,
): Promise<void> {
  return apiClient.request<void>(`/api/users/${userId}/password`, {
    method: "POST",
    json: request,
  });
}

/** 查询全部权限定义。 */
export function listPermissions(signal?: AbortSignal): Promise<PermissionResponse[]> {
  return apiClient.request<PermissionResponse[]>("/api/permissions", { signal });
}
