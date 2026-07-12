// 本文件拥有可跨页面复用的物品编辑草稿、请求转换与变更快照；它不发起 API 请求。
import type { FileAttributeReference } from '../../api/inbound'
import type { ItemAttributeRequest, ItemCreateRequest, ItemResponse, ItemUpdateRequest } from '../../api/items'
import type { ItemAttributeTemplateFieldResponse, ItemAttributeTemplateResponse } from '../../api/itemAttributeTemplates'
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
  /** 当前物品必选主图草稿。 */
  image: FileDraftValue | null
  /** 当前主图是否为尚未绑定的临时图片。 */
  imageTemporary: boolean
  /** 更新成功后需要删除的旧主图文件 ID。 */
  obsoleteImageFileId: number | null
  unit: string
  description: string
  defaultPrice: number | null
  reorderPoint: number | null
  attributes: ItemAttributeDraft[]
}

export function emptyItemDraft(): ItemDraft {
  return { id: null, name: '', sku: '', categoryId: null, attributeTemplateId: null, image: null, imageTemporary: true, obsoleteImageFileId: null, unit: '个', description: '', defaultPrice: null, reorderPoint: null, attributes: [] }
}

export function draftFromItem(
  item: ItemResponse,
  template: ItemAttributeTemplateResponse | null = null,
): ItemDraft {
  const draft: ItemDraft = {
    id: item.id, name: item.name, sku: item.sku, categoryId: item.category_id,
    attributeTemplateId: item.attribute_template_id, unit: item.unit,
    image: { kind: 'file', fileId: item.image_file_id, name: `${item.name} 主图`, mimeType: 'image/*', sizeBytes: 0, status: 'uploaded', progress: 100, error: '' },
    imageTemporary: false,
    obsoleteImageFileId: null,
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
  if (template?.id === item.attribute_template_id) applyAttributeTemplate(draft, template)
  return draft
}

export function applyAttributeTemplate(draft: ItemDraft, template: ItemAttributeTemplateResponse | null): void {
  draft.attributeTemplateId = template?.id ?? null
  draft.attributes = draft.attributes.filter((attribute) =>
    attribute.templateFieldId === null || hasAttributeValue(attribute))
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
      applyTemplateUnit(current, field)
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

/** 把共享物品草稿转换为创建请求，供物品页和其它业务入口复用。 */
export function itemCreateRequest(draft: ItemDraft): ItemCreateRequest {
  return {
    name: draft.name.trim(),
    sku: draft.sku.trim(),
    unit: draft.unit.trim(),
    image_file_id: draft.image?.fileId as number,
    category_id: draft.categoryId ?? undefined,
    attribute_template_id: draft.attributeTemplateId ?? undefined,
    description: draft.description.trim() || undefined,
    default_price: draft.defaultPrice ?? undefined,
    reorder_point: draft.reorderPoint ?? undefined,
    attributes: itemAttributeRequests(draft),
  }
}

/** 按 OpenAPI 的部分更新语义只发送变化字段，并显式保留可清空字段的 null。 */
export function itemUpdateRequest(draft: ItemDraft, baseline: ItemDraft): ItemUpdateRequest {
  const request: ItemUpdateRequest = {}
  if (draft.name.trim() !== baseline.name.trim()) request.name = draft.name.trim()
  if (draft.sku.trim() !== baseline.sku.trim()) request.sku = draft.sku.trim()
  if (draft.unit.trim() !== baseline.unit.trim()) request.unit = draft.unit.trim()
  if (draft.image?.fileId !== baseline.image?.fileId) request.image_file_id = draft.image?.fileId
  if (draft.categoryId !== baseline.categoryId) request.category_id = draft.categoryId
  if (draft.attributeTemplateId !== baseline.attributeTemplateId) request.attribute_template_id = draft.attributeTemplateId

  const description = draft.description.trim() || null
  const baselineDescription = baseline.description.trim() || null
  if (description !== baselineDescription) request.description = description
  if (draft.defaultPrice !== baseline.defaultPrice) request.default_price = draft.defaultPrice
  if (draft.reorderPoint !== baseline.reorderPoint) request.reorder_point = draft.reorderPoint

  const attributes = itemAttributeRequests(draft)
  if (JSON.stringify(attributes) !== JSON.stringify(itemAttributeRequests(baseline))) {
    request.attributes = attributes
  }
  return request
}

/** 生成可比较的草稿快照，用于跨页面统一判断是否存在未保存修改。 */
export function itemDraftFingerprint(draft: ItemDraft): string {
  return JSON.stringify({
    id: draft.id,
    name: draft.name,
    sku: draft.sku,
    categoryId: draft.categoryId,
    attributeTemplateId: draft.attributeTemplateId,
    image: draft.image ? [draft.image.fileId, draft.image.name, draft.image.sizeBytes] : null,
    unit: draft.unit,
    description: draft.description,
    defaultPrice: draft.defaultPrice,
    reorderPoint: draft.reorderPoint,
    attributes: draft.attributes.map((attribute) => ({
      templateFieldId: attribute.templateFieldId,
      fieldName: attribute.fieldName,
      fieldType: attribute.fieldType,
      value: typeof attribute.value === 'object' && attribute.value?.kind === 'file'
        ? [attribute.value.fileId, attribute.value.name, attribute.value.sizeBytes]
        : attribute.value,
      unit: attribute.unit,
    })),
  })
}

function attributeFromField(field: ItemAttributeTemplateFieldResponse): ItemAttributeDraft {
  const attribute = { key: crypto.randomUUID(), templateFieldId: field.id, fieldName: field.field_name, fieldType: field.field_type, value: initialFieldValue(field), unit: '', fileTemporary: true }
  applyTemplateUnit(attribute, field)
  return attribute
}

function initialFieldValue(field: TemplateFieldResponse): ItemAttributeDraft['value'] {
  return field.default_value === null ? (field.field_type === 'boolean' ? undefined : '')
    : field.field_type === 'number' ? Number(field.default_value)
      : field.field_type === 'boolean' ? field.default_value === 'true' : field.default_value
}

function applyTemplateUnit(
  attribute: ItemAttributeDraft,
  field: ItemAttributeTemplateFieldResponse,
): void {
  const rule = field.unit
  if (rule.mode === 'fixed') attribute.unit = rule.value ?? ''
  else if (rule.mode === 'none') attribute.unit = ''
  else if (rule.mode === 'select' && !rule.options?.includes(attribute.unit)) attribute.unit = ''
}

function hasAttributeValue(attribute: ItemAttributeDraft): boolean {
  if (typeof attribute.value === 'string') return attribute.value.trim().length > 0
  return attribute.value !== undefined && attribute.value !== null
}
