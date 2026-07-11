// 本文件拥有入库模板 DTO 和请求，属于 frontend HTTP 边界；模板只描述本次收货属性。
import { apiClient } from './client'
import type { TemplateFieldResponse } from './templateFields'

export interface InboundTemplateResponse {
  id: number
  name: string
  description: string | null
  fields: TemplateFieldResponse[]
  created_at: string
  updated_at: string
}

export function listInboundTemplates(signal?: AbortSignal) {
  return apiClient.request<InboundTemplateResponse[]>('/api/inbound-templates', { signal })
}

export function getInboundTemplate(id: number, signal?: AbortSignal) {
  return apiClient.request<InboundTemplateResponse>(`/api/inbound-templates/${id}`, { signal })
}
