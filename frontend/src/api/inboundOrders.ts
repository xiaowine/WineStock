// 本文件拥有入库单列表与详情 HTTP 契约，不管理页面筛选、分页或审批操作。
import { apiClient } from "./client";
import type { PaginatedResponse } from "./pagination";

export type InboundOrderStatus = "pending" | "approved" | "rejected";

export interface InboundOrderItemResponse {
  id: number;
  order_id: number;
  item_id: number;
  item_name: string;
  item_sku: string;
  item_unit: string;
  item_image_file_id: number;
  quantity: number;
  unit_price: number;
  location_id: number;
  location_name: string;
  batch_no: string | null;
  expires_at: string | null;
  inbound_template_id: number | null;
  ext_attributes: Record<string, unknown> | null;
  created_at: string;
}

export interface InboundOrderResponse {
  id: number;
  source: string;
  status: InboundOrderStatus;
  notes: string | null;
  created_by_user_id: number | null;
  approved_by_user_id: number | null;
  rejected_by_user_id: number | null;
  created_at: string;
  updated_at: string;
  approved_at: string | null;
  rejected_at: string | null;
  items: InboundOrderItemResponse[];
}

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
