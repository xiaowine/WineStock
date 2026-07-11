// 本文件拥有物品分类 DTO 和请求，属于 frontend HTTP 边界；分类不承担属性模板职责。
import { apiClient } from './client'

export interface ItemCategoryResponse {
  id: number
  name: string
  description: string | null
  sort_order: number
  created_at: string
  updated_at: string
}

export function listItemCategories(signal?: AbortSignal) {
  return apiClient.request<ItemCategoryResponse[]>('/api/item-categories', { signal })
}
