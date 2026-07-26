// 本文件拥有创建待审批出库单的 HTTP 契约；它不管理页面草稿或审批操作。
// DTO 通过 contract.ts 别名映射到生成 schema，导出名保持稳定。
import { apiClient } from "./client";
import type { ApiResponse, ApiSchema } from "./contract";

/** 创建出库单的一条物品明细。 */
export type OutboundItemRequest = ApiSchema<"OutboundItemRequest">;

/** 创建待审批出库单的请求。 */
export type OutboundCreateRequest = ApiSchema<"OutboundCreateRequest">;

/** 创建接口返回的完整出库单响应；创建场景页面只读取摘要字段。 */
export type OutboundCreateResponse = ApiResponse<ApiSchema<"OutboundResponse">>;

/** 创建待审批出库单；库存仅在后续审批通过时扣减。 */
export function createOutbound(request: OutboundCreateRequest) {
  return apiClient.request<OutboundCreateResponse>("/api/outbound", {
    method: "POST",
    json: request,
  });
}
