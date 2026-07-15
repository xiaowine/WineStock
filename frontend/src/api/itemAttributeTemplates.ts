// 本文件拥有物品属性模板 DTO 和请求，属于 frontend HTTP 边界；模板只提供可选预设。
import { apiClient } from './client'
import type { TemplateFieldResponse } from './templateFields'

/** 物品模板字段支持的单位交互模式。 */
export type ItemAttributeUnitMode = 'none' | 'fixed' | 'select'

/** 由物品属性模板显式定义的单位规则。 */
export interface ItemAttributeUnitRule {
  /** 控制物品录入时隐藏、只读、选择或自由填写单位。 */
  mode: ItemAttributeUnitMode
  /** fixed 模式的固定单位，其它模式为空。 */
  value: string | null
  /** select 模式的单位候选项，其它模式为空。 */
  options: string[] | null
}

/** 带物品专属单位规则的模板字段响应。 */
export interface ItemAttributeTemplateFieldResponse extends TemplateFieldResponse {
  /** 服务端归一化后的必需单位规则。 */
  unit: ItemAttributeUnitRule
  /** 是否作为物品目录中的关键属性展示。 */
  catalog_visible: boolean
}

export interface ItemAttributeTemplateResponse {
  id: number
  name: string
  description: string | null
  default_inbound_template_id: number | null
  item_usage_count: number
  fields: ItemAttributeTemplateFieldResponse[]
  created_at: string
  updated_at: string
}

export interface ItemAttributeTemplateDeletionResponse {
  affected_active_item_count: number
}

/** 更新物品属性模板时整体提交的字段定义。 */
export interface ItemAttributeTemplateFieldRequest {
  definition_id: number | null
  field_name: string
  field_type: TemplateFieldResponse['field_type']
  default_value: string | null
  options: string[] | null
  required: boolean
  searchable: boolean
  catalog_visible: boolean
  unit: ItemAttributeUnitRule
}

/** 物品属性模板创建与整体保存请求。 */
export interface ItemAttributeTemplateWriteRequest {
  name: string
  description: string | null
  default_inbound_template_id: number | null
  fields: ItemAttributeTemplateFieldRequest[]
}

/** 更新接口允许只提交发生变化的部分；字段存在时仍按完整数组替换。 */
export interface ItemAttributeTemplateUpdateRequest {
  name?: string
  description?: string | null
  default_inbound_template_id?: number | null
  fields?: ItemAttributeTemplateFieldRequest[]
}

export interface TemplateCopyRequest {
  name: string
}

export function listItemAttributeTemplates(signal?: AbortSignal) {
  return apiClient.request<ItemAttributeTemplateResponse[]>('/api/item-attribute-templates', { signal })
}

export function getItemAttributeTemplate(id: number, signal?: AbortSignal) {
  return apiClient.request<ItemAttributeTemplateResponse>(`/api/item-attribute-templates/${id}`, { signal })
}

export function createItemAttributeTemplate(request: ItemAttributeTemplateWriteRequest) {
  return apiClient.request<ItemAttributeTemplateResponse>('/api/item-attribute-templates', {
    method: 'POST',
    json: request,
  })
}

/** 更新物品属性模板基础信息和完整字段配置。 */
export function updateItemAttributeTemplate(id: number, request: ItemAttributeTemplateUpdateRequest) {
  return apiClient.request<ItemAttributeTemplateResponse>(`/api/item-attribute-templates/${id}`, {
    method: 'PUT',
    json: request,
  })
}

export function copyItemAttributeTemplate(id: number, request: TemplateCopyRequest) {
  return apiClient.request<ItemAttributeTemplateResponse>(`/api/item-attribute-templates/${id}/copy`, {
    method: 'POST',
    json: request,
  })
}

export function deleteItemAttributeTemplate(id: number) {
  return apiClient.request<ItemAttributeTemplateDeletionResponse>(`/api/item-attribute-templates/${id}`, { method: 'DELETE' })
}
