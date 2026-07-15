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

export interface ItemCategoryWriteRequest {
  name: string
  description: string | null
  sort_order: number
}

export function listItemCategories(signal?: AbortSignal) {
  return apiClient.request<ItemCategoryResponse[]>('/api/item-categories', { signal })
}

export function getItemCategory(id: number, signal?: AbortSignal) {
  return apiClient.request<ItemCategoryResponse>(`/api/item-categories/${id}`, { signal })
}

export function createItemCategory(request: ItemCategoryWriteRequest) {
  return apiClient.request<ItemCategoryResponse>('/api/item-categories', {
    method: 'POST',
    json: request,
  })
}

export function updateItemCategory(id: number, request: ItemCategoryWriteRequest) {
  return apiClient.request<ItemCategoryResponse>(`/api/item-categories/${id}`, {
    method: 'PUT',
    json: request,
  })
}

export function deleteItemCategory(id: number) {
  return apiClient.request<void>(`/api/item-categories/${id}`, { method: 'DELETE' })
}
