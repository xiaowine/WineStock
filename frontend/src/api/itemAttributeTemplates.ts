// 本文件拥有物品属性模板 DTO 和请求，属于 frontend HTTP 边界；模板只提供可选预设。
import { apiClient } from './client'
import type { TemplateFieldResponse } from './templateFields'

export interface ItemAttributeTemplateResponse {
  id: number
  name: string
  description: string | null
  default_inbound_template_id: number | null
  fields: TemplateFieldResponse[]
  created_at: string
  updated_at: string
}

export function listItemAttributeTemplates(signal?: AbortSignal) {
  return apiClient.request<ItemAttributeTemplateResponse[]>('/api/item-attribute-templates', { signal })
}

export function getItemAttributeTemplate(id: number, signal?: AbortSignal) {
  return apiClient.request<ItemAttributeTemplateResponse>(`/api/item-attribute-templates/${id}`, { signal })
}
