// 本文件拥有入库编辑所需的模板 DTO 和创建请求，并复用统一库位 API；它不维护页面草稿或审批状态。
// DTO 通过 contract.ts 别名映射到生成 schema，导出名保持稳定。
import { apiClient } from "./client";
import type { ApiResponse, ApiSchema } from "./contract";
export { listLocations } from "./locations";
export type { LocationResponse } from "./locations";
export type { InboundTemplateResponse } from "./inboundTemplates";
export type { TemplateFieldResponse, TemplateFieldType } from "./templateFields";

/** 创建入库单的单条物品明细。 */
export type InboundItemRequest = ApiSchema<"InboundItemRequest">;

/** 模板 file 字段保存的稳定服务端引用。 */
export type FileAttributeReference = ApiSchema<"FileAttributeReference">;

/** 创建入库单时采用的服务端处理方式。 */
export type InboundSubmissionMode = ApiSchema<"InboundSubmissionMode">;

/** 创建待审批单据或直接完成入库的请求。 */
export type InboundCreateRequest = ApiSchema<"InboundCreateRequest">;

/** 创建接口返回的完整入库单响应；创建场景页面只读取摘要字段。 */
export type InboundResponse = ApiResponse<ApiSchema<"InboundResponse">>;

/** 创建待审批单据或直接完成入库；响应明确返回实际采用的提交方式。 */
export function createInbound(request: InboundCreateRequest) {
  return apiClient.request<InboundResponse>("/api/inbound", {
    method: "POST",
    json: request,
  });
}
