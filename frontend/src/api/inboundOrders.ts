// 本文件拥有入库单列表与详情 HTTP 契约，不管理页面筛选、分页或审批操作。
// DTO 通过 contract.ts 别名映射到生成 schema，导出名保持稳定；查询参数模型仍手写。
import { apiClient } from "./client";
import type { ApiResponse, ApiSchema } from "./contract";
import type { PaginatedResponse } from "./pagination";

/** 单据状态；生成 schema 名为 OrderStatus。 */
export type InboundOrderStatus = ApiSchema<"OrderStatus">;

/** 入库单明细行；生成 schema 名为 InboundItemResponse。 */
export type InboundOrderItemResponse = ApiResponse<ApiSchema<"InboundItemResponse">>;

/** 入库单详情；生成 schema 名为 InboundResponse。 */
export type InboundOrderResponse = ApiResponse<ApiSchema<"InboundResponse">>;

export interface InboundOrderListQuery {
  page: number;
  page_size: number;
  search?: string;
  status?: InboundOrderStatus;
  date_from?: string;
  date_to?: string;
}

export function listInboundOrders(query: InboundOrderListQuery, signal?: AbortSignal) {
  return apiClient.request<PaginatedResponse<InboundOrderResponse>>("/api/inbound", {
    query: { ...query },
    signal,
  });
}

export function getInboundOrder(id: number, signal?: AbortSignal) {
  return apiClient.request<InboundOrderResponse>(`/api/inbound/${id}`, { signal });
}
