// 本文件拥有库存物品查询 DTO 和 HTTP 调用，属于 frontend API 边界；它不管理入库草稿或页面状态。
import { apiClient } from './client'
import type { FileAttributeReference } from './inbound'
import type { TemplateFieldType } from './templateFields'

export interface ItemAttributeResponse {
  id: number
  template_field_id: number | null
  field_name: string
  field_type: TemplateFieldType
  value: string | number | boolean | FileAttributeReference
  unit: string | null
  sort_order: number
}

export interface ItemAttributeRequest {
  template_field_id?: number
  field_name: string
  field_type: TemplateFieldType
  value: string | number | boolean | FileAttributeReference
  unit?: string
}

/** 物品列表使用的基础资料。 */
export interface ItemResponse {
  /** 物品数据库 ID。 */
  id: number
  /** 物品名称。 */
  name: string
  /** 唯一物品编号。 */
  sku: string
  /** 物品分类 ID。 */
  category_id: number | null
  /** 可选物品属性模板 ID。 */
  attribute_template_id: number | null
  /** 必选物品主图文件对象 ID。 */
  image_file_id: number
  /** 物品主图受控读取地址。 */
  image_url: string
  /** 计量单位。 */
  unit: string
  /** 可选物品描述。 */
  description: string | null
  /** 入库时可作为初始值的参考单价。 */
  default_price: number | null
  /** 再订货点。 */
  reorder_point: number | null
  /** 物品自身的类型化属性。 */
  attributes: ItemAttributeResponse[]
  /** 创建时间。 */
  created_at: string
  /** 最近更新时间。 */
  updated_at: string
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

/** 通用分页响应。 */
export interface PaginatedResponse<TItem> {
  /** 当前页数据。 */
  items: TItem[]
  /** 满足条件的总记录数。 */
  total: number
  /** 当前页码。 */
  page: number
  /** 每页数量。 */
  page_size: number
  /** 总页数。 */
  total_pages: number
}

export function createItem(request: ItemCreateRequest) {
  return apiClient.request<ItemResponse>('/api/items', { method: 'POST', json: request })
}

export function updateItem(id: number, request: ItemUpdateRequest) {
  return apiClient.request<ItemResponse>(`/api/items/${id}`, { method: 'PUT', json: request })
}

/** 查询物品列表；空搜索不发送 search 参数，避免触发服务端空搜索校验。 */
export function listItems(search: string, page: number, pageSize: number, signal?: AbortSignal) {
  const normalizedSearch = search.trim()
  return apiClient.request<PaginatedResponse<ItemResponse>>('/api/items', {
    query: {
      page,
      page_size: pageSize,
      search: normalizedSearch || undefined,
    },
    signal,
  })
}
