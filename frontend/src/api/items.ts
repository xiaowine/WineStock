// 本文件拥有物品命令、目录、选择器、编辑资料和库存详情 HTTP 契约；不同场景不共享万能响应。
import { apiClient } from './client'
import type { FileAttributeReference } from './inbound'
import type { TemplateFieldType } from './templateFields'

export interface ItemAttributeResponse {
  id: number
  definition_id: number
  custom: boolean
  field_name: string
  field_type: TemplateFieldType
  options: string[] | null
  unit_mode: 'none' | 'fixed' | 'select'
  fixed_unit: string | null
  unit_options: string[] | null
  value: string | number | boolean | FileAttributeReference
  unit: string | null
  sort_order: number
}

export interface ItemAttributeRequest {
  definition_id?: number
  field_name: string
  field_type: TemplateFieldType
  options?: string[]
  unit_mode?: 'none' | 'fixed' | 'select'
  fixed_unit?: string
  unit_options?: string[]
  value: string | number | boolean | FileAttributeReference
  unit?: string
}

/** 已有物品编辑器恢复草稿所需的完整资料。 */
export interface ItemEditorResponse {
  id: number
  name: string
  sku: string
  category_id: number | null
  attribute_template_id: number | null
  image_file_id: number
  image_url: string
  unit: string
  description: string | null
  default_price: number | null
  reorder_point: number | null
  attributes: ItemAttributeResponse[]
  created_at: string
  updated_at: string
}

export interface ItemMutationResponse {
  id: number
  updated_at: string
}

export type ItemStockState = 'out_of_stock' | 'reorder_due' | 'needs_configuration' | 'normal'
export type ItemStockFilter = 'all' | 'needs_attention' | 'out_of_stock' | 'reorder_due' | 'needs_configuration'
export type ItemCatalogSort = 'replenishment_priority' | 'name' | 'quantity_asc' | 'quantity_desc' | 'inventory_value_desc' | 'updated_desc'

export interface CatalogAttributeResponse {
  name: string
  value: string | number | boolean | FileAttributeReference
  unit: string | null
}

export interface ItemCatalogEntryResponse {
  id: number
  name: string
  sku: string
  category_id: number | null
  category_name: string | null
  attribute_template_id: number | null
  image_file_id: number
  image_url: string
  unit: string
  default_price: number | null
  reorder_point: number | null
  catalog_attributes: CatalogAttributeResponse[]
  current_quantity: number
  inventory_value: number
  location_count: number
  batch_count: number
  stock_state: ItemStockState
  updated_at: string
}

export interface ItemCatalogCountsResponse {
  total: number
  needs_attention: number
  out_of_stock: number
  reorder_due: number
  needs_configuration: number
}

export interface ItemCatalogPageResponse {
  items: ItemCatalogEntryResponse[]
  counts: ItemCatalogCountsResponse
  total: number
  page: number
  page_size: number
  total_pages: number
}

export interface ItemFilterValueResponse {
  value: string
  count: number
}

export interface ItemFilterFieldResponse {
  key: string
  label: string
  source: 'base' | 'template'
  value_type: TemplateFieldType | 'mixed'
  values: ItemFilterValueResponse[]
}

export interface ItemFilterValuesResponse {
  fields: ItemFilterFieldResponse[]
}

export interface ItemCatalogFilters {
  categoryId: number | null
  attributeTemplateId: number | null
  fields: Record<string, string[]>
}

/** 入库等业务选择器使用的轻量物品资料。 */
export interface ItemOptionResponse {
  id: number
  name: string
  sku: string
  category_id: number | null
  category_name: string | null
  attribute_template_id: number | null
  recommended_inbound_template_id: number | null
  recommended_inbound_template_available: boolean
  image_file_id: number
  image_url: string
  unit: string
}

export interface ItemOptionPageResponse {
  items: ItemOptionResponse[]
  total: number
  page: number
  page_size: number
  total_pages: number
}

export interface ItemLocationStockResponse {
  location_id: number
  location_name: string
  quantity: number
  value: number
  batch_count: number
}

export interface ItemInventoryResponse {
  id: number
  name: string
  sku: string
  unit: string
  reorder_point: number | null
  current_quantity: number
  inventory_value: number
  stock_state: ItemStockState
  batch_count: number
  locations: ItemLocationStockResponse[]
}

export interface ItemBatchStockResponse {
  id: number
  batch_no: string
  location_id: number
  location_name: string
  initial_quantity: number
  remaining_quantity: number
  unit_cost: number
  value: number
  received_at: string
  expires_at: string | null
}

export interface ItemBatchPageResponse {
  items: ItemBatchStockResponse[]
  total: number
  page: number
  page_size: number
  total_pages: number
}

export interface ItemCreateRequest {
  name: string
  sku: string
  category_id?: number
  attribute_template_id?: number
  image_file_id: number
  unit: string
  description?: string
  default_price?: number
  reorder_point?: number
  attributes: ItemAttributeRequest[]
}

export interface ItemUpdateRequest extends Partial<Omit<ItemCreateRequest, 'attributes' | 'category_id' | 'attribute_template_id' | 'description' | 'default_price' | 'reorder_point'>> {
  category_id?: number | null
  attribute_template_id?: number | null
  description?: string | null
  default_price?: number | null
  reorder_point?: number | null
  attributes?: ItemAttributeRequest[]
}

export function createItem(request: ItemCreateRequest) {
  return apiClient.request<ItemMutationResponse>('/api/items', { method: 'POST', json: request })
}

export function updateItem(id: number, request: ItemUpdateRequest) {
  return apiClient.request<ItemMutationResponse>(`/api/items/${id}`, { method: 'PUT', json: request })
}

/** 软删除物品；历史库存与业务记录仍由服务端保留。 */
export function deleteItem(id: number) {
  return apiClient.request<void>(`/api/items/${id}`, { method: 'DELETE' })
}

export function getItem(id: number, signal?: AbortSignal) {
  return apiClient.request<ItemEditorResponse>(`/api/items/${id}`, { signal })
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
  return apiClient.request<ItemCatalogPageResponse>('/api/items', {
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
  })
}

export function getItemFilterValues(
  search: string,
  stockFilter: ItemStockFilter,
  filters: ItemCatalogFilters,
  signal?: AbortSignal,
) {
  return apiClient.request<ItemFilterValuesResponse>('/api/items/filter-values', {
    query: {
      search: search.trim() || undefined,
      category_id: filters.categoryId ?? undefined,
      attribute_template_id: filters.attributeTemplateId ?? undefined,
      stock_filter: stockFilter,
      filters: serializeItemCatalogFilters(filters),
    },
    signal,
  })
}

export function emptyItemCatalogFilters(): ItemCatalogFilters {
  return { categoryId: null, attributeTemplateId: null, fields: {} }
}

export function cloneItemCatalogFilters(filters: ItemCatalogFilters): ItemCatalogFilters {
  return {
    categoryId: filters.categoryId,
    attributeTemplateId: filters.attributeTemplateId,
    fields: Object.fromEntries(Object.entries(filters.fields).map(([key, values]) => [key, [...values]])),
  }
}

export function serializeItemCatalogFilters(filters: ItemCatalogFilters): string | undefined {
  const serialized = Object.entries(filters.fields)
    .map(([key, values]) => ({ key, values: [...new Set(values.map((value) => value.trim()).filter(Boolean))].sort() }))
    .filter((filter) => filter.values.length)
    .sort((left, right) => left.key.localeCompare(right.key))
  return serialized.length ? JSON.stringify(serialized) : undefined
}

export function listItemOptions(search: string, page: number, pageSize: number, signal?: AbortSignal) {
  return apiClient.request<ItemOptionPageResponse>('/api/items/options', {
    query: { page, page_size: pageSize, search: search.trim() || undefined },
    signal,
  })
}

export function getItemInventory(id: number, signal?: AbortSignal) {
  return apiClient.request<ItemInventoryResponse>(`/api/items/${id}/inventory`, { signal })
}

export function listItemBatches(id: number, page: number, pageSize = 20, signal?: AbortSignal) {
  return apiClient.request<ItemBatchPageResponse>(`/api/items/${id}/batches`, {
    query: { page, page_size: pageSize },
    signal,
  })
}
