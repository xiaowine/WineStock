// 本文件拥有创建待审批出库单的 HTTP 契约；它不管理页面草稿或审批操作。
import { apiClient } from "./client";

/** 创建出库单的一条物品明细。 */
export interface OutboundItemRequest {
  /** 既有物品 ID。 */
  item_id: number;
  /** 申请出库数量，必须大于零。 */
  quantity: number;
  /** 指定扣减批次；为空时由审批按 FIFO 分配。 */
  batch_id?: number;
  /** 限制扣减范围的库位；为空时允许全部库位。 */
  location_id?: number;
}

/** 创建待审批出库单的请求。 */
export interface OutboundCreateRequest {
  /** 客户、部门或项目等出库去向。 */
  destination: string;
  /** 可选说明。 */
  notes?: string;
  /** 至少一条出库明细。 */
  items: OutboundItemRequest[];
}

/** 创建成功后的出库单摘要。 */
export interface OutboundCreateResponse {
  id: number;
  destination: string;
  status: "pending" | "approved";
  notes: string | null;
}

/** 创建待审批出库单；库存仅在后续审批通过时扣减。 */
export function createOutbound(request: OutboundCreateRequest) {
  return apiClient.request<OutboundCreateResponse>("/api/outbound", {
    method: "POST",
    json: request,
  });
}
