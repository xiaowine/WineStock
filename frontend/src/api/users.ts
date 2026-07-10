// 本文件拥有 frontend 用户管理 HTTP 契约和请求函数；它不保存会话或决定页面权限展示。
import type { AuthRegisterRequest, AuthUserResponse } from './auth'
import { apiClient } from './client'

/** 用户账号状态。 */
export type UserStatus = 'active' | 'disabled'

/** 用户管理接口返回的用户详情。 */
export interface UserAdminResponse {
  /** 数字形式的用户 ID。 */
  id: number
  /** 登录用户名。 */
  username: string
  /** 当前账号状态。 */
  status: UserStatus
  /** 用户直接拥有的权限代码。 */
  permissions: string[]
  /** 是否必须在下次登录后修改临时密码。 */
  password_change_required: boolean
  /** 创建时间，使用服务端 UTC 字符串。 */
  created_at: string
  /** 最近更新时间，使用服务端 UTC 字符串。 */
  updated_at: string
}

/** 通用分页响应。 */
export interface PaginatedResponse<TItem> {
  /** 当前页数据。 */
  items: TItem[]
  /** 满足条件的总记录数。 */
  total: number
  /** 当前页码，从 1 开始。 */
  page: number
  /** 当前每页数量。 */
  page_size: number
  /** 总页数；无数据时为 0。 */
  total_pages: number
}

/** 用户列表查询参数。 */
export interface UserListQuery {
  /** 页码，从 1 开始。 */
  page: number
  /** 每页数量，服务端最大允许 200。 */
  page_size: number
  /** 可选用户名搜索文本。 */
  search?: string
  /** 可选账号状态筛选。 */
  status?: UserStatus
}

/** 权限定义。 */
export interface PermissionResponse {
  /** 稳定权限代码。 */
  code: string
  /** 面向管理者的权限说明。 */
  description: string | null
}

/** 用户状态更新请求。 */
export interface UserStatusUpdateRequest {
  /** 目标账号状态。 */
  status: UserStatus
}

/** 用户权限整体替换请求。 */
export interface UserPermissionsUpdateRequest {
  /** 替换后的完整权限代码列表。 */
  permissions: string[]
}

/** 管理员设置临时密码请求。 */
export interface UserPasswordResetRequest {
  /** 临时明文密码，只允许发送到本接口。 */
  password: string
}

/** 分页查询用户。 */
export function listUsers(
  query: UserListQuery,
  signal?: AbortSignal,
): Promise<PaginatedResponse<UserAdminResponse>> {
  return apiClient.request<PaginatedResponse<UserAdminResponse>>('/api/users', {
    query: { ...query },
    signal,
  })
}

/** 使用当前会话权限创建后续用户；新用户默认没有权限。 */
export function registerUser(request: AuthRegisterRequest): Promise<AuthUserResponse> {
  return apiClient.request<AuthUserResponse>('/api/auth/register', {
    method: 'POST',
    json: request,
  })
}

/** 更新目标用户状态。 */
export function updateUserStatus(
  userId: number,
  request: UserStatusUpdateRequest,
): Promise<UserAdminResponse> {
  return apiClient.request<UserAdminResponse>(`/api/users/${userId}/status`, {
    method: 'PATCH',
    json: request,
  })
}

/** 整体替换目标用户权限。 */
export function updateUserPermissions(
  userId: number,
  request: UserPermissionsUpdateRequest,
): Promise<UserAdminResponse> {
  return apiClient.request<UserAdminResponse>(`/api/users/${userId}/permissions`, {
    method: 'PUT',
    json: request,
  })
}

/** 为目标用户设置临时密码；成功响应为 204。 */
export function resetUserPassword(
  userId: number,
  request: UserPasswordResetRequest,
): Promise<void> {
  return apiClient.request<void>(`/api/users/${userId}/password`, {
    method: 'POST',
    json: request,
  })
}

/** 查询全部权限定义。 */
export function listPermissions(signal?: AbortSignal): Promise<PermissionResponse[]> {
  return apiClient.request<PermissionResponse[]>('/api/permissions', { signal })
}
