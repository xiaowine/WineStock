// 本文件拥有物品属性模板 DTO 和请求，属于 frontend HTTP 边界；模板只提供可选预设。
// DTO 通过 contract.ts 别名映射到生成 schema，导出名保持稳定。
import { apiClient } from "./client";
import type { ApiResponse, ApiSchema } from "./contract";

/** 物品模板字段支持的单位交互模式。 */
export type ItemAttributeUnitMode = ApiSchema<"ItemAttributeUnitMode">;

/** 由物品属性模板显式定义的单位规则；响应中经服务端归一化后各字段始终存在。 */
export type ItemAttributeUnitRule = ApiResponse<ApiSchema<"ItemAttributeUnitRule">>;

/** 带物品专属单位规则的模板字段响应。 */
export type ItemAttributeTemplateFieldResponse = ApiResponse<
  ApiSchema<"ItemAttributeTemplateFieldResponse">
>;

export type ItemAttributeTemplateResponse = ApiResponse<ApiSchema<"ItemAttributeTemplateResponse">>;

export type ItemAttributeTemplateDeletionResponse = ApiResponse<
  ApiSchema<"ItemAttributeTemplateDeleteResponse">
>;

/** 更新物品属性模板时整体提交的字段定义；生成 schema 名为 ItemAttributeTemplateFieldDef。 */
export type ItemAttributeTemplateFieldRequest = ApiSchema<"ItemAttributeTemplateFieldDef">;

/** 物品属性模板创建与整体保存请求。 */
export type ItemAttributeTemplateWriteRequest = ApiSchema<"ItemAttributeTemplateCreateRequest">;

/** 更新接口允许只提交发生变化的部分；字段存在时仍按完整数组替换。 */
export type ItemAttributeTemplateUpdateRequest = ApiSchema<"ItemAttributeTemplateUpdateRequest">;

export type TemplateCopyRequest = ApiSchema<"TemplateCopyRequest">;

export function listItemAttributeTemplates(signal?: AbortSignal) {
  return apiClient.request<ItemAttributeTemplateResponse[]>("/api/item-attribute-templates", {
    signal,
  });
}

export function getItemAttributeTemplate(id: number, signal?: AbortSignal) {
  return apiClient.request<ItemAttributeTemplateResponse>(`/api/item-attribute-templates/${id}`, {
    signal,
  });
}

export function createItemAttributeTemplate(request: ItemAttributeTemplateWriteRequest) {
  return apiClient.request<ItemAttributeTemplateResponse>("/api/item-attribute-templates", {
    method: "POST",
    json: request,
  });
}

/** 更新物品属性模板基础信息和完整字段配置。 */
export function updateItemAttributeTemplate(
  id: number,
  request: ItemAttributeTemplateUpdateRequest,
) {
  return apiClient.request<ItemAttributeTemplateResponse>(`/api/item-attribute-templates/${id}`, {
    method: "PUT",
    json: request,
  });
}

export function copyItemAttributeTemplate(id: number, request: TemplateCopyRequest) {
  return apiClient.request<ItemAttributeTemplateResponse>(
    `/api/item-attribute-templates/${id}/copy`,
    {
      method: "POST",
      json: request,
    },
  );
}

export function deleteItemAttributeTemplate(id: number) {
  return apiClient.request<ItemAttributeTemplateDeletionResponse>(
    `/api/item-attribute-templates/${id}`,
    { method: "DELETE" },
  );
}
