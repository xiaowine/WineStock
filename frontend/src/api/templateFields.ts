// 本文件拥有两类属性模板共享的字段 DTO，属于 frontend HTTP 类型边界；它不发起请求。
export type TemplateFieldType = 'text' | 'number' | 'select' | 'date' | 'file' | 'url' | 'boolean'

export interface TemplateFieldResponse {
  id: number
  field_name: string
  field_type: TemplateFieldType
  default_value: string | null
  options: string[] | null
  required: boolean
  searchable: boolean
  sort_order: number
}

export interface TemplateFieldRequest {
  field_name: string
  field_type: TemplateFieldType
  default_value: string | null
  options: string[] | null
  required: boolean
  searchable: boolean
}
