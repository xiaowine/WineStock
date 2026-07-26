// 本文件拥有出库单列表与详情 HTTP 契约，不管理页面筛选、分页或审批操作。
// DTO 通过 contract.ts 别名映射到生成 schema，导出名保持稳定；查询参数模型仍手写。
import { apiClient } from "./client";
import type { ApiResponse, ApiSchema } from "./contract";
import type { PaginatedResponse } from "./pagination";

/** 单据状态；生成 schema 名为 OrderStatus。 */
export type OutboundOrderStatus = ApiSchema<"OrderStatus">;

/** 出库单明细行；生成 schema 名为 OutboundItemResponse。 */
export type OutboundOrderItemResponse = ApiResponse<ApiSchema<"OutboundItemResponse">>;

/** 出库单详情；生成 schema 名为 OutboundResponse。 */
export type OutboundOrderResponse = ApiResponse<ApiSchema<"OutboundResponse">>;
export interface OutboundOrderListQuery {
  page: number;
  page_size: number;
  search?: string;
  status?: OutboundOrderStatus;
  date_from?: string;
  date_to?: string;
}
export const listOutboundOrders = (query: OutboundOrderListQuery, signal?: AbortSignal) =>
  apiClient.request<PaginatedResponse<OutboundOrderResponse>>("/api/outbound", {
    query: { ...query },
    signal,
  });
export const getOutboundOrder = (id: number, signal?: AbortSignal) =>
  apiClient.request<OutboundOrderResponse>(`/api/outbound/${id}`, { signal });
