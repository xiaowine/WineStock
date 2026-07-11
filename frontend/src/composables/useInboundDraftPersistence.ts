// 本文件拥有入库草稿的 localStorage 序列化、恢复和浏览器离开提示；它不提交业务 API。
import { onBeforeUnmount, onMounted, watch, type ComputedRef, type Ref } from 'vue'
import type { ItemResponse } from '../api/items'
import { createLineId, type AttributeValue, type FileDraftValue, type InboundDraftLine } from '../pages/inbound-draft/model'

const storageKey = 'winestock.inbound-draft.v3'

interface PersistedFileValue {
  kind: 'file'
  fileId: number
  name: string
  mimeType: string
  sizeBytes: number
}

interface PersistedDraft {
  version: 3
  source: string
  notes: string
  notesOpen: boolean
  lines: Array<{
    lineId: string
    item: ItemResponse
    quantity: number
    unitPrice: number
    locationId: number | null
    batchNo: string
    expiresAt: string
    extAttributes: Record<string, string | number | boolean | PersistedFileValue>
    templateId: number | null
  }>
}

/** 绑定页面草稿引用；调用 restore 后再 resume，避免恢复过程中覆盖本地记录。 */
export function useInboundDraftPersistence(
  source: Ref<string>, notes: Ref<string>, notesOpen: Ref<boolean>,
  lines: Ref<InboundDraftLine[]>, hasDraft: ComputedRef<boolean>,
) {
  let suspended = true
  watch([source, notes, notesOpen, lines], save, { deep: true })
  onMounted(() => window.addEventListener('beforeunload', handleBeforeUnload))
  onBeforeUnmount(() => window.removeEventListener('beforeunload', handleBeforeUnload))

  function resume(): void { suspended = false; save() }
  function remove(): void { localStorage.removeItem(storageKey) }

  function restore(): boolean {
    const raw = localStorage.getItem(storageKey)
    if (!raw) return false
    try {
      const draft = JSON.parse(raw) as PersistedDraft
      if (draft.version !== 3 || !Array.isArray(draft.lines)) throw new Error('invalid draft')
      source.value = typeof draft.source === 'string' ? draft.source : ''
      notes.value = typeof draft.notes === 'string' ? draft.notes : ''
      notesOpen.value = Boolean(draft.notesOpen || notes.value)
      lines.value = draft.lines.map((line) => ({
        lineId: line.lineId || createLineId(), item: line.item, quantity: line.quantity,
        unitPrice: line.unitPrice, locationId: line.locationId, batchNo: line.batchNo || '',
        expiresAt: line.expiresAt || '', extAttributes: restoreAttributes(line.extAttributes),
        template: null, templateLoading: false, templateId: line.templateId, templateError: '',
      }))
      return true
    } catch {
      remove()
      return false
    }
  }

  function save(): void {
    if (suspended) return
    if (!hasDraft.value) { remove(); return }
    const draft: PersistedDraft = {
      version: 3, source: source.value, notes: notes.value, notesOpen: notesOpen.value,
      lines: lines.value.map((line) => ({
        lineId: line.lineId, item: line.item, quantity: line.quantity, unitPrice: line.unitPrice,
        locationId: line.locationId, batchNo: line.batchNo, expiresAt: line.expiresAt,
        extAttributes: persistAttributes(line.extAttributes), templateId: line.templateId,
      })),
    }
    try { localStorage.setItem(storageKey, JSON.stringify(draft)) } catch { /* 配额失败不阻断当前录入。 */ }
  }

  function handleBeforeUnload(event: BeforeUnloadEvent): void {
    if (!hasDraft.value) return
    event.preventDefault()
    event.returnValue = ''
  }

  return { restoreDraft: restore, resumeDraftSaving: resume, removePersistedDraft: remove }
}

function persistAttributes(attributes: Record<string, AttributeValue>): PersistedDraft['lines'][number]['extAttributes'] {
  const result: PersistedDraft['lines'][number]['extAttributes'] = {}
  for (const [name, value] of Object.entries(attributes)) {
    if (typeof value === 'object' && value?.kind === 'file') {
      if (value.status === 'uploaded' && value.fileId) result[name] = {
        kind: 'file', fileId: value.fileId, name: value.name, mimeType: value.mimeType, sizeBytes: value.sizeBytes,
      }
    } else if (value !== undefined) result[name] = value as string | number | boolean
  }
  return result
}

function restoreAttributes(attributes: PersistedDraft['lines'][number]['extAttributes']): Record<string, AttributeValue> {
  const result: Record<string, AttributeValue> = {}
  for (const [name, value] of Object.entries(attributes ?? {})) {
    result[name] = isPersistedFileValue(value)
      ? { kind: 'file', fileId: value.fileId, name: value.name, mimeType: value.mimeType, sizeBytes: value.sizeBytes, status: 'uploaded', progress: 100, error: '' } satisfies FileDraftValue
      : value as string | number | boolean
  }
  return result
}

function isPersistedFileValue(value: unknown): value is PersistedFileValue {
  return typeof value === 'object' && value !== null && 'kind' in value && value.kind === 'file' &&
    'fileId' in value && typeof value.fileId === 'number' && 'name' in value && typeof value.name === 'string' &&
    'mimeType' in value && typeof value.mimeType === 'string' && 'sizeBytes' in value && typeof value.sizeBytes === 'number'
}
