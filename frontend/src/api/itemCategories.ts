// 本文件拥有物品分类 DTO 和请求，属于 frontend HTTP 边界；分类不承担属性模板职责。
// DTO 通过 contract.ts 别名映射到生成 schema，导出名保持稳定。
import { apiClient } from "./client";
import type { ApiResponse, ApiSchema } from "./contract";

export type ItemCategoryResponse = ApiResponse<ApiSchema<"ItemCategoryResponse">>;

export type ItemCategoryDeletionResponse = ApiResponse<ApiSchema<"ItemCategoryDeleteResponse">>;

/** 创建与整体更新共用完整写入模型，因此统一采用创建请求 schema。 */
export type ItemCategoryWriteRequest = ApiSchema<"ItemCategoryCreateRequest">;

export function listItemCategories(signal?: AbortSignal) {
  return apiClient.request<ItemCategoryResponse[]>("/api/item-categories", { signal });
}

export function getItemCategory(id: number, signal?: AbortSignal) {
  return apiClient.request<ItemCategoryResponse>(`/api/item-categories/${id}`, { signal });
}

export function createItemCategory(request: ItemCategoryWriteRequest) {
  return apiClient.request<ItemCategoryResponse>("/api/item-categories", {
    method: "POST",
    json: request,
  });
}

export function updateItemCategory(id: number, request: ItemCategoryWriteRequest) {
  return apiClient.request<ItemCategoryResponse>(`/api/item-categories/${id}`, {
    method: "PUT",
    json: request,
  });
}

export function deleteItemCategory(id: number) {
  return apiClient.request<ItemCategoryDeletionResponse>(`/api/item-categories/${id}`, {
    method: "DELETE",
  });
}
