<!--
  本文件拥有桌面端多明细入库工作台，属于 frontend 页面层。
  它管理本地草稿、动态模板、临时图片和 pending 单据创建，不拥有审批或成功后详情跳转。
-->
<template>
  <section class="route-page inbound-draft-page">
    <header class="content-header inbound-draft-page__header">
      <h1>新建入库</h1>
      <div class="inbound-page-actions">
        <span>{{ draftItems.length }} 条明细</span>
        <span>{{ formatQuantity(draftQuantity) }} 件</span>
        <strong>预计金额 {{ formatMoney(draftTotal) }}</strong>
        <button class="secondary-button" type="button" :disabled="!hasDraft || submitting" @click="openClearConfirmation">
          清空
        </button>
        <button class="primary-button" type="button" :disabled="submitting" @click="reviewDraft">
          {{ submitting ? '提交中…' : '提交入库单' }}
        </button>
      </div>
    </header>
    <div class="inbound-draft-page__mobile-note">当前批量入库工作台仅在桌面宽度下提供。</div>

    <div class="inbound-workbench">
      <section class="inbound-catalog" aria-labelledby="inbound-catalog-title">
        <header class="inbound-panel-header">
          <div>
            <h2 id="inbound-catalog-title">物品列表</h2>
            <p>{{ itemResultLabel }}</p>
          </div>
        </header>

        <form class="inbound-search" role="search" @submit.prevent="submitSearch">
          <label for="inbound-item-search">搜索物品</label>
          <div>
            <input
              id="inbound-item-search"
              ref="itemSearchInput"
              v-model="searchInput"
              type="search"
              placeholder="名称、SKU 或模板属性"
              autocomplete="off"
              @input="handleSearchInput"
              @search="handleSearchInput"
            />
            <button class="secondary-button" type="submit" :disabled="loadingItems">查询</button>
          </div>
        </form>

        <div v-if="itemError && items.length === 0" class="inbound-panel-state inbound-panel-state--error" role="alert">
          <p>{{ itemError }}</p>
          <button class="text-button" type="button" @click="resetItems">重试</button>
        </div>
        <div v-else-if="loadingItems && items.length === 0" class="inbound-panel-state" role="status">正在加载物品…</div>
        <div v-else-if="items.length === 0" class="inbound-panel-state">没有找到可加入入库单的物品。</div>

        <div
          v-else
          :ref="setItemList"
          class="inbound-item-list"
          aria-label="可选物品"
          @scroll.passive="handleItemScroll"
        >
          <article
            v-for="item in items"
            :key="item.id"
            class="inbound-item-card"
            :class="{ 'inbound-item-card--added': draftItemIds.has(item.id) }"
            draggable="true"
            @dragstart="startDragging($event, item)"
            @dragend="draggingItemId = null"
            @dblclick="addItem(item)"
          >
            <div class="inbound-item-card__identity">
              <span class="inbound-item-card__mark" aria-hidden="true">{{ itemInitial(item) }}</span>
              <div>
                <strong :title="item.name">{{ item.name }}</strong>
                <span>{{ item.sku }} · {{ item.unit }}</span>
              </div>
            </div>
          </article>
          <div v-if="loadingItems" class="inbound-panel-state" role="status">正在加载更多物品…</div>
          <div v-else-if="itemError" class="inbound-panel-state inbound-panel-state--error" role="alert">
            <p>{{ itemError }}</p>
            <button class="text-button" type="button" @click="loadNextItems">重试本页</button>
          </div>
          <div v-else-if="itemsExhausted" class="inbound-panel-state">已加载全部物品</div>
        </div>
      </section>

      <section
        class="inbound-order"
        :class="{ 'inbound-order--drag-active': draggingItemId !== null }"
        aria-labelledby="inbound-order-title"
        @dragover.prevent
        @drop="dropItem"
      >
        <header class="inbound-panel-header inbound-order__header">
          <div>
            <h2 id="inbound-order-title">入库明细</h2>
            <p>{{ draftItems.length }} 条明细</p>
          </div>
        </header>

        <div class="inbound-order__body">
          <section class="inbound-order-meta" aria-label="入库单基础信息">
            <label class="inbound-order-meta__source">
              <span>来源 *</span>
              <input
                ref="sourceInput"
                v-model="source"
                :class="{ 'inbound-control--error': validationAttempted && !source.trim() }"
                :title="validationAttempted && !source.trim() ? '请填写入库来源' : undefined"
                type="text"
                maxlength="128"
                placeholder="供应商名称或采购单号"
              />
            </label>
            <button
              class="inbound-order-meta__notes-toggle"
              :class="{ 'inbound-order-meta__notes-toggle--filled': notes.trim().length > 0 }"
              type="button"
              :aria-expanded="notesOpen"
              aria-controls="inbound-order-notes"
              @click="notesOpen = !notesOpen"
            >
              {{ notesOpen ? '收起备注' : notes.trim() ? '备注 · 已填写' : '添加备注' }}
            </button>
            <label v-if="notesOpen" id="inbound-order-notes" class="inbound-order-meta__notes">
              <span>备注</span>
              <input v-model="notes" type="text" maxlength="1024" placeholder="可选，记录采购或收货说明" />
            </label>
          </section>

          <div v-if="locationError" class="inbound-location-error" role="alert">
            {{ locationError }}
            <button class="text-button" type="button" @click="loadLocationOptions">重试</button>
          </div>

          <div v-if="draftItems.length === 0" class="inbound-drop-zone">
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M4 7.5 12 3l8 4.5v9L12 21l-8-4.5v-9Z M4 7.5l8 4.5 8-4.5 M12 12v9" />
            </svg>
            <strong>拖动物品到这里</strong>
            <span>同一物品可重复添加为不同批次，也可以双击左侧物品。</span>
          </div>

          <div v-else class="inbound-detail-workspace">
            <section class="inbound-lines" aria-label="入库明细">
              <table>
                <thead>
                  <tr><th>物品</th><th>数量</th><th>单价</th><th>库位</th><th>小计</th><th><span class="visually-hidden">操作</span></th></tr>
                </thead>
                <tbody>
                  <tr
                    v-for="line in draftItems"
                    :key="line.lineId"
                    :class="{ 'inbound-line--selected': selectedLineId === line.lineId }"
                    @click="selectLine(line.lineId)"
                  >
                    <td><strong>{{ line.item.name }}</strong><span>{{ line.item.sku }} · {{ line.item.unit }}</span></td>
                    <td>
                      <input
                        v-model.number="line.quantity"
                        :data-line-id="line.lineId"
                        data-field="quantity"
                        :class="{ 'inbound-control--error': validationAttempted && !validQuantity(line.quantity) }"
                        :title="validationAttempted && !validQuantity(line.quantity) ? '入库数量必须大于 0' : undefined"
                        type="number" min="0.01" step="0.01" aria-label="入库数量"
                      />
                    </td>
                    <td>
                      <input
                        v-model.number="line.unitPrice"
                        :data-line-id="line.lineId"
                        data-field="unitPrice"
                        :class="{ 'inbound-control--error': validationAttempted && !validUnitPrice(line.unitPrice) }"
                        :title="validationAttempted && !validUnitPrice(line.unitPrice) ? '入库单价不能小于 0' : undefined"
                        type="number" min="0" step="0.01" aria-label="入库单价"
                      />
                    </td>
                    <td>
                      <select
                        v-model="line.locationId"
                        :data-line-id="line.lineId"
                        data-field="locationId"
                        :class="{ 'inbound-control--error': validationAttempted && line.locationId === null }"
                        :title="validationAttempted && line.locationId === null ? '请选择入库库位' : undefined"
                        aria-label="入库库位"
                      >
                        <option :value="null">请选择</option>
                        <option v-for="location in locations" :key="location.id" :value="location.id">{{ location.code }} · {{ location.name }}</option>
                      </select>
                    </td>
                    <td class="inbound-line__subtotal">{{ formatMoney(lineSubtotal(line)) }}</td>
                    <td><button class="inbound-line__remove" type="button" :aria-label="`移除 ${line.item.name}`" @click.stop="removeLine(line.lineId)">×</button></td>
                  </tr>
                </tbody>
              </table>
            </section>

            <InboundLineEditor
              v-if="selectedLine"
              :line="selectedLine"
              :templates="inboundTemplates"
              :validation-attempted="validationAttempted"
              @select-template="selectInboundTemplate"
              @retry-template="retryLineTemplate"
            />
          </div>
        </div>
      </section>
    </div>

    <ModalDialog
      :open="confirmationMode !== null"
      :title="confirmationMode === 'clear' ? '清空入库草稿？' : '离开当前页面？'"
      :description="confirmationMode === 'clear' ? '所有未提交明细和未绑定图片都会被删除。' : '当前草稿已自动保存在本机，离开后仍可恢复。'"
      :busy="clearingDraft"
      @close="cancelConfirmation"
    >
      <p>{{ confirmationMode === 'clear' ? '此操作无法撤销。' : '确认离开当前入库工作台吗？' }}</p>
      <template #actions>
        <button class="secondary-button" type="button" :disabled="clearingDraft" @click="cancelConfirmation">取消</button>
        <button class="primary-button" type="button" :disabled="clearingDraft" @click="confirmCurrentAction">
          {{ clearingDraft ? '处理中…' : confirmationMode === 'clear' ? '确认清空' : '确认离开' }}
        </button>
      </template>
    </ModalDialog>
  </section>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { onBeforeRouteLeave } from 'vue-router'
import ModalDialog from '../components/ModalDialog.vue'
import InboundLineEditor from '../components/inbound/InboundLineEditor.vue'
import { createInbound, listLocations, type LocationResponse } from '../api/inbound'
import { getItemAttributeTemplate, type ItemAttributeTemplateResponse } from '../api/itemAttributeTemplates'
import { getInboundTemplate, listInboundTemplates, type InboundTemplateResponse } from '../api/inboundTemplates'
import type { ItemResponse } from '../api/items'
import { deleteImage } from '../api/files'
import { ApiError } from '../api/errors'
import { useInboundDraftPersistence } from '../composables/useInboundDraftPersistence'
import { useInboundItemCatalog } from '../composables/useInboundItemCatalog'
import { notice } from '../notices/notice'
import {
  buildInboundRequest, createDraftLine, lineReady, lineSubtotal, positiveNumber,
  revokeLinePreviews, templateFieldError, validQuantity, validUnitPrice,
  type FileDraftValue, type InboundDraftLine,
} from './inbound-draft/model'
import {
  formatMoney, formatQuantity, inboundSubmitErrorMessage, isAbortError, itemErrorMessage, itemInitial,
} from './inbound-draft/presentation'

type ConfirmationMode = 'clear' | 'leave' | null
const itemSearchInput = ref<HTMLInputElement | null>(null)
const draggingItemId = ref<number | null>(null)
const draftItems = ref<InboundDraftLine[]>([])
const locations = ref<LocationResponse[]>([])
const inboundTemplates = ref<InboundTemplateResponse[]>([])
const locationError = ref('')
const source = ref('')
const sourceInput = ref<HTMLInputElement | null>(null)
const notes = ref('')
const notesOpen = ref(false)
const selectedLineId = ref<string | null>(null)
const submitting = ref(false)
const validationAttempted = ref(false)
const confirmationMode = ref<ConfirmationMode>(null)
const clearingDraft = ref(false)
let locationAbortController: AbortController | null = null
let pendingLeaveResolution: ((allowed: boolean) => void) | null = null
const templateAbortControllers = new Map<string, AbortController>()
const templateCache = new Map<number, InboundTemplateResponse>()
const itemTemplateCache = new Map<number, ItemAttributeTemplateResponse>()

const {
  items, searchInput, loadingItems, itemError, itemList, itemsExhausted,
  itemResultLabel, resetItems, loadNextItems, submitSearch, handleSearchInput, handleItemScroll,
} = useInboundItemCatalog((error) => itemErrorMessage(error))

function setItemList(element: unknown): void {
  itemList.value = element instanceof HTMLElement ? element : null
}

const draftItemIds = computed(() => new Set(draftItems.value.map((line) => line.item.id)))
const selectedLine = computed(() => draftItems.value.find((line) => line.lineId === selectedLineId.value) ?? null)
const draftQuantity = computed(() => draftItems.value.reduce((total, line) => total + positiveNumber(line.quantity), 0))
const draftTotal = computed(() => draftItems.value.reduce((total, line) => total + lineSubtotal(line), 0))
const hasDraft = computed(() => source.value.trim().length > 0 || notes.value.trim().length > 0 || draftItems.value.length > 0)
const draftReady = computed(() => source.value.trim().length > 0 && draftItems.value.length > 0 && draftItems.value.every(lineReady))
const { restoreDraft, resumeDraftSaving, removePersistedDraft } = useInboundDraftPersistence(
  source, notes, notesOpen, draftItems, hasDraft,
)

onMounted(() => {
  const restored = restoreDraft()
  selectedLineId.value = draftItems.value[0]?.lineId ?? null
  draftItems.value.forEach((line) => { if (line.templateId) void loadLineTemplate(line, line.templateId) })
  if (restored) notice.info('已恢复上次未提交的入库草稿')
  resumeDraftSaving()
  void resetItems()
  void loadLocationOptions()
  void loadInboundTemplateOptions()
})

onBeforeUnmount(() => {
  locationAbortController?.abort()
  templateAbortControllers.forEach((controller) => controller.abort())
  draftItems.value.forEach(revokeLinePreviews)
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

function startDragging(event: DragEvent, item: ItemResponse): void {
  draggingItemId.value = item.id
  event.dataTransfer?.setData('text/plain', String(item.id))
  if (event.dataTransfer) event.dataTransfer.effectAllowed = 'copy'
}

function dropItem(event: DragEvent): void {
  const itemId = Number(event.dataTransfer?.getData('text/plain'))
  draggingItemId.value = null
  const item = items.value.find((candidate) => candidate.id === itemId)
  if (item) addItem(item)
}

/** 每次添加都创建独立 lineId，同一物品可分别维护多个批次和库位。 */
function addItem(item: ItemResponse): void {
  const line = createDraftLine(item)
  draftItems.value.push(line)
  selectedLineId.value = line.lineId
  void loadDefaultInboundTemplate(line)
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
  if (selectedLineId.value === lineId) selectedLineId.value = draftItems.value[0]?.lineId ?? null
}

function selectLine(lineId: string): void { selectedLineId.value = lineId }

async function reviewDraft(): Promise<void> {
  validationAttempted.value = true
  if (!draftReady.value) {
    notice.warning('入库单信息尚未填写完整', { detail: draftBlockingReason() })
    await focusFirstError()
    return
  }
  submitting.value = true
  try {
    const created = await createInbound(buildInboundRequest(source.value, notes.value, draftItems.value))
    notice.success('入库单已提交', { detail: `单号 #${created.id} 已进入待审批状态。` })
    clearLocalDraftState()
  } catch (error) {
    const message = inboundSubmitErrorMessage(error)
    notice.error(message.title, { detail: message.detail })
    await focusBackendError(error)
  } finally {
    submitting.value = false
  }
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
  if (!source.value.trim()) { sourceInput.value?.focus(); return }
  if (draftItems.value.length === 0) { itemSearchInput.value?.focus(); return }
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
  selectedLineId.value = line.lineId
  await nextTick()
  document.querySelector<HTMLElement>(`[data-line-id="${line.lineId}"][data-field="${field}"]`)?.focus()
}

async function focusLineTemplate(line: InboundDraftLine, fieldName?: string): Promise<void> {
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
</script>

<style lang="scss" src="./InboundDraftPage.scss"></style>
