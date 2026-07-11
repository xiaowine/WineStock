<!--
  本文件拥有可跨桌面与移动端使用的多明细入库工作台，属于 frontend 页面层。
  它管理本地草稿、动态模板、临时图片和入库单创建，不拥有后续审批或单据详情展示。
-->
<template>
  <section class="route-page inbound-draft-page">
    <header class="content-header inbound-draft-page__header">
      <div class="inbound-page-title">
        <div class="inbound-progress" aria-label="入库流程进度">
          <span :class="{ 'inbound-progress__step--active': currentStep === 'catalog' }">1</span>
          <i></i>
          <span :class="{ 'inbound-progress__step--active': currentStep === 'draft' }">2</span>
        </div>
        <div>
          <h1>新建入库</h1>
        </div>
      </div>
      <div v-if="currentStep === 'draft'" class="inbound-draft-summary" aria-label="当前入库草稿摘要">
        <span><strong>{{ draftItems.length }}</strong> 条明细</span>
        <span><strong>{{ quantitySummary }}</strong> {{ quantitySummaryLabel }}</span>
        <span><strong>¥{{ formatMoney(draftTotal) }}</strong> 预计金额</span>
      </div>
      <div class="inbound-page-actions">
        <button class="text-button inbound-clear-button" type="button" :disabled="!hasDraft || submitting" @click="openClearConfirmation">清空草稿</button>
        <template v-if="currentStep === 'draft'">
          <button v-if="canDirectInbound" class="secondary-button" type="button" :disabled="submitting" @click="reviewDraft('pending_approval')">提交审核</button>
          <button class="primary-button" type="button" :disabled="submitting" @click="reviewDraft(defaultSubmissionMode)">
            {{ submitting ? '正在提交…' : canDirectInbound ? '直接入库' : '提交审核' }}
          </button>
        </template>
      </div>
    </header>

    <InboundCatalogStep
      v-if="currentStep === 'catalog'"
      :items="items"
      :search-input="searchInput"
      :loading-items="loadingItems"
      :item-error="itemError"
      :items-exhausted="itemsExhausted"
      :draft-counts="draftItemCounts"
      :can-continue="draftItems.length > 0"
      @update:search-input="searchInput = $event"
      @search-input="handleSearchInput"
      @search="submitSearch"
      @reset-items="resetItems"
      @load-next-items="loadNextItems"
      @scroll-items="handleItemScroll"
      @list-element="itemList = $event"
      @toggle-item="toggleCatalogItem"
      @continue="openDraftStep"
    />

    <InboundDraftStep
      v-else
      :lines="draftItems"
      :locations="locations"
      :location-error="locationError"
      :source="source"
      :notes="notes"
      :notes-open="notesOpen"
      :validation-attempted="validationAttempted"
      :selected-line-id="selectedLineId"
      :drawer-open="selectedLine !== null"
      @update:source="source = $event"
      @update:notes="notes = $event"
      @update:notes-open="notesOpen = $event"
      @continue-adding="continueAddingItems"
      @retry-locations="loadLocationOptions"
      @select-line="selectLine"
      @remove-line="removeLine"
    >
      <Transition name="inbound-editor">
        <div v-if="selectedLine" class="inbound-line-editor-layer">
          <button class="inbound-line-editor-backdrop" type="button" aria-label="关闭当前明细详情" @click="closeLineEditor"></button>
          <InboundLineEditor
            :line="selectedLine"
            :templates="inboundTemplates"
            :validation-attempted="validationAttempted"
            @close="closeLineEditor"
            @select-template="selectInboundTemplate"
            @retry-template="retryLineTemplate"
          />
        </div>
      </Transition>
    </InboundDraftStep>

    <ModalDialog
      :open="confirmationMode !== null"
      :title="confirmationMode === 'clear' ? '清空入库草稿？' : '离开当前页面？'"
      :description="confirmationMode === 'clear' ? '所有未提交明细和未绑定图片都会被删除。' : '当前草稿已自动保存在本机，离开后仍可恢复。'"
      :busy="clearingDraft"
      @close="cancelConfirmation"
    >
      <p>{{ confirmationMode === 'clear' ? '此操作无法撤销。' : '确认离开当前入库流程吗？' }}</p>
      <template #actions>
        <button class="secondary-button" type="button" :disabled="clearingDraft" @click="cancelConfirmation">取消</button>
        <button class="primary-button" type="button" :disabled="clearingDraft" @click="confirmCurrentAction">
          {{ clearingDraft ? '处理中…' : confirmationMode === 'clear' ? '确认清空' : '确认离开' }}
        </button>
      </template>
    </ModalDialog>

    <ModalDialog
      :open="submissionConfirmationMode !== null"
      :title="submissionConfirmationMode === 'direct' ? '确认直接入库？' : '确认提交审核？'"
      :description="submissionConfirmationMode === 'direct' ? '提交后将立即增加库存并写入库存流水。' : '提交后单据进入待审批状态，审批通过前不会增加库存。'"
      :busy="submitting"
      @close="cancelSubmissionConfirmation"
    >
      <dl class="inbound-submit-summary">
        <div><dt>入库来源</dt><dd>{{ source.trim() }}</dd></div>
        <div><dt>明细数量</dt><dd>{{ draftItems.length }} 条</dd></div>
        <div><dt>入库总量</dt><dd>{{ quantitySummary }}</dd></div>
        <div><dt>预计金额</dt><dd>¥{{ formatMoney(draftTotal) }}</dd></div>
      </dl>
      <p v-if="submissionConfirmationMode === 'direct'" class="inbound-submit-warning">请确认库位、数量和单价无误。直接入库完成后应通过后续库存业务进行调整。</p>
      <template #actions>
        <button class="secondary-button" type="button" :disabled="submitting" @click="cancelSubmissionConfirmation">返回检查</button>
        <button class="primary-button" type="button" :disabled="submitting" @click="submitConfirmedDraft">
          {{ submitting ? '正在提交…' : submissionConfirmationMode === 'direct' ? '确认并入库' : '确认提交' }}
        </button>
      </template>
    </ModalDialog>
  </section>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { onBeforeRouteLeave } from 'vue-router'
import ModalDialog from '../components/ModalDialog.vue'
import InboundCatalogStep from '../components/inbound/InboundCatalogStep.vue'
import InboundDraftStep from '../components/inbound/InboundDraftStep.vue'
import InboundLineEditor from '../components/inbound/InboundLineEditor.vue'
import {
  createInbound, listLocations, type InboundSubmissionMode, type LocationResponse,
} from '../api/inbound'
import { getItemAttributeTemplate, type ItemAttributeTemplateResponse } from '../api/itemAttributeTemplates'
import { getInboundTemplate, listInboundTemplates, type InboundTemplateResponse } from '../api/inboundTemplates'
import type { ItemResponse } from '../api/items'
import { deleteImage } from '../api/files'
import { ApiError } from '../api/errors'
import { useInboundDraftPersistence } from '../composables/useInboundDraftPersistence'
import { useInboundItemCatalog } from '../composables/useInboundItemCatalog'
import { notice } from '../notices/notice'
import { authSession } from '../auth/session'
import { hasPermission, stockPermissions } from '../auth/permissions'
import { isImageDraftValue, uploadImageDrafts } from '../components/attributes/imageDraft'
import {
  buildInboundRequest, createDraftLine, lineReady, lineSubtotal, positiveNumber,
  revokeLinePreviews, templateFieldError, validQuantity, validUnitPrice,
  type FileDraftValue, type InboundDraftLine,
} from './inbound-draft/model'
import {
  formatMoney, formatQuantity, inboundSubmitErrorMessage, isAbortError, itemErrorMessage,
} from './inbound-draft/presentation'

type ConfirmationMode = 'clear' | 'leave' | null
type InboundDraftStepName = 'catalog' | 'draft'
const stepSessionKey = 'winestock.inbound.step'
const restoredNoticeSessionKey = 'winestock.inbound.restored-notice'
const currentStep = ref<InboundDraftStepName>(readSessionStep())
const draftItems = ref<InboundDraftLine[]>([])
const locations = ref<LocationResponse[]>([])
const inboundTemplates = ref<InboundTemplateResponse[]>([])
const locationError = ref('')
const source = ref('')
const notes = ref('')
const notesOpen = ref(false)
const selectedLineId = ref<string | null>(null)
const submitting = ref(false)
const validationAttempted = ref(false)
const confirmationMode = ref<ConfirmationMode>(null)
const clearingDraft = ref(false)
const submissionConfirmationMode = ref<InboundSubmissionMode | null>(null)
let locationAbortController: AbortController | null = null
let pendingLeaveResolution: ((allowed: boolean) => void) | null = null
const templateAbortControllers = new Map<string, AbortController>()
const templateCache = new Map<number, InboundTemplateResponse>()
const itemTemplateCache = new Map<number, ItemAttributeTemplateResponse>()

const {
  items, searchInput, loadingItems, itemError, itemList, itemsExhausted,
  resetItems, loadNextItems, submitSearch, handleSearchInput, handleItemScroll,
} = useInboundItemCatalog((error) => itemErrorMessage(error))

const draftItemCounts = computed(() => {
  const counts = new Map<number, number>()
  for (const line of draftItems.value) counts.set(line.item.id, (counts.get(line.item.id) ?? 0) + 1)
  return counts
})
const selectedLine = computed(() => draftItems.value.find((line) => line.lineId === selectedLineId.value) ?? null)
const draftQuantity = computed(() => draftItems.value.reduce((total, line) => total + positiveNumber(line.quantity), 0))
const draftTotal = computed(() => draftItems.value.reduce((total, line) => total + lineSubtotal(line), 0))
const quantitySummary = computed(() => {
  const units = new Set(draftItems.value.map((line) => line.item.unit).filter(Boolean))
  if (units.size === 1) return `${formatQuantity(draftQuantity.value)} ${Array.from(units)[0]}`
  return draftItems.value.length ? '按明细分别计量' : '0'
})
const quantitySummaryLabel = computed(() => quantitySummary.value === '按明细分别计量' ? '数量' : '入库总量')
const hasDraft = computed(() => source.value.trim().length > 0 || notes.value.trim().length > 0 || draftItems.value.length > 0)
const draftReady = computed(() => source.value.trim().length > 0 && draftItems.value.length > 0 && draftItems.value.every(lineReady))
const canDirectInbound = computed(() => hasPermission(authSession.value?.user.permissions, stockPermissions.inboundApprove))
const defaultSubmissionMode = computed<InboundSubmissionMode>(() => canDirectInbound.value ? 'direct' : 'pending_approval')
const { restoreDraft, resumeDraftSaving, removePersistedDraft } = useInboundDraftPersistence(
  source, notes, notesOpen, draftItems, hasDraft,
)

// 响应式 Shell 在断点切换时会重挂载页面，步骤必须在当前标签页内保持稳定。
watch(currentStep, (step) => sessionStorage.setItem(stepSessionKey, step))

onMounted(async () => {
  const restored = await restoreDraft()
  const removedDuplicates = removeRestoredDuplicateItems()
  // 恢复历史草稿只恢复数据，不主动进入任一明细的详情编辑模式。
  selectedLineId.value = null
  currentStep.value = draftItems.value.length > 0 ? readSessionStep() : 'catalog'
  draftItems.value.forEach((line) => { if (line.templateId) void loadLineTemplate(line, line.templateId) })
  if (restored && sessionStorage.getItem(restoredNoticeSessionKey) !== 'shown') {
    sessionStorage.setItem(restoredNoticeSessionKey, 'shown')
    notice.info('已恢复上次未提交的入库草稿')
  }
  if (removedDuplicates > 0) notice.info(`已移除 ${removedDuplicates} 条重复物品明细`)
  resumeDraftSaving()
  void resetItems()
  void loadLocationOptions()
  void loadInboundTemplateOptions()
  window.addEventListener('keydown', handlePageKeydown)
})

/** 兼容旧版草稿数据，恢复时按物品 ID 保留第一条明细并清理重复项资源。 */
function removeRestoredDuplicateItems(): number {
  const seen = new Set<number>()
  const unique: InboundDraftLine[] = []
  const duplicates: InboundDraftLine[] = []
  for (const line of draftItems.value) {
    if (seen.has(line.item.id)) duplicates.push(line)
    else {
      seen.add(line.item.id)
      unique.push(line)
    }
  }
  if (!duplicates.length) return 0
  duplicates.forEach((line) => {
    void deleteLineUploads(line)
    revokeLinePreviews(line)
  })
  draftItems.value = unique
  return duplicates.length
}

onBeforeUnmount(() => {
  locationAbortController?.abort()
  templateAbortControllers.forEach((controller) => controller.abort())
  draftItems.value.forEach(revokeLinePreviews)
  window.removeEventListener('keydown', handlePageKeydown)
})

onBeforeRouteLeave(() => {
  if (!hasDraft.value) return true
  confirmationMode.value = 'leave'
  return new Promise<boolean>((resolve) => { pendingLeaveResolution = resolve })
})

async function loadLocationOptions(): Promise<void> {
  locationAbortController?.abort()
  const controller = new AbortController()
  locationAbortController = controller
  locationError.value = ''
  try {
    locations.value = await listLocations(controller.signal)
  } catch (error) {
    if (!isAbortError(error)) locationError.value = itemErrorMessage(error, '加载库位失败')
  } finally {
    if (locationAbortController === controller) locationAbortController = null
  }
}

/** 选择阶段按物品去重，每个物品在当前入库单中只保留一条独立明细。 */
function addItem(item: ItemResponse): void {
  if (draftItems.value.some((line) => line.item.id === item.id)) return
  const line = createDraftLine(item)
  draftItems.value.push(line)
  // 添加物品只创建明细；详情编辑必须由用户通过“详情”按钮显式进入。
  selectedLineId.value = null
  void loadDefaultInboundTemplate(line)
  notice.info(`已加入 ${item.name}`)
}

function toggleCatalogItem(item: ItemResponse): void {
  const line = draftItems.value.find((candidate) => candidate.item.id === item.id)
  if (line) removeLine(line.lineId)
  else addItem(item)
}

async function loadInboundTemplateOptions(): Promise<void> {
  try {
    inboundTemplates.value = await listInboundTemplates()
  } catch (error) {
    notice.error('加载入库模板失败', { detail: itemErrorMessage(error) })
  }
}

async function loadDefaultInboundTemplate(line: InboundDraftLine): Promise<void> {
  const itemTemplateId = line.item.attribute_template_id
  if (!itemTemplateId) return
  try {
    let itemTemplate = itemTemplateCache.get(itemTemplateId)
    if (!itemTemplate) {
      itemTemplate = await getItemAttributeTemplate(itemTemplateId)
      itemTemplateCache.set(itemTemplateId, itemTemplate)
    }
    const defaultId = itemTemplate.default_inbound_template_id
    if (defaultId) {
      line.templateId = defaultId
      await loadLineTemplate(line, defaultId)
    }
  } catch (error) {
    line.templateError = itemErrorMessage(error, `无法加载 ${line.item.name} 的推荐入库模板`)
  }
}

async function loadLineTemplate(line: InboundDraftLine, templateId: number): Promise<void> {
  const cached = templateCache.get(templateId)
  if (cached) { applyTemplate(line, cached); return }
  templateAbortControllers.get(line.lineId)?.abort()
  const controller = new AbortController()
  templateAbortControllers.set(line.lineId, controller)
  line.templateLoading = true
  line.templateError = ''
  try {
    const template = await getInboundTemplate(templateId, controller.signal)
    templateCache.set(templateId, template)
    applyTemplate(line, template)
  } catch (error) {
    if (!isAbortError(error)) line.templateError = itemErrorMessage(error, `无法加载 ${line.item.name} 的模板`)
  } finally {
    if (templateAbortControllers.get(line.lineId) === controller) {
      templateAbortControllers.delete(line.lineId)
      line.templateLoading = false
    }
  }
}

function retryLineTemplate(line: InboundDraftLine): void {
  if (line.templateId !== null) void loadLineTemplate(line, line.templateId)
}

function applyTemplate(line: InboundDraftLine, template: InboundTemplateResponse): void {
  line.template = template
  for (const field of template.fields) {
    if (field.default_value !== null && line.extAttributes[field.field_name] === undefined && field.field_type !== 'file') {
      line.extAttributes[field.field_name] = field.field_type === 'number'
        ? Number(field.default_value)
        : field.field_type === 'boolean' ? field.default_value === 'true' : field.default_value
    }
  }
}

async function selectInboundTemplate(templateId: number | null): Promise<void> {
  const line = selectedLine.value
  if (!line) return
  await deleteLineUploads(line)
  line.templateId = templateId
  line.template = null
  line.templateError = ''
  line.extAttributes = {}
  if (templateId) void loadLineTemplate(line, templateId)
}

function removeLine(lineId: string): void {
  const line = draftItems.value.find((candidate) => candidate.lineId === lineId)
  if (!line) return
  templateAbortControllers.get(lineId)?.abort()
  templateAbortControllers.delete(lineId)
  void deleteLineUploads(line)
  revokeLinePreviews(line)
  draftItems.value = draftItems.value.filter((candidate) => candidate.lineId !== lineId)
  if (selectedLineId.value === lineId) selectedLineId.value = null
  notice.info(`已移除 ${line.item.name}`)
}

function openDraftStep(): void {
  if (draftItems.value.length === 0) return
  selectedLineId.value = null
  currentStep.value = 'draft'
}

function continueAddingItems(): void {
  selectedLineId.value = null
  currentStep.value = 'catalog'
  void nextTick(() => document.querySelector<HTMLElement>('[data-inbound-catalog-search]')?.focus())
}

function selectLine(lineId: string): void { selectedLineId.value = lineId }

function closeLineEditor(): void {
  const lineId = selectedLineId.value
  selectedLineId.value = null
  if (lineId) void nextTick(() => document.querySelector<HTMLElement>(`[data-line-action="${lineId}"]`)?.focus())
}

async function reviewDraft(submissionMode: InboundSubmissionMode): Promise<void> {
  validationAttempted.value = true
  if (!draftReady.value) {
    notice.warning('入库单信息尚未填写完整', { detail: draftBlockingReason() })
    await focusFirstError()
    return
  }
  submissionConfirmationMode.value = submissionMode
}

async function submitConfirmedDraft(): Promise<void> {
  const submissionMode = submissionConfirmationMode.value
  if (!submissionMode || submitting.value) return
  submitting.value = true
  try {
    await uploadImageDrafts(inboundDraftImages())
    const created = await createInbound(buildInboundRequest(source.value, notes.value, draftItems.value, submissionMode))
    if (created.submission_mode === 'direct') {
      notice.success('入库成功', { detail: `单号 #${created.id} 已完成入库，库存已更新。` })
    } else {
      notice.success('入库单已提交', { detail: `单号 #${created.id} 已进入待审批状态。` })
    }
    submissionConfirmationMode.value = null
    clearLocalDraftState()
  } catch (error) {
    const failedImage = firstFailedImage()
    if (failedImage) {
      notice.error('入库图片上传失败', { detail: failedImage.value.error })
      await focusLineTemplate(failedImage.line, failedImage.fieldName)
      return
    }
    const message = inboundSubmitErrorMessage(error)
    notice.error(message.title, { detail: message.detail })
    await focusBackendError(error)
  } finally {
    submitting.value = false
  }
}

function cancelSubmissionConfirmation(): void {
  if (!submitting.value) submissionConfirmationMode.value = null
}

function handlePageKeydown(event: KeyboardEvent): void {
  if (event.key !== 'Escape' || submissionConfirmationMode.value !== null || confirmationMode.value !== null) return
  if (selectedLineId.value) closeLineEditor()
}

function inboundDraftImages(): FileDraftValue[] {
  return draftItems.value.flatMap((line) => Object.values(line.extAttributes).filter(isImageDraftValue))
}

function firstFailedImage(): { line: InboundDraftLine; fieldName: string; value: FileDraftValue } | null {
  for (const line of draftItems.value) {
    for (const [fieldName, value] of Object.entries(line.extAttributes)) {
      if (isImageDraftValue(value) && value.status === 'failed') return { line, fieldName, value }
    }
  }
  return null
}

function openClearConfirmation(): void { if (hasDraft.value) confirmationMode.value = 'clear' }

function cancelConfirmation(): void {
  if (confirmationMode.value === 'leave') pendingLeaveResolution?.(false)
  pendingLeaveResolution = null
  confirmationMode.value = null
}

async function confirmCurrentAction(): Promise<void> {
  if (confirmationMode.value === 'leave') {
    const resolve = pendingLeaveResolution
    pendingLeaveResolution = null
    confirmationMode.value = null
    resolve?.(true)
    return
  }
  if (confirmationMode.value !== 'clear') return
  clearingDraft.value = true
  const lines = [...draftItems.value]
  await Promise.allSettled(lines.map(deleteLineUploads))
  lines.forEach(revokeLinePreviews)
  clearLocalDraftState()
  clearingDraft.value = false
  confirmationMode.value = null
}

function clearLocalDraftState(): void {
  templateAbortControllers.forEach((controller) => controller.abort())
  templateAbortControllers.clear()
  draftItems.value.forEach(revokeLinePreviews)
  source.value = ''
  notes.value = ''
  notesOpen.value = false
  draftItems.value = []
  selectedLineId.value = null
  currentStep.value = 'catalog'
  validationAttempted.value = false
  removePersistedDraft()
}

async function deleteLineUploads(line: InboundDraftLine): Promise<void> {
  const deletions = Object.values(line.extAttributes)
    .filter((value): value is FileDraftValue => typeof value === 'object' && value?.kind === 'file')
    .map((value) => { value.abortController?.abort(); return value.fileId ? deleteImage(value.fileId) : Promise.resolve() })
  await Promise.allSettled(deletions)
}

async function focusFirstError(): Promise<void> {
  if (draftItems.value.length === 0) {
    currentStep.value = 'catalog'
    await nextTick()
    document.querySelector<HTMLElement>('[data-inbound-catalog-search]')?.focus()
    return
  }
  currentStep.value = 'draft'
  selectedLineId.value = null
  await nextTick()
  if (!source.value.trim()) {
    document.querySelector<HTMLElement>('[data-inbound-source]')?.focus()
    return
  }
  for (const line of draftItems.value) {
    if (!validQuantity(line.quantity)) return focusLineControl(line, 'quantity')
    if (!validUnitPrice(line.unitPrice)) return focusLineControl(line, 'unitPrice')
    if (line.locationId === null) return focusLineControl(line, 'locationId')
    if (line.templateLoading || line.templateError) return focusLineTemplate(line)
    const field = line.template?.fields.find((candidate) => templateFieldError(line, candidate) !== null)
    if (field) return focusLineTemplate(line, field.field_name)
  }
}

async function focusLineControl(line: InboundDraftLine, field: string): Promise<void> {
  currentStep.value = 'draft'
  selectedLineId.value = null
  await nextTick()
  document.querySelector<HTMLElement>(`[data-line-id="${line.lineId}"][data-field="${field}"]`)?.focus()
}

async function focusLineTemplate(line: InboundDraftLine, fieldName?: string): Promise<void> {
  currentStep.value = 'draft'
  selectedLineId.value = line.lineId
  await nextTick()
  if (fieldName) document.querySelector<HTMLElement>(`[data-template-field="${CSS.escape(fieldName)}"]`)?.focus()
  else (document.querySelector<HTMLElement>('[data-template-retry]') ??
    document.querySelector<HTMLElement>('[data-template-picker]'))?.focus()
}

async function focusBackendError(error: unknown): Promise<void> {
  if (!(error instanceof ApiError) || !isRecord(error.details)) return
  const lineIndex = typeof error.details.line_index === 'number' ? error.details.line_index : -1
  const line = draftItems.value[lineIndex]
  if (!line) return
  validationAttempted.value = true
  currentStep.value = 'draft'
  if (error.code === 'location_not_found') await focusLineControl(line, 'locationId')
  else if (error.code === 'template_not_found') await focusLineTemplate(line)
  else if (error.code === 'invalid_inbound_field' || error.code === 'inbound_file_unavailable') {
    const fieldName = typeof error.details.field_name === 'string' ? error.details.field_name : undefined
    await focusLineTemplate(line, fieldName)
  } else { selectedLineId.value = line.lineId; await nextTick() }
}

function draftBlockingReason(): string {
  if (!source.value.trim()) return '请填写入库来源。'
  if (!draftItems.value.length) return '请至少添加一条入库明细。'
  const invalid = draftItems.value.find((line) => !lineReady(line))
  return invalid ? `请检查“${invalid.item.name}”的数量、单价、库位和模板属性。` : '请检查入库单信息。'
}

function isRecord(value: unknown): value is Record<string, unknown> { return typeof value === 'object' && value !== null }

function readSessionStep(): InboundDraftStepName {
  return sessionStorage.getItem(stepSessionKey) === 'draft' ? 'draft' : 'catalog'
}
</script>

<style lang="scss" src="./InboundDraftPage.scss"></style>
