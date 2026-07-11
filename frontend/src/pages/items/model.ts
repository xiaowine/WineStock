// 本文件拥有物品编辑草稿与 HTTP 请求转换，属于 frontend 页面模型层；它不发起 API 请求。
import type { FileAttributeReference } from '../../api/inbound'
import type { ItemAttributeRequest, ItemResponse } from '../../api/items'
import type { ItemAttributeTemplateResponse } from '../../api/itemAttributeTemplates'
import type { TemplateFieldResponse, TemplateFieldType } from '../../api/templateFields'
import type { FileDraftValue } from '../inbound-draft/model'

export interface ItemAttributeDraft {
  key: string
  templateFieldId: number | null
  fieldName: string
  fieldType: TemplateFieldType
  value: string | number | boolean | FileDraftValue | undefined
  unit: string
  fileTemporary: boolean
}

export interface ItemDraft {
  id: number | null
  name: string
  sku: string
  categoryId: number | null
  attributeTemplateId: number | null
  unit: string
  description: string
  defaultPrice: number | null
  reorderPoint: number | null
  attributes: ItemAttributeDraft[]
}

export function emptyItemDraft(): ItemDraft {
  return { id: null, name: '', sku: '', categoryId: null, attributeTemplateId: null, unit: '个', description: '', defaultPrice: null, reorderPoint: null, attributes: [] }
}

export function draftFromItem(item: ItemResponse): ItemDraft {
  return {
    id: item.id, name: item.name, sku: item.sku, categoryId: item.category_id,
    attributeTemplateId: item.attribute_template_id, unit: item.unit,
    description: item.description ?? '', defaultPrice: item.default_price,
    reorderPoint: item.reorder_point,
    attributes: item.attributes.map((attribute) => ({
      key: crypto.randomUUID(), templateFieldId: attribute.template_field_id,
      fieldName: attribute.field_name, fieldType: attribute.field_type,
      value: attribute.field_type === 'file'
        ? { kind: 'file', fileId: (attribute.value as FileAttributeReference).file_id, name: `图片 #${(attribute.value as FileAttributeReference).file_id}`, mimeType: 'image/*', sizeBytes: 0, status: 'uploaded', progress: 100, error: '' }
        : attribute.value as string | number | boolean,
      unit: attribute.unit ?? '', fileTemporary: false,
    })),
  }
}

export function applyAttributeTemplate(draft: ItemDraft, template: ItemAttributeTemplateResponse | null): void {
  draft.attributeTemplateId = template?.id ?? null
  draft.attributes.forEach((attribute) => { attribute.templateFieldId = null })
  if (!template) return
  const existing = new Map(draft.attributes.map((attribute) => [attribute.fieldName.toLowerCase(), attribute]))
  for (const field of template.fields) {
    const current = existing.get(field.field_name.toLowerCase())
    if (current) {
      current.templateFieldId = field.id
      if (current.fieldType !== field.field_type) {
        current.fieldType = field.field_type
        current.value = initialFieldValue(field)
        current.unit = ''
      }
      continue
    }
    draft.attributes.push(attributeFromField(field))
  }
}

export function newCustomAttribute(): ItemAttributeDraft {
  return { key: crypto.randomUUID(), templateFieldId: null, fieldName: '', fieldType: 'text', value: '', unit: '', fileTemporary: true }
}

export function itemAttributeRequests(draft: ItemDraft): ItemAttributeRequest[] {
  return draft.attributes.filter((attribute) => attribute.fieldName.trim() && attribute.value !== undefined && attribute.value !== '').map((attribute) => ({
    template_field_id: attribute.templateFieldId ?? undefined,
    field_name: attribute.fieldName.trim(), field_type: attribute.fieldType,
    value: attribute.fieldType === 'file'
      ? { file_id: (attribute.value as FileDraftValue).fileId as number }
      : attribute.fieldType === 'number' ? Number(attribute.value) : attribute.value as string | boolean,
    unit: attribute.unit.trim() || undefined,
  }))
}

function attributeFromField(field: TemplateFieldResponse): ItemAttributeDraft {
  return { key: crypto.randomUUID(), templateFieldId: field.id, fieldName: field.field_name, fieldType: field.field_type, value: initialFieldValue(field), unit: '', fileTemporary: true }
}

function initialFieldValue(field: TemplateFieldResponse): ItemAttributeDraft['value'] {
  return field.default_value === null ? (field.field_type === 'boolean' ? undefined : '')
    : field.field_type === 'number' ? Number(field.default_value)
      : field.field_type === 'boolean' ? field.default_value === 'true' : field.default_value
}
