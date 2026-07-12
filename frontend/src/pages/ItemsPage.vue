<!--
  本文件拥有物品管理页面的数据加载、分页目录、草稿切换和保存编排，属于 frontend 页面层。
  它只通过 HTTP API 管理物品，不直接访问数据库或后端内部对象。
-->
<template>
  <section class="route-page items-page" :class="{ 'items-page--editing': editorOpen }">
    <header class="content-header items-page__header">
      <div>
        <h1>物品</h1>
        <p>维护入库、出库和库存统计使用的物品资料。</p>
      </div>
      <button class="primary-button items-page__create" type="button" @click="requestStartNew">
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M12 5v14M5 12h14" />
        </svg>
        新建物品
      </button>
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
            <span>{{ catalogSummary }}</span>
            <div class="items-catalog__actions">
              <button
                class="icon-button"
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
              <button class="primary-button items-catalog__mobile-create" type="button" @click="requestStartNew">
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  <path d="M12 5v14M5 12h14" />
                </svg>
                新建
              </button>
            </div>
          </div>
        </div>

        <div class="items-catalog__body" :aria-busy="catalogPending">
          <div v-if="loading && items.length === 0" class="items-catalog__state" role="status">
            正在加载物品…
          </div>

          <div v-else-if="loadError && items.length === 0" class="items-catalog__state items-catalog__state--error" role="alert">
            <strong>无法加载物品</strong>
            <span>{{ loadError }}</span>
            <button class="secondary-button" type="button" @click="resetAndLoadCatalog">重试</button>
          </div>

          <div v-else-if="items.length === 0" class="items-catalog__state">
            <strong>{{ activeSearch ? '没有符合条件的物品' : '还没有物品' }}</strong>
            <button v-if="activeSearch" class="text-button" type="button" @click="clearSearch">清除搜索</button>
            <button v-else class="secondary-button" type="button" @click="requestStartNew">新建物品</button>
          </div>

          <template v-else>
            <div class="items-catalog__list">
              <button
                v-for="item in items"
                :key="item.id"
                class="items-catalog__item"
                :class="{ 'items-catalog__item--selected': draft.id === item.id }"
                type="button"
                :aria-current="draft.id === item.id ? 'true' : undefined"
                @click="requestEditItem(item)"
              >
                <AuthenticatedImage :file-id="item.image_file_id" :alt="`${item.name} 主图`" :size="48" />
                <span class="items-catalog__identity">
                  <strong>{{ item.name }}</strong>
                  <span>{{ item.sku }} · {{ item.unit }}</span>
                  <small>{{ itemCategoryLabel(item) }} · {{ item.attributes.length }} 项属性</small>
                </span>
                <svg class="items-catalog__chevron" viewBox="0 0 24 24" aria-hidden="true">
                  <path d="m9 5 7 7-7 7" />
                </svg>
              </button>
            </div>

            <div ref="loadMoreSentinel" class="items-catalog__load-more" aria-live="polite">
              <span v-if="loadingMore">正在加载更多…</span>
              <template v-else-if="loadMoreError">
                <span>{{ loadMoreError }}</span>
                <button class="text-button" type="button" @click="loadNextPage">重试</button>
              </template>
              <button v-else-if="hasMoreItems" class="text-button" type="button" @click="loadNextPage">加载更多</button>
              <span v-else>已加载全部 {{ total }} 个物品</span>
            </div>
          </template>
        </div>
      </aside>

      <Transition name="items-editor-panel">
        <div v-if="!editorOpen" class="items-editor-empty">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="m12 3 8 4.5v9L12 21l-8-4.5v-9L12 3Z" />
            <path d="m4.5 7.8 7.5 4.1 7.5-4.1M12 12v9" />
          </svg>
          <strong>选择物品进行编辑</strong>
        </div>

        <ItemEditor
          v-else
          :draft="draft"
          :categories="categories"
          :templates="templates"
          :saving="saving"
          :metadata-error="metadataError"
          @save="save"
          @close="requestCloseMobileEditor"
        />
      </Transition>
    </div>

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
import { createItem, listItems, updateItem, type ItemCreateRequest, type ItemResponse, type ItemUpdateRequest } from '../api/items'
import { listItemCategories, type ItemCategoryResponse } from '../api/itemCategories'
import { listItemAttributeTemplates, type ItemAttributeTemplateResponse } from '../api/itemAttributeTemplates'
import ItemEditor from '../components/items/ItemEditor.vue'
import AuthenticatedImage from '../components/attributes/AuthenticatedImage.vue'
import ModalDialog from '../components/ModalDialog.vue'
import { ApiError } from '../api/errors'
import { notice } from '../notices/notice'
import { draftFromItem, emptyItemDraft, itemAttributeRequests, type ItemDraft } from './items/model'
import { discardTemporaryItemFiles } from './items/fileCleanup'
import { isImageDraftValue, uploadImageDrafts } from '../components/attributes/imageDraft'
import { deleteImage } from '../api/files'
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
const loading = ref(false)
const loadingMore = ref(false)
const saving = ref(false)
const loadError = ref('')
const loadMoreError = ref('')
const metadataError = ref('')
const editorOpen = ref(false)
const discardDialogOpen = ref(false)
const loadMoreSentinel = ref<HTMLElement | null>(null)
const baselineFingerprint = ref('')

let searchTimer: number | undefined
let catalogAbortController: AbortController | null = null
let loadMoreObserver: IntersectionObserver | null = null
let pendingTransition: (() => Promise<void>) | null = null

const hasMoreItems = computed(() => page.value < totalPages.value)
const catalogPending = computed(() => loading.value || loadingMore.value)
const hasUnsavedChanges = computed(() => draftFingerprint(draft.value) !== baselineFingerprint.value)
const categoriesById = computed(() => new Map(categories.value.map((category) => [category.id, category.name])))
const catalogSummary = computed(() => {
  if (loading.value && items.value.length === 0) return '加载中'
  return activeSearch.value ? `${total.value} 个结果` : `${total.value} 个物品`
})

watch(loadMoreSentinel, (element, previousElement) => {
  if (previousElement) loadMoreObserver?.unobserve(previousElement)
  if (element) loadMoreObserver?.observe(element)
})

onMounted(() => {
  loadMoreObserver = new IntersectionObserver(handleLoadMoreIntersection, { rootMargin: '220px 0px' })
  if (loadMoreSentinel.value) loadMoreObserver.observe(loadMoreSentinel.value)
  void initializePage()
})

onBeforeUnmount(() => {
  window.clearTimeout(searchTimer)
  catalogAbortController?.abort()
  loadMoreObserver?.disconnect()
  void discardCurrentTemporaryFiles()
})

async function initializePage(): Promise<void> {
  baselineFingerprint.value = draftFingerprint(draft.value)
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

async function resetAndLoadCatalog(): Promise<void> {
  items.value = []
  total.value = 0
  page.value = 1
  totalPages.value = 0
  loadMoreError.value = ''
  await loadCatalog(1)
}

function scheduleSearch(): void {
  window.clearTimeout(searchTimer)
  searchTimer = window.setTimeout(() => {
    const nextSearch = searchInput.value.trim()
    if (nextSearch === activeSearch.value) return
    activeSearch.value = nextSearch
    void resetAndLoadCatalog()
  }, 280)
}

function clearSearch(): void {
  searchInput.value = ''
  activeSearch.value = ''
  void resetAndLoadCatalog()
}

function requestRefreshCatalog(): void {
  requestDraftTransition(refreshCatalog)
}

async function refreshCatalog(): Promise<void> {
  await discardCurrentTemporaryFiles()
  editorOpen.value = false
  draft.value = emptyItemDraft()
  baselineFingerprint.value = draftFingerprint(draft.value)
  await resetAndLoadCatalog()
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

function requestCloseMobileEditor(): void {
  requestDraftTransition(async () => {
    await restoreCurrentDraft()
    editorOpen.value = false
  })
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
}

async function confirmPendingTransition(): Promise<void> {
  const action = pendingTransition
  discardDialogOpen.value = false
  pendingTransition = null
  if (action) await action()
}

async function prepareNewDraft(openEditor: boolean): Promise<void> {
  await discardCurrentTemporaryFiles()
  const next = emptyItemDraft()
  draft.value = next
  baselineFingerprint.value = draftFingerprint(next)
  editorOpen.value = openEditor
}

async function editItem(item: ItemResponse): Promise<void> {
  await discardCurrentTemporaryFiles()
  draft.value = draftFromItem(item)
  baselineFingerprint.value = draftFingerprint(draft.value)
  editorOpen.value = true
}

async function restoreCurrentDraft(): Promise<void> {
  await discardCurrentTemporaryFiles()
  const current = items.value.find((item) => item.id === draft.value.id)
  if (current) {
    draft.value = draftFromItem(current)
    baselineFingerprint.value = draftFingerprint(draft.value)
    return
  }
  draft.value = emptyItemDraft()
  baselineFingerprint.value = draftFingerprint(draft.value)
}

async function save(): Promise<void> {
  if (!draft.value.name.trim() || !draft.value.sku.trim() || !draft.value.unit.trim()) {
    notice.warning('请填写名称、SKU 和计量单位')
    return
  }
  if (!draft.value.image) {
    notice.warning('请选择物品主图')
    return
  }
  saving.value = true
  try {
    await uploadImageDrafts([
      draft.value.image,
      ...draft.value.attributes.map((attribute) => attribute.value).filter(isImageDraftValue),
    ])
    const baseRequest = {
      name: draft.value.name.trim(),
      sku: draft.value.sku.trim(),
      unit: draft.value.unit.trim(),
      attributes: itemAttributeRequests(draft.value),
    }
    const wasEditing = Boolean(draft.value.id)
    const saved = draft.value.id
      ? await updateItem(draft.value.id, updateRequest(baseRequest))
      : await createItem(createRequest(baseRequest))
    if (draft.value.obsoleteImageFileId) {
      await deleteImage(draft.value.obsoleteImageFileId).catch(() => {
        notice.warning('旧物品主图未能立即清理', { detail: '服务会在超过保留期限后自动清理。' })
      })
    }
    draft.value.attributes.forEach((attribute) => { attribute.fileTemporary = false })
    draft.value.imageTemporary = false
    draft.value.obsoleteImageFileId = null
    draft.value = draftFromItem(saved)
    baselineFingerprint.value = draftFingerprint(draft.value)
    notice.success(wasEditing ? '物品已更新' : '物品已创建')
    await resetAndLoadCatalog()
  } catch (error) {
    const imageError = [draft.value.image, ...draft.value.attributes.map((attribute) => attribute.value)]
      .find((value) => isImageDraftValue(value) && value.status === 'failed')
    notice.error(imageError ? '物品图片上传失败' : '保存物品失败', {
      detail: isImageDraftValue(imageError) ? imageError.error : errorMessage(error),
    })
  } finally {
    saving.value = false
  }
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

function mergeItems(currentItems: ItemResponse[], nextItems: ItemResponse[]): ItemResponse[] {
  const itemsById = new Map(currentItems.map((item) => [item.id, item]))
  nextItems.forEach((item) => itemsById.set(item.id, item))
  return Array.from(itemsById.values())
}

function draftFingerprint(target: ItemDraft): string {
  return JSON.stringify({
    id: target.id,
    name: target.name,
    sku: target.sku,
    categoryId: target.categoryId,
    attributeTemplateId: target.attributeTemplateId,
    image: target.image ? [target.image.fileId, target.image.name, target.image.sizeBytes] : null,
    unit: target.unit,
    description: target.description,
    defaultPrice: target.defaultPrice,
    reorderPoint: target.reorderPoint,
    attributes: target.attributes.map((attribute) => ({
      templateFieldId: attribute.templateFieldId,
      fieldName: attribute.fieldName,
      fieldType: attribute.fieldType,
      value: isImageDraftValue(attribute.value)
        ? [attribute.value.fileId, attribute.value.name, attribute.value.sizeBytes]
        : attribute.value,
      unit: attribute.unit,
    })),
  })
}

function createRequest(base: Pick<ItemCreateRequest, 'name' | 'sku' | 'unit' | 'attributes'>): ItemCreateRequest {
  return {
    ...base,
    image_file_id: draft.value.image?.fileId as number,
    category_id: draft.value.categoryId ?? undefined,
    attribute_template_id: draft.value.attributeTemplateId ?? undefined,
    description: draft.value.description.trim() || undefined,
    default_price: draft.value.defaultPrice ?? undefined,
    reorder_point: draft.value.reorderPoint ?? undefined,
  }
}

function updateRequest(base: Pick<ItemUpdateRequest, 'name' | 'sku' | 'unit' | 'attributes'>): ItemUpdateRequest {
  return {
    ...base,
    image_file_id: draft.value.image?.fileId,
    category_id: draft.value.categoryId,
    attribute_template_id: draft.value.attributeTemplateId,
    description: draft.value.description.trim() || null,
    default_price: draft.value.defaultPrice,
    reorder_point: draft.value.reorderPoint,
  }
}
</script>
