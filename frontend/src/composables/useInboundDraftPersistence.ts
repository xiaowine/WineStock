// 本文件拥有入库草稿的 localStorage/IndexedDB 序列化、恢复和浏览器离开提示；它不提交业务 API。
import { onBeforeUnmount, onMounted, watch, type ComputedRef, type Ref } from 'vue'
import type { ItemOptionResponse } from '../api/items'
import { createLineId, type AttributeValue, type FileDraftValue, type InboundDraftLine } from '../pages/inbound-draft/model'
import { clearInboundDraftImages, readInboundDraftImage, replaceInboundDraftImages } from '../storage/inboundDraftImageStore'
import { createPendingImageDraft } from '../components/attributes/imageDraft'

const storageKey = 'winestock.inbound-draft.v5'
const legacyStorageKey = 'winestock.inbound-draft.v4'
const obsoleteStorageKey = 'winestock.inbound-draft.v3'

interface PersistedFileValue {
  kind: 'file'
  fileId?: number
  localKey?: string
  name: string
  mimeType: string
  sizeBytes: number
}

interface PersistedDraft {
  version: 4 | 5
  source: string
  notes: string
  notesOpen: boolean
  lines: Array<{
    lineId: string
    item: ItemOptionResponse
    quantity: number
    unitPrice: number
    locationId: number | null
    batchNo: string
    expiresAt: string
    extAttributes: Record<string, string | number | boolean | PersistedFileValue>
    templateId: number | null
    recommendedTemplateId?: number | null
    templateSource?: InboundDraftLine['templateSource']
  }>
}

/** 绑定页面草稿引用；调用 restore 后再 resume，避免恢复过程中覆盖本地记录。 */
export function useInboundDraftPersistence(
  source: Ref<string>, notes: Ref<string>, notesOpen: Ref<boolean>,
  lines: Ref<InboundDraftLine[]>, hasDraft: ComputedRef<boolean>,
) {
  let suspended = true
  let imageSaveTimer: number | undefined
  watch([source, notes, notesOpen, lines], save, { deep: true })
  onMounted(() => window.addEventListener('beforeunload', handleBeforeUnload))
  onBeforeUnmount(() => {
    window.clearTimeout(imageSaveTimer)
    window.removeEventListener('beforeunload', handleBeforeUnload)
  })

  function resume(): void { suspended = false; save() }
  function remove(): void {
    window.clearTimeout(imageSaveTimer)
    localStorage.removeItem(storageKey)
    localStorage.removeItem(legacyStorageKey)
    void clearInboundDraftImages()
  }

  async function restore(): Promise<boolean> {
    localStorage.removeItem(obsoleteStorageKey)
    const raw = localStorage.getItem(storageKey) ?? localStorage.getItem(legacyStorageKey)
    if (!raw) return false
    try {
      const draft = JSON.parse(raw) as PersistedDraft
      if (![4, 5].includes(draft.version) || !Array.isArray(draft.lines)) throw new Error('invalid draft')
      source.value = typeof draft.source === 'string' ? draft.source : ''
      notes.value = typeof draft.notes === 'string' ? draft.notes : ''
      notesOpen.value = Boolean(draft.notesOpen || notes.value)
      lines.value = await Promise.all(draft.lines.map(async (line) => ({
        lineId: line.lineId || createLineId(), item: line.item, quantity: line.quantity,
        unitPrice: line.unitPrice, locationId: line.locationId, batchNo: line.batchNo || '',
        expiresAt: line.expiresAt || '', extAttributes: await restoreAttributes(line.lineId, line.extAttributes),
        template: null,
        templateId: line.templateId,
        recommendedTemplateId: line.recommendedTemplateId ?? line.item.recommended_inbound_template_id ?? null,
        templateSource: line.templateSource ?? (line.templateId === null ? 'none' : 'manual'),
        templateState: line.templateId === null ? 'idle' : 'resolving',
        templateError: '',
      })))
      return true
    } catch {
      remove()
      return false
    }
  }

  function save(): void {
    if (suspended) return
    if (!hasDraft.value) { remove(); return }
    const localImages = new Map<string, File>()
    const draft: PersistedDraft = {
      version: 5, source: source.value, notes: notes.value, notesOpen: notesOpen.value,
      lines: lines.value.map((line) => ({
        lineId: line.lineId, item: line.item, quantity: line.quantity, unitPrice: line.unitPrice,
        locationId: line.locationId, batchNo: line.batchNo, expiresAt: line.expiresAt,
        extAttributes: persistAttributes(line.lineId, line.extAttributes, localImages),
        templateId: line.templateId,
        recommendedTemplateId: line.recommendedTemplateId,
        templateSource: line.templateSource,
      })),
    }
    try {
      localStorage.setItem(storageKey, JSON.stringify(draft))
      localStorage.removeItem(legacyStorageKey)
    } catch { /* 配额失败不阻断当前录入。 */ }
    window.clearTimeout(imageSaveTimer)
    imageSaveTimer = window.setTimeout(() => {
      void replaceInboundDraftImages(localImages).catch(() => undefined)
    }, 200)
  }

  function handleBeforeUnload(event: BeforeUnloadEvent): void {
    if (!hasDraft.value) return
    event.preventDefault()
    event.returnValue = ''
  }

  return { restoreDraft: restore, resumeDraftSaving: resume, removePersistedDraft: remove }
}

function persistAttributes(
  lineId: string,
  attributes: Record<string, AttributeValue>,
  localImages: Map<string, File>,
): PersistedDraft['lines'][number]['extAttributes'] {
  const result: PersistedDraft['lines'][number]['extAttributes'] = {}
  for (const [name, value] of Object.entries(attributes)) {
    if (typeof value === 'object' && value?.kind === 'file') {
      const localKey = value.localFile ? `${lineId}:${name}` : undefined
      if (localKey && value.localFile) localImages.set(localKey, value.localFile)
      if (value.fileId || localKey) result[name] = {
        kind: 'file', fileId: value.fileId, localKey,
        name: value.name, mimeType: value.mimeType, sizeBytes: value.sizeBytes,
      }
    } else if (value !== undefined) result[name] = value as string | number | boolean
  }
  return result
}

async function restoreAttributes(
  lineId: string,
  attributes: PersistedDraft['lines'][number]['extAttributes'],
): Promise<Record<string, AttributeValue>> {
  const result: Record<string, AttributeValue> = {}
  for (const [name, value] of Object.entries(attributes ?? {})) {
    if (!isPersistedFileValue(value)) { result[name] = value as string | number | boolean; continue }
    if (value.fileId) {
      result[name] = { kind: 'file', fileId: value.fileId, name: value.name, mimeType: value.mimeType, sizeBytes: value.sizeBytes, status: 'uploaded', progress: 100, error: '' } satisfies FileDraftValue
      continue
    }
    const file = await readInboundDraftImage(value.localKey ?? `${lineId}:${name}`)
    result[name] = file
      ? createPendingImageDraft(file)
      : { kind: 'file', name: value.name, mimeType: value.mimeType, sizeBytes: value.sizeBytes, status: 'failed', progress: 0, error: '本地草稿图片已丢失，请重新选择' } satisfies FileDraftValue
  }
  return result
}

function isPersistedFileValue(value: unknown): value is PersistedFileValue {
  return typeof value === 'object' && value !== null && 'kind' in value && value.kind === 'file' &&
    (('fileId' in value && typeof value.fileId === 'number') || ('localKey' in value && typeof value.localKey === 'string')) &&
    'name' in value && typeof value.name === 'string' &&
    'mimeType' in value && typeof value.mimeType === 'string' && 'sizeBytes' in value && typeof value.sizeBytes === 'number'
}
