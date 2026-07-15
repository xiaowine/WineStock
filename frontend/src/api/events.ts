// 本文件拥有 frontend 审计日志 HTTP 契约和查询函数；它不解释历史详情 JSON 的业务语义。
import { apiClient } from './client'
import type { PaginatedResponse } from './pagination'

/** JSON 原始值。 */
export type JsonPrimitive = string | number | boolean | null

/** 审计详情允许返回的递归 JSON 值。 */
export type JsonValue = JsonPrimitive | JsonValue[] | { [key: string]: JsonValue }

/** 审计事件响应。 */
export interface EventLogResponse {
  id: number
  timestamp: string
  user_id: number | null
  username: string | null
  entity_type: string
  entity_id: number | null
  action: string
  details: unknown
}

/** 审计事件分页查询。 */
export interface EventListQuery {
  page: number
  page_size: number
  entity_type?: string
  entity_id?: number
  action?: string
  user_id?: number
  date_from?: string
  date_to?: string
}

/** 按服务端固定时间倒序分页查询审计事件。 */
export function listEvents(
  query: EventListQuery,
  signal?: AbortSignal,
): Promise<PaginatedResponse<EventLogResponse>> {
  return apiClient.request<PaginatedResponse<EventLogResponse>>('/api/events', {
    query: { ...query },
    signal,
  })
}
