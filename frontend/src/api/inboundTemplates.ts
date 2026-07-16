// 本文件拥有入库模板 DTO 和请求，属于 frontend HTTP 边界；模板只描述本次收货属性。
import { apiClient } from "./client";
import type { TemplateFieldRequest, TemplateFieldResponse } from "./templateFields";
import type { TemplateCopyRequest } from "./itemAttributeTemplates";

export interface InboundTemplateResponse {
  id: number;
  name: string;
  description: string | null;
  fields: TemplateFieldResponse[];
  created_at: string;
  updated_at: string;
}

export interface InboundTemplateWriteRequest {
  name: string;
  description: string | null;
  fields: TemplateFieldRequest[];
}

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
