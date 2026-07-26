// 本文件拥有 frontend 审计日志 HTTP 契约和查询函数；它不解释历史详情 JSON 的业务语义。
// DTO 通过 contract.ts 别名映射到生成 schema；查询参数与本地 JSON 值类型仍手写。
import { apiClient } from "./client";
import type { ApiResponse, ApiSchema } from "./contract";
import type { PaginatedResponse } from "./pagination";

/** JSON 原始值。 */
export type JsonPrimitive = string | number | boolean | null;

/** 审计详情允许返回的递归 JSON 值。 */
export type JsonValue = JsonPrimitive | JsonValue[] | { [key: string]: JsonValue };

/** 审计事件响应；`details` 保持未知 JSON 值，由页面兼容历史结构。 */
export type EventLogResponse = ApiResponse<ApiSchema<"EventLogResponse">>;

/** 审计事件分页查询。 */
export interface EventListQuery {
  page: number;
  page_size: number;
  entity_type?: string;
  entity_id?: number;
  action?: string;
  user_id?: number;
  date_from?: string;
  date_to?: string;
}

/** 按服务端固定时间倒序分页查询审计事件。 */
export function listEvents(
  query: EventListQuery,
  signal?: AbortSignal,
): Promise<PaginatedResponse<EventLogResponse>> {
  return apiClient.request<PaginatedResponse<EventLogResponse>>("/api/events", {
    query: { ...query },
    signal,
  });
}
