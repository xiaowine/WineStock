// 本文件拥有物品命令、目录、选择器、编辑资料和库存详情 HTTP 契约；不同场景不共享万能响应。
// DTO 通过 contract.ts 别名映射到生成 schema，导出名保持稳定；仅前端本地的筛选草稿类型仍手写。
import { apiClient } from "./client";
import type { ApiResponse, ApiSchema } from "./contract";

export type ItemAttributeResponse = ApiResponse<ApiSchema<"ItemAttributeResponse">>;

export type ItemAttributeRequest = ApiSchema<"ItemAttributeRequest">;

/** 物品 file 属性保存的稳定服务端引用。 */
export type FileAttributeReference = ApiSchema<"FileAttributeReference">;

/** 已有物品编辑器恢复草稿所需的完整资料。 */
export type ItemEditorResponse = ApiResponse<ApiSchema<"ItemEditorResponse">>;

export type ItemMutationResponse = ApiResponse<ApiSchema<"ItemMutationResponse">>;

export type LcscItemLookupParameterResponse = ApiResponse<ApiSchema<"LcscLookupParameterResponse">>;

/** 立创资料服务返回的单物品候选信息；应用前不会修改物品草稿。 */
export type LcscItemLookupResponse = ApiResponse<ApiSchema<"LcscItemLookupResponse">>;

export type ItemStockState = ApiSchema<"ItemStockState">;
export type ItemStockFilter = ApiSchema<"ItemStockFilter">;
export type ItemCatalogSort = ApiSchema<"ItemCatalogSort">;

export type CatalogAttributeResponse = ApiResponse<ApiSchema<"CatalogAttributeResponse">>;

export type ItemCatalogEntryResponse = ApiResponse<ApiSchema<"ItemCatalogEntryResponse">>;

export type ItemCatalogCountsResponse = ApiResponse<ApiSchema<"ItemCatalogCountsResponse">>;

export type ItemCatalogPageResponse = ApiResponse<ApiSchema<"ItemCatalogPageResponse">>;

export type ItemFilterValueResponse = ApiResponse<ApiSchema<"FilterValueResponse">>;

export type ItemFilterFieldResponse = ApiResponse<ApiSchema<"FilterFieldResponse">>;

export type ItemFilterValuesResponse = ApiResponse<ApiSchema<"FilterValuesResponse">>;

/** 目录页高级筛选草稿；仅前端本地状态，不是 HTTP 契约。 */
export interface ItemCatalogFilters {
  categoryId: number | null;
  attributeTemplateId: number | null;
  fields: Record<string, string[]>;
}

/** 入库等业务选择器使用的轻量物品资料。 */
export type ItemOptionResponse = ApiResponse<ApiSchema<"ItemOptionResponse">>;

export type ItemOptionPageResponse = ApiResponse<ApiSchema<"ItemOptionPageResponse">>;

export type ItemLocationStockResponse = ApiResponse<ApiSchema<"ItemLocationStockResponse">>;

export type ItemInventoryResponse = ApiResponse<ApiSchema<"ItemInventoryResponse">>;

export type ItemBatchStockResponse = ApiResponse<ApiSchema<"ItemBatchStockResponse">>;

export type ItemBatchPageResponse = ApiResponse<ApiSchema<"ItemBatchPageResponse">>;

export type ItemCreateRequest = ApiSchema<"ItemCreateRequest">;

export type ItemUpdateRequest = ApiSchema<"ItemUpdateRequest">;

export function createItem(request: ItemCreateRequest) {
  return apiClient.request<ItemMutationResponse>("/api/items", { method: "POST", json: request });
}

export function updateItem(id: number, request: ItemUpdateRequest) {
  return apiClient.request<ItemMutationResponse>(`/api/items/${id}`, {
    method: "PUT",
    json: request,
  });
}

/** 软删除物品；历史库存与业务记录仍由服务端保留。 */
export function deleteItem(id: number) {
  return apiClient.request<void>(`/api/items/${id}`, { method: "DELETE" });
}

export function getItem(id: number, signal?: AbortSignal) {
  return apiClient.request<ItemEditorResponse>(`/api/items/${id}`, { signal });
}

export function lookupLcscItem(productCode: string, signal?: AbortSignal) {
  return apiClient.request<LcscItemLookupResponse>(
    `/api/items/lookups/lcsc/${encodeURIComponent(productCode)}`,
    { signal },
  );
}

/** 通过 WineStock Core 读取立创商品首图；浏览器不直接访问第三方图片地址。 */
export function readLcscItemImage(productCode: string, signal?: AbortSignal) {
  return apiClient.request<Blob>(
    `/api/items/lookups/lcsc/${encodeURIComponent(productCode)}/image`,
    { signal, responseType: "blob" },
  );
}

export function listItemCatalog(
  search: string,
  page: number,
  pageSize: number,
  stockFilter: ItemStockFilter,
  sort: ItemCatalogSort,
  filters: ItemCatalogFilters,
  signal?: AbortSignal,
) {
  return apiClient.request<ItemCatalogPageResponse>("/api/items", {
    query: {
      page,
      page_size: pageSize,
      search: search.trim() || undefined,
      category_id: filters.categoryId ?? undefined,
      attribute_template_id: filters.attributeTemplateId ?? undefined,
      stock_filter: stockFilter,
      sort,
      filters: serializeItemCatalogFilters(filters),
    },
    signal,
  });
}

export function getItemFilterValues(
  search: string,
  stockFilter: ItemStockFilter,
  filters: ItemCatalogFilters,
  signal?: AbortSignal,
) {
  return apiClient.request<ItemFilterValuesResponse>("/api/items/filter-values", {
    query: {
      search: search.trim() || undefined,
      category_id: filters.categoryId ?? undefined,
      attribute_template_id: filters.attributeTemplateId ?? undefined,
      stock_filter: stockFilter,
      filters: serializeItemCatalogFilters(filters),
    },
    signal,
  });
}

export function emptyItemCatalogFilters(): ItemCatalogFilters {
  return { categoryId: null, attributeTemplateId: null, fields: {} };
}

export function cloneItemCatalogFilters(filters: ItemCatalogFilters): ItemCatalogFilters {
  return {
    categoryId: filters.categoryId,
    attributeTemplateId: filters.attributeTemplateId,
    fields: Object.fromEntries(
      Object.entries(filters.fields).map(([key, values]) => [key, [...values]]),
    ),
  };
}

export function serializeItemCatalogFilters(filters: ItemCatalogFilters): string | undefined {
  const serialized = Object.entries(filters.fields)
    .map(([key, values]) => ({
      key,
      values: [...new Set(values.map((value) => value.trim()).filter(Boolean))].sort(),
    }))
    .filter((filter) => filter.values.length)
    .sort((left, right) => left.key.localeCompare(right.key));
  return serialized.length ? JSON.stringify(serialized) : undefined;
}

export function listItemOptions(
  search: string,
  page: number,
  pageSize: number,
  signal?: AbortSignal,
) {
  return apiClient.request<ItemOptionPageResponse>("/api/items/options", {
    query: { page, page_size: pageSize, search: search.trim() || undefined },
    signal,
  });
}

export function getItemInventory(id: number, signal?: AbortSignal) {
  return apiClient.request<ItemInventoryResponse>(`/api/items/${id}/inventory`, { signal });
}

export function listItemBatches(id: number, page: number, pageSize = 20, signal?: AbortSignal) {
  return apiClient.request<ItemBatchPageResponse>(`/api/items/${id}/batches`, {
    query: { page, page_size: pageSize },
    signal,
  });
}
