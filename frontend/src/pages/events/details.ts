// 本文件把版本不固定的审计详情 JSON 转换为安全展示模型；它不请求当前业务对象覆盖历史快照。
import type { EventLogResponse } from '../../api/events'
import { eventFieldLabel } from './catalog'

export interface EventDetailEntry {
  key: string
  label: string
  value: unknown
}

export interface EventDiffRow {
  key: string
  label: string
  previous: unknown
  next: unknown
}

export interface PermissionChanges {
  added: string[]
  removed: string[]
}

export function isJsonObject(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
}

/** 从通用 previous/new 快照生成真实变化行。 */
export function eventDiffRows(details: unknown): EventDiffRow[] {
  if (!isJsonObject(details)) {
    return []
  }
  const previous = details.previous
  const next = details.new
  if (!isJsonObject(previous) || !isJsonObject(next)) return []
  const declared = Array.isArray(details.changed_fields)
    ? details.changed_fields.filter((value): value is string => typeof value === 'string')
    : []
  const keys = declared.length > 0
    ? declared
    : Array.from(new Set([...Object.keys(previous), ...Object.keys(next)]))
  return keys
    .filter((key) => jsonSignature(previous[key]) !== jsonSignature(next[key]))
    .map((key) => ({
      key,
      label: eventFieldLabel(key),
      previous: previous[key],
      next: next[key],
    }))
}

/** 计算用户权限事件中的新增和移除项。 */
export function eventPermissionChanges(details: unknown): PermissionChanges | null {
  if (!isJsonObject(details)) return null
  const previous = stringArray(details.previous_permissions)
  const next = stringArray(details.new_permissions)
  if (previous === null || next === null) return null
  const previousSet = new Set(previous)
  const nextSet = new Set(next)
  return {
    added: next.filter((permission) => !previousSet.has(permission)),
    removed: previous.filter((permission) => !nextSet.has(permission)),
  }
}

/** 通用详情键值行，排除已经由差异和权限区展示的结构字段。 */
export function eventDetailEntries(details: unknown): EventDetailEntry[] {
  if (!isJsonObject(details)) return []
  const omitted = new Set([
    'previous', 'new', 'changed_fields', 'previous_permissions', 'new_permissions',
  ])
  return Object.entries(details)
    .filter(([key]) => !omitted.has(key))
    .map(([key, value]) => ({ key, label: eventFieldLabel(key), value }))
}

/** 删除事件优先展示服务端记录的删除前快照。 */
export function eventPreviousSnapshot(details: unknown): EventDetailEntry[] {
  if (!isJsonObject(details) || !isJsonObject(details.previous) || isJsonObject(details.new)) {
    return []
  }
  return Object.entries(details.previous).map(([key, value]) => ({
    key,
    label: eventFieldLabel(key),
    value,
  }))
}

/** 为列表生成紧凑、可降级的审计摘要。 */
export function eventSummary(event: EventLogResponse): string {
  const details = event.details
  if (!isJsonObject(details)) {
    if (details === null) return '暂无结构化详情'
    if (Array.isArray(details)) return `包含 ${details.length} 项详情`
    return formatJsonValue(details)
  }

  if (event.entity_type === 'item' && event.action === 'created') {
    const name = stringValue(details.name)
    const unit = stringValue(details.unit)
    return [name ? `创建“${name}”` : '创建物品', unit ? `单位 ${unit}` : ''].filter(Boolean).join('，')
  }
  if (event.entity_type === 'user' && details.field === 'permissions') {
    const changes = eventPermissionChanges(details)
    if (changes) return `新增 ${changes.added.length} 项权限，移除 ${changes.removed.length} 项权限`
  }
  if (event.entity_type === 'location_transfer') {
    const batch = idValue(details.batch_id)
    const from = idValue(details.from_location_id)
    const to = idValue(details.to_location_id)
    const quantity = details.quantity
    return `批次 ${batch} 从库位 ${from} 移至 ${to}${quantity === undefined ? '' : `，数量 ${formatJsonValue(quantity)}`}`
  }
  if (['inbound', 'outbound'].includes(event.entity_type) && typeof details.item_count === 'number') {
    const prefix = event.action === 'approved' ? '审批' : event.action === 'rejected' ? '驳回' : '包含'
    return `${prefix} ${details.item_count} 条明细`
  }

  const diff = eventDiffRows(details)
  if (diff.length > 0) return `${diff.slice(0, 2).map((row) => row.label).join('、')}${diff.length > 2 ? `等 ${diff.length} 项` : ''}发生变化`
  const entries = eventDetailEntries(details)
  if (entries.length > 0) {
    return entries.slice(0, 2).map((entry) => `${entry.label}：${formatJsonValue(entry.value)}`).join(' · ')
  }
  return `包含 ${Object.keys(details).length} 个详情字段`
}

/** 把任意 JSON 值转换为不执行 HTML 的紧凑文本。 */
export function formatJsonValue(value: unknown): string {
  if (value === undefined) return '无记录'
  if (value === null) return '未设置'
  if (typeof value === 'boolean') return value ? '是' : '否'
  if (typeof value === 'string') return value || '空字符串'
  if (typeof value === 'number') return String(value)
  if (Array.isArray(value) && value.every((item) => item === null || ['string', 'number', 'boolean'].includes(typeof item))) {
    return value.length > 0 ? value.map((item) => formatJsonValue(item)).join('、') : '空列表'
  }
  return safeJsonStringify(value, 0)
}

export function safeJsonStringify(value: unknown, spacing = 2): string {
  try {
    return JSON.stringify(value, null, spacing) ?? 'null'
  } catch {
    return '无法序列化当前详情'
  }
}

function jsonSignature(value: unknown): string {
  return value === undefined ? '__missing__' : safeJsonStringify(value, 0)
}

function stringArray(value: unknown): string[] | null {
  return Array.isArray(value) && value.every((item) => typeof item === 'string')
    ? value as string[]
    : null
}

function stringValue(value: unknown): string | null {
  return typeof value === 'string' && value ? value : null
}

function idValue(value: unknown): string {
  return typeof value === 'number' ? `#${value}` : '未知'
}
