<!--
  本文件拥有物品管理页面的数据加载、分页目录、草稿切换和保存编排，属于 frontend 页面层。
  它只通过 HTTP API 管理物品，不直接访问数据库或后端内部对象。
-->
<template>
  <section
    class="route-page items-page"
  >
    <header class="content-header items-page__header">
      <div>
        <h1>{{ $route.meta.title }}</h1>
        <p>维护入库、出库和库存统计使用的物品资料。</p>
      </div>
    </header>

    <div class="items-page__workspace">
      <aside class="items-catalog" aria-label="物品目录">
        <div class="items-catalog__toolbar">
          <label class="items-catalog__search">
            <span>搜索物品</span>
            <span class="items-catalog__search-control">
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <circle cx="11" cy="11" r="6.5" />
                <path d="m16 16 4 4" />
              </svg>
              <input
                v-model="searchInput"
                name="item_search"
                type="search"
                maxlength="128"
                placeholder="名称、SKU 或属性"
                @input="scheduleSearch"
              />
            </span>
          </label>
          <div class="items-catalog__summary">
            <span class="items-catalog__summary-content">
              <span>{{ catalogSummary }}</span>
              <Transition name="items-catalog-refresh-status">
                <span
                  v-if="showStableCatalogLoading && items.length > 0"
                  class="items-catalog__refresh-status"
                  role="status"
                >
                  正在刷新
                </span>
              </Transition>
            </span>
            <div class="items-catalog__actions">
              <button
                class="icon-button items-catalog__refresh"
                :class="{ 'items-catalog__refresh--pending': showStableCatalogLoading }"
                type="button"
                title="刷新物品目录"
                aria-label="刷新物品目录"
                :disabled="catalogPending"
                @click="requestRefreshCatalog"
              >
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  <path d="M20 7v5h-5" />
                  <path d="M18.2 16a7 7 0 1 1 .8-7l1 3" />
                </svg>
              </button>
              <button
                class="icon-button items-catalog__create"
                type="button"
                title="新建物品"
                aria-label="新建物品"
                @click="requestStartNew"
              >
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  <path d="M12 5v14M5 12h14" />
                </svg>
              </button>
            </div>
          </div>
        </div>

        <div
          class="items-catalog__body"
          :class="{ 'items-catalog__body--refreshing': showStableCatalogLoading && items.length > 0 }"
          :aria-busy="catalogPending"
        >
          <div v-if="loadError && items.length === 0" class="items-catalog__state items-catalog__state--error" role="alert">
            <strong>无法加载物品</strong>
            <span>{{ loadError }}</span>
            <button class="secondary-button" type="button" @click="reloadCatalog">重试</button>
          </div>

          <div v-else-if="showCatalogLoadingState" class="items-catalog__state" aria-live="polite">
            <span v-if="showStableCatalogLoading" role="status">正在加载物品…</span>
          </div>

          <div v-else-if="items.length === 0" class="items-catalog__state">
            <strong>{{ activeSearch ? '没有符合条件的物品' : '还没有物品' }}</strong>
            <button v-if="activeSearch" class="text-button" type="button" @click="clearSearch">清除搜索</button>
          </div>

          <template v-else>
            <div class="items-catalog__list">
              <article
                v-for="item in items"
                :key="item.id"
                class="items-catalog__item"
                :class="{ 'items-catalog__item--selected': editorOpen && draft.id === item.id }"
              >
                <AuthenticatedImage
                  :file-id="item.image_file_id"
                  :alt="`${item.name} 主图`"
                  :size="48"
                  previewable
                />
                <button
                  class="items-catalog__item-main"
                  type="button"
                  :aria-current="editorOpen && draft.id === item.id ? 'true' : undefined"
                  :aria-label="`编辑物品：${item.name}`"
                  @click="requestEditItem(item)"
                >
                  <span class="items-catalog__identity">
                    <strong>{{ item.name }}</strong>
                    <span>{{ item.sku }} · {{ item.unit }}</span>
                    <small>{{ itemCategoryLabel(item) }} · {{ item.attributes.length }} 项属性</small>
                  </span>
                  <svg class="items-catalog__chevron" viewBox="0 0 24 24" aria-hidden="true">
                    <path d="m9 5 7 7-7 7" />
                  </svg>
                </button>
              </article>
            </div>

            <div ref="loadMoreSentinel" class="items-catalog__load-more" aria-live="polite">
              <span v-if="showStableLoadingMore" role="status">正在加载更多…</span>
              <template v-else-if="loadMoreError">
                <span>{{ loadMoreError }}</span>
                <button class="text-button" type="button" @click="loadNextPage">重试</button>
              </template>
              <button
                v-else-if="hasMoreItems"
                class="text-button"
                type="button"
                :disabled="loadingMore"
                @click="loadNextPage"
              >
                加载更多
              </button>
              <span v-else>已加载全部 {{ total }} 个物品</span>
            </div>
          </template>
        </div>
      </aside>

    </div>

    <ItemEditorDialog
      :open="editorOpen"
      :draft="draft"
      :categories="categories"
      :templates="templates"
      :saving="saving"
      :metadata-error="metadataError"
      :validation-errors="validationErrors"
      @save="save"
      @close="requestCloseEditor"
    />

    <ModalDialog
      :open="discardDialogOpen"
      title="放弃未保存的修改？"
      description="当前物品草稿中的修改不会保留。"
      @close="cancelPendingTransition"
    >
      <p>确认后将继续刚才的操作。</p>
      <template #actions>
        <button class="secondary-button" type="button" @click="cancelPendingTransition">继续编辑</button>
        <button class="danger-button" type="button" @click="confirmPendingTransition">放弃修改</button>
      </template>
    </ModalDialog>
  </section>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { onBeforeRouteLeave } from 'vue-router'
import { createItem, listItems, updateItem, type ItemResponse } from '../api/items'
import { listItemCategories, type ItemCategoryResponse } from '../api/itemCategories'
import { listItemAttributeTemplates, type ItemAttributeTemplateResponse } from '../api/itemAttributeTemplates'
import ItemEditorDialog from '../components/items/ItemEditorDialog.vue'
import AuthenticatedImage from '../components/attributes/AuthenticatedImage.vue'
import ModalDialog from '../components/ModalDialog.vue'
import { ApiError } from '../api/errors'
import { notice } from '../notices/notice'
import {
  draftFromItem, emptyItemDraft, itemCreateRequest, itemDraftFingerprint, itemUpdateRequest,
  itemDraftValidationFromApiError, validateItemDraft, type ItemDraft,
} from './items/model'
import { discardTemporaryItemFiles } from './items/fileCleanup'
import { isImageDraftValue, uploadImageDrafts } from '../components/attributes/imageDraft'
import { deleteImage } from '../api/files'
import { useStablePendingIndicator } from '../composables/useStablePendingIndicator'
import { useFormValidation } from '../composables/useFormValidation'
import './ItemsPage.scss'

const PAGE_SIZE = 50

const items = ref<ItemResponse[]>([])
const categories = ref<ItemCategoryResponse[]>([])
const templates = ref<ItemAttributeTemplateResponse[]>([])
const draft = ref<ItemDraft>(emptyItemDraft())
const searchInput = ref('')
const activeSearch = ref('')
const total = ref(0)
const page = ref(1)
const totalPages = ref(0)
const loading = ref(true)
const loadingMore = ref(false)
const saving = ref(false)
const loadError = ref('')
const loadMoreError = ref('')
const metadataError = ref('')
const editorOpen = ref(false)
const discardDialogOpen = ref(false)
const loadMoreSentinel = ref<HTMLElement | null>(null)
const baselineFingerprint = ref('')
const baselineDraft = ref<ItemDraft>(emptyItemDraft())
const validationErrors = ref<Record<string, string>>({})
useFormValidation(validationErrors)
const emptyCatalogLoadingGate = ref(true)

let searchTimer: number | undefined
let catalogAbortController: AbortController | null = null
let loadMoreObserver: IntersectionObserver | null = null
let pendingTransition: (() => Promise<void>) | null = null
let pendingLeaveResolution: ((allowed: boolean) => void) | null = null

const hasMoreItems = computed(() => page.value < totalPages.value)
const catalogPending = computed(() => loading.value || loadingMore.value)
const showStableCatalogLoading = useStablePendingIndicator(loading, {
  showDelayMs: 200,
  minimumVisibleMs: 350,
})
const showStableLoadingMore = useStablePendingIndicator(loadingMore, {
  showDelayMs: 200,
  minimumVisibleMs: 350,
})
const showCatalogLoadingState = computed(() =>
  emptyCatalogLoadingGate.value || (showStableCatalogLoading.value && items.value.length === 0),
)
const hasUnsavedChanges = computed(() => itemDraftFingerprint(draft.value) !== baselineFingerprint.value)
const categoriesById = computed(() => new Map(categories.value.map((category) => [category.id, category.name])))
const catalogSummary = computed(() => {
  if (showCatalogLoadingState.value) return ''
  return activeSearch.value ? `${total.value} 个结果` : `${total.value} 个物品`
})

watch(loadMoreSentinel, (element, previousElement) => {
  if (previousElement) loadMoreObserver?.unobserve(previousElement)
  if (element) loadMoreObserver?.observe(element)
})

watch([loading, showStableCatalogLoading], ([pending, visible]) => {
  if (!pending && !visible) emptyCatalogLoadingGate.value = false
})

watch(() => itemDraftFingerprint(draft.value), () => {
  if (Object.keys(validationErrors.value).length > 0) validationErrors.value = {}
})

onMounted(() => {
  loadMoreObserver = new IntersectionObserver(handleLoadMoreIntersection, { rootMargin: '220px 0px' })
  if (loadMoreSentinel.value) loadMoreObserver.observe(loadMoreSentinel.value)
  window.addEventListener('beforeunload', handleBeforeUnload)
  void initializePage()
})

onBeforeUnmount(() => {
  window.clearTimeout(searchTimer)
  catalogAbortController?.abort()
  loadMoreObserver?.disconnect()
  window.removeEventListener('beforeunload', handleBeforeUnload)
  pendingLeaveResolution?.(false)
  pendingLeaveResolution = null
  void discardCurrentTemporaryFiles()
})

onBeforeRouteLeave(() => {
  if (!editorOpen.value || !hasUnsavedChanges.value) return true
  discardDialogOpen.value = true
  return new Promise<boolean>((resolve) => { pendingLeaveResolution = resolve })
})

async function initializePage(): Promise<void> {
  baselineFingerprint.value = itemDraftFingerprint(draft.value)
  await Promise.all([loadMetadata(), loadCatalog(1)])
}

/** 分类和模板属于编辑器元数据，只在页面初始化时加载，搜索物品不会重复请求。 */
async function loadMetadata(): Promise<void> {
  metadataError.value = ''
  try {
    const [nextCategories, nextTemplates] = await Promise.all([listItemCategories(), listItemAttributeTemplates()])
    categories.value = nextCategories
    templates.value = nextTemplates
  } catch (error) {
    metadataError.value = errorMessage(error)
    notice.error('物品编辑选项加载失败', { detail: metadataError.value })
  }
}

/** 查询指定目录页；新查询取消旧请求，追加时按 ID 合并以避免跨页重复。 */
async function loadCatalog(targetPage: number, append = false): Promise<void> {
  catalogAbortController?.abort()
  const controller = new AbortController()
  catalogAbortController = controller
  const shouldAppend = append && items.value.length > 0
  if (!shouldAppend && items.value.length === 0) emptyCatalogLoadingGate.value = true
  loading.value = !shouldAppend
  loadingMore.value = shouldAppend
  loadMoreError.value = ''
  if (!shouldAppend) loadError.value = ''

  try {
    const response = await listItems(activeSearch.value, targetPage, PAGE_SIZE, controller.signal)
    items.value = shouldAppend ? mergeItems(items.value, response.items) : response.items
    total.value = response.total
    page.value = response.page
    totalPages.value = response.total_pages
    await nextTick()
    refreshLoadMoreObservation()
  } catch (error) {
    if (error instanceof DOMException && error.name === 'AbortError') return
    const message = errorMessage(error)
    if (shouldAppend) loadMoreError.value = message
    else loadError.value = message
    notice.error(shouldAppend ? '加载更多物品失败' : '加载物品失败', { detail: message })
  } finally {
    if (catalogAbortController === controller) {
      catalogAbortController = null
      loading.value = false
      loadingMore.value = false
    }
  }
}

async function reloadCatalog(): Promise<void> {
  page.value = 1
  loadMoreError.value = ''
  await loadCatalog(1)
}

function scheduleSearch(): void {
  window.clearTimeout(searchTimer)
  searchTimer = window.setTimeout(() => {
    const nextSearch = searchInput.value.trim()
    if (nextSearch === activeSearch.value) return
    activeSearch.value = nextSearch
    void reloadCatalog()
  }, 280)
}

function clearSearch(): void {
  searchInput.value = ''
  activeSearch.value = ''
  void reloadCatalog()
}

function requestRefreshCatalog(): void {
  requestDraftTransition(refreshCatalog)
}

async function refreshCatalog(): Promise<void> {
  await discardCurrentTemporaryFiles()
  editorOpen.value = false
  draft.value = emptyItemDraft()
  baselineDraft.value = emptyItemDraft()
  baselineFingerprint.value = itemDraftFingerprint(draft.value)
  await reloadCatalog()
  if (!loadError.value) notice.success('物品目录已刷新')
}

async function loadNextPage(): Promise<void> {
  if (catalogPending.value || !hasMoreItems.value) return
  await loadCatalog(page.value + 1, true)
}

function handleLoadMoreIntersection(entries: IntersectionObserverEntry[]): void {
  if (entries.some((entry) => entry.isIntersecting)) void loadNextPage()
}

function refreshLoadMoreObservation(): void {
  const sentinel = loadMoreSentinel.value
  if (!sentinel || !loadMoreObserver) return
  loadMoreObserver.unobserve(sentinel)
  loadMoreObserver.observe(sentinel)
}

function requestStartNew(): void {
  requestDraftTransition(() => prepareNewDraft(true))
}

function requestEditItem(item: ItemResponse): void {
  if (draft.value.id === item.id) {
    editorOpen.value = true
    return
  }
  requestDraftTransition(() => editItem(item))
}

function requestCloseEditor(): void {
  requestDraftTransition(async () => {
    await clearCurrentSelection()
    editorOpen.value = false
  })
}

/** Dialog 关闭后清除编辑上下文，避免隐藏草稿继续驱动目录选中态。 */
async function clearCurrentSelection(): Promise<void> {
  await discardCurrentTemporaryFiles()
  draft.value = emptyItemDraft()
  baselineDraft.value = emptyItemDraft()
  baselineFingerprint.value = itemDraftFingerprint(draft.value)
}

function requestDraftTransition(action: () => Promise<void>): void {
  if (!hasUnsavedChanges.value) {
    void action()
    return
  }
  pendingTransition = action
  discardDialogOpen.value = true
}

function cancelPendingTransition(): void {
  discardDialogOpen.value = false
  pendingTransition = null
  pendingLeaveResolution?.(false)
  pendingLeaveResolution = null
}

async function confirmPendingTransition(): Promise<void> {
  const action = pendingTransition
  const resolveLeave = pendingLeaveResolution
  discardDialogOpen.value = false
  pendingTransition = null
  pendingLeaveResolution = null
  if (resolveLeave) {
    await clearCurrentSelection()
    editorOpen.value = false
    resolveLeave(true)
    return
  }
  if (action) await action()
}

async function prepareNewDraft(openEditor: boolean): Promise<void> {
  await discardCurrentTemporaryFiles()
  const next = emptyItemDraft()
  draft.value = next
  baselineDraft.value = emptyItemDraft()
  baselineFingerprint.value = itemDraftFingerprint(next)
  validationErrors.value = {}
  editorOpen.value = openEditor
}

async function editItem(item: ItemResponse): Promise<void> {
  await discardCurrentTemporaryFiles()
  draft.value = editorDraftFromItem(item)
  baselineDraft.value = editorDraftFromItem(item)
  baselineFingerprint.value = itemDraftFingerprint(draft.value)
  validationErrors.value = {}
  editorOpen.value = true
}

async function save(): Promise<void> {
  const validation = validateItemDraft(draft.value, templates.value)
  if (validation) {
    validationErrors.value = validation.errors
    notice.warning('请检查物品信息', { detail: validation.firstMessage })
    return
  }
  validationErrors.value = {}
  const mainImage = draft.value.image
  if (!mainImage) return
  saving.value = true
  try {
    await uploadImageDrafts([
      mainImage,
      ...draft.value.attributes.map((attribute) => attribute.value).filter(isImageDraftValue),
    ])
    const wasEditing = Boolean(draft.value.id)
    let saved: ItemResponse
    if (draft.value.id) {
      const request = itemUpdateRequest(draft.value, baselineDraft.value)
      if (Object.keys(request).length === 0) {
        await clearCurrentSelection()
        editorOpen.value = false
        return
      }
      saved = await updateItem(draft.value.id, request)
    } else {
      saved = await createItem(itemCreateRequest(draft.value))
    }
    if (draft.value.obsoleteImageFileId) {
      await deleteImage(draft.value.obsoleteImageFileId).catch(() => {
        notice.warning('旧物品主图未能立即清理', { detail: '服务会在超过保留期限后自动清理。' })
      })
    }
    draft.value.attributes.forEach((attribute) => { attribute.fileTemporary = false })
    draft.value.imageTemporary = false
    draft.value.obsoleteImageFileId = null
    draft.value = editorDraftFromItem(saved)
    baselineDraft.value = editorDraftFromItem(saved)
    baselineFingerprint.value = itemDraftFingerprint(draft.value)
    notice.success(wasEditing ? '物品已更新' : '物品已创建')
    await clearCurrentSelection()
    editorOpen.value = false
    await reloadCatalog()
  } catch (error) {
    if (error instanceof ApiError) {
      const apiValidation = itemDraftValidationFromApiError(error, draft.value)
      if (apiValidation) {
        validationErrors.value = apiValidation.errors
        notice.warning('请检查物品信息', { detail: apiValidation.firstMessage })
        return
      }
    }
    const imageError = [draft.value.image, ...draft.value.attributes.map((attribute) => attribute.value)]
      .find((value) => isImageDraftValue(value) && value.status === 'failed')
    notice.error(imageError ? '物品图片上传失败' : '保存物品失败', {
      detail: isImageDraftValue(imageError) ? imageError.error : errorMessage(error),
    })
  } finally {
    saving.value = false
  }
}

/** 打开编辑器时补齐当前模板的未持久化空字段，同时保持基线草稿一致。 */
function editorDraftFromItem(item: ItemResponse): ItemDraft {
  const template = templates.value.find((candidate) => candidate.id === item.attribute_template_id) ?? null
  return draftFromItem(item, template)
}

function itemCategoryLabel(item: ItemResponse): string {
  return item.category_id ? categoriesById.value.get(item.category_id) ?? '未知分类' : '未分类'
}

function errorMessage(error: unknown): string {
  return error instanceof ApiError ? error.message : '无法连接到 WineStock 服务'
}

async function discardCurrentTemporaryFiles(): Promise<void> {
  try {
    await discardTemporaryItemFiles(draft.value)
  } catch {
    notice.warning('部分临时图片未能立即删除', { detail: '服务会在超过保留期限后自动清理。' })
  }
}

function handleBeforeUnload(event: BeforeUnloadEvent): void {
  if (!editorOpen.value || !hasUnsavedChanges.value) return
  event.preventDefault()
  event.returnValue = ''
}

function mergeItems(currentItems: ItemResponse[], nextItems: ItemResponse[]): ItemResponse[] {
  const itemsById = new Map(currentItems.map((item) => [item.id, item]))
  nextItems.forEach((item) => itemsById.set(item.id, item))
  return Array.from(itemsById.values())
}

</script>
