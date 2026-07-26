// 本文件拥有入库模板 DTO 和请求，属于 frontend HTTP 边界；模板只描述本次收货属性。
// DTO 通过 contract.ts 别名映射到生成 schema，导出名保持稳定。
import { apiClient } from "./client";
import type { ApiResponse, ApiSchema } from "./contract";
import type { TemplateCopyRequest } from "./itemAttributeTemplates";

export type InboundTemplateResponse = ApiResponse<ApiSchema<"InboundTemplateResponse">>;

/** 创建与整体更新共用完整写入模型，因此统一采用创建请求 schema。 */
export type InboundTemplateWriteRequest = ApiSchema<"InboundTemplateCreateRequest">;

export function listInboundTemplates(signal?: AbortSignal) {
  return apiClient.request<InboundTemplateResponse[]>("/api/inbound-templates", { signal });
}

export function getInboundTemplate(id: number, signal?: AbortSignal) {
  return apiClient.request<InboundTemplateResponse>(`/api/inbound-templates/${id}`, { signal });
}

export function createInboundTemplate(request: InboundTemplateWriteRequest) {
  return apiClient.request<InboundTemplateResponse>("/api/inbound-templates", {
    method: "POST",
    json: request,
  });
}

export function updateInboundTemplate(id: number, request: InboundTemplateWriteRequest) {
  return apiClient.request<InboundTemplateResponse>(`/api/inbound-templates/${id}`, {
    method: "PUT",
    json: request,
  });
}

export function copyInboundTemplate(id: number, request: TemplateCopyRequest) {
  return apiClient.request<InboundTemplateResponse>(`/api/inbound-templates/${id}/copy`, {
    method: "POST",
    json: request,
  });
}

export function deleteInboundTemplate(id: number) {
  return apiClient.request<void>(`/api/inbound-templates/${id}`, { method: "DELETE" });
}
