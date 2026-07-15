<!-- 本页面拥有入库单服务端筛选、触底追加与只读详情；新建和审批分别属于其它路由。 -->
<template>
  <section class="route-page inbound-orders-page">
    <header class="content-header inbound-orders-page__header">
      <div><h1>{{ $route.meta.title }}</h1><p>查询入库记录、状态与每条收货明细。</p></div>
      <button v-if="canCreate" class="primary-button" type="button" @click="router.push({ name: 'inbound' })">新建入库</button>
    </header>

    <section class="inbound-orders-workspace" aria-label="入库单列表">
      <div class="inbound-orders-toolbar">
        <SearchField v-model="searchInput" label="搜索入库单" name="inbound_order_search" placeholder="搜索单号、来源、物品或批次" hide-label @search="applySearch" />
        <div class="inbound-orders-toolbar__meta">
          <span class="inbound-orders-count">{{ total }} 条</span>
          <div class="inbound-orders-toolbar__actions">
            <button class="icon-button inbound-orders-toolbar__filter" type="button" title="筛选入库单" aria-label="筛选入库单" :aria-expanded="filterDialogOpen" @click="filterDialogOpen = true">
              <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 6h16M7 12h10M10 18h4" /></svg><span v-if="filterCount" aria-hidden="true">{{ filterCount }}</span>
            </button>
            <button class="icon-button inbound-orders-toolbar__refresh" :class="{ 'inbound-orders-refresh--pending': showRefreshing }" type="button" title="刷新入库单" :disabled="requestPending" @click="refreshCurrent">
              <svg viewBox="0 0 24 24"><path d="M20 7v5h-5"/><path d="M18.2 16a7 7 0 1 1 .8-7l1 3"/></svg>
            </button>
          </div>
        </div>
      </div>

      <div ref="resultsElement" class="inbound-orders-results" :class="{ 'inbound-orders-results--refreshing': showRefreshing }" :aria-busy="requestPending">
        <section v-if="loadError && !loaded" class="inbound-orders-state inbound-orders-state--error"><strong>无法加载入库单</strong><span>{{ loadError }}</span><button class="secondary-button" type="button" @click="loadCurrentPage">重试</button></section>
        <section v-else-if="showInitialLoading && !loaded" class="inbound-orders-state" role="status">正在加载入库单…</section>
        <section v-else-if="!orders.length" class="inbound-orders-state"><strong>{{ hasFilters ? '没有符合筛选条件的入库单' : '暂无入库单' }}</strong><button v-if="hasFilters" class="text-button" type="button" @click="clearFilters">清除筛选</button></section>
        <template v-else>
          <p v-if="loadError" class="inbound-orders-inline-error" role="alert">{{ loadError }}</p>
          <InboundOrderList :orders="orders" :format-date="formatDate" :money="money" :quantity-label="quantityLabel" :status-hint="approvalLabel" :status-label="statusLabel" :status-time="statusTime" :total-amount="totalAmount" @open="openDetail" />
          <div ref="loadMoreSentinel" class="inbound-orders-load-more" aria-live="polite">
            <span v-if="loadingMore" role="status">正在加载更多入库单…</span>
            <button v-else-if="loadMoreError" class="secondary-button" type="button" @click="loadNextPage">加载失败，点击重试</button>
            <span v-else-if="hasMoreOrders">继续向下滚动加载</span>
            <span v-else>已加载全部 {{ total }} 条入库单</span>
          </div>
        </template>
      </div>
    </section>

    <InboundOrderFiltersDialog :open="filterDialogOpen" :value="filterValue" @close="filterDialogOpen = false" @apply="applyFilters" />
    <ModalDialog :open="selected !== null" wide :title="selected ? `入库单 #${selected.id}` : ''" :busy="detailLoading" @close="closeDetail">
      <template #context><div v-if="selected" class="dialog-account-context"><span>{{ statusLabel(selected.status) }}</span><strong>{{ selected.source }}</strong></div></template>
      <section v-if="detailError" class="inbound-detail-error"><strong>{{ detailError }}</strong><button class="secondary-button" type="button" @click="loadDetail">重试</button></section>
      <section v-else-if="detailLoading" class="inbound-detail-loading">正在加载入库单详情…</section>
      <template v-else-if="selected"><dl class="inbound-detail-summary"><div><dt>创建时间</dt><dd>{{ formatDate(selected.created_at) }}</dd></div><div><dt>状态</dt><dd>{{ statusLabel(selected.status) }}</dd></div><div><dt>备注</dt><dd>{{ selected.notes || '暂无备注' }}</dd></div><div><dt>审批记录</dt><dd>{{ approvalLabel(selected) }}</dd></div></dl><section class="inbound-detail-items"><h3>入库物品 <span>{{ selected.items.length }} 条</span></h3><article v-for="item in selected.items" :key="item.id"><header class="inbound-detail-item__header"><AuthenticatedImage :file-id="item.item_image_file_id" :alt="`${item.item_name} 主图`" :size="52" previewable /><div><strong>{{ item.item_name }}</strong><small>{{ item.item_sku }} · {{ item.item_unit }} · 物品 #{{ item.item_id }}</small></div><span>¥{{ money(item.quantity * item.unit_price) }}</span></header><dl><div><dt>数量</dt><dd>{{ item.quantity }} {{ item.item_unit }}</dd></div><div><dt>单价</dt><dd>¥{{ money(item.unit_price) }}</dd></div><div><dt>库位</dt><dd>{{ item.location_name }}</dd></div><div><dt>批次</dt><dd>{{ item.batch_no || '自动生成' }}</dd></div><div><dt>有效期</dt><dd>{{ item.expires_at || '未设置' }}</dd></div><div><dt>模板</dt><dd>{{ item.inbound_template_id ? `#${item.inbound_template_id}` : '未使用' }}</dd></div></dl><dl v-if="item.ext_attributes" class="inbound-detail-attributes"><div v-for="(value, key) in item.ext_attributes" :key="key"><dt>{{ key }}</dt><dd>{{ jsonValue(value) }}</dd></div></dl></article></section></template>
      <template #actions><button class="secondary-button" type="button" @click="closeDetail">关闭</button><button v-if="canApprove && selected?.status === 'pending'" class="primary-button" type="button" @click="router.push({ name: 'inbound-approvals' })">前往入库审批</button></template>
    </ModalDialog>
  </section>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { getInboundOrder, listInboundOrders, type InboundOrderResponse, type InboundOrderStatus } from '../api/inboundOrders'
import { ApiError } from '../api/errors'
import { hasPermission, stockPermissions } from '../auth/permissions'
import { authSession } from '../auth/session'
import AuthenticatedImage from '../components/attributes/AuthenticatedImage.vue'
import ModalDialog from '../components/ModalDialog.vue'
import InboundOrderFiltersDialog, { type InboundOrderFilterValue } from '../components/inbound/InboundOrderFiltersDialog.vue'
import InboundOrderList from '../components/inbound/InboundOrderList.vue'
import SearchField from '../components/SearchField.vue'
import { useStablePendingIndicator } from '../composables/useStablePendingIndicator'
import './InboundOrdersPage.scss'

const route = useRoute()
const router = useRouter()
const orders = ref<InboundOrderResponse[]>([])
const total = ref(0)
const totalPages = ref(0)
const loaded = ref(false)
const loading = ref(false)
const loadingMore = ref(false)
const loadError = ref('')
const loadMoreError = ref('')
const selected = ref<InboundOrderResponse | null>(null)
const detailLoading = ref(false)
const detailError = ref('')
const searchInput = ref('')
const dateFromInput = ref('')
const dateToInput = ref('')
const filterDialogOpen = ref(false)
const resultsElement = ref<HTMLElement | null>(null)
const loadMoreSentinel = ref<HTMLElement | null>(null)
const state = reactive<{ page: number; status: InboundOrderStatus | ''; search: string }>({ page: 1, status: '', search: '' })
let controller: AbortController | null = null
let detailController: AbortController | null = null
let loadMoreObserver: IntersectionObserver | null = null

const canCreate = computed(() => hasPermission(authSession.value?.user.permissions, stockPermissions.inboundCreate))
const canApprove = computed(() => hasPermission(authSession.value?.user.permissions, stockPermissions.inboundApprove))
const requestPending = computed(() => loading.value || loadingMore.value)
const hasMoreOrders = computed(() => state.page < totalPages.value)
const showInitialLoading = useStablePendingIndicator(computed(() => loading.value && !loaded.value), { showDelayMs: 200, minimumVisibleMs: 350 })
const showRefreshing = useStablePendingIndicator(computed(() => loading.value && loaded.value), { showDelayMs: 200, minimumVisibleMs: 350 })
const hasFilters = computed(() => Boolean(state.search || state.status || dateFromInput.value || dateToInput.value))
const filterCount = computed(() => [state.status, dateFromInput.value, dateToInput.value].filter(Boolean).length)
const filterValue = computed<InboundOrderFilterValue>(() => ({ status: state.status, dateFrom: isoToLocalInput(dateFromInput.value), dateTo: isoToLocalInput(dateToInput.value) }))

watch(() => route.fullPath, () => {
  const query = route.query
  state.status = isStatus(query.status) ? query.status : ''
  state.search = typeof query.search === 'string' ? query.search : ''
  searchInput.value = state.search
  dateFromInput.value = validDateQuery(query.date_from)
  dateToInput.value = validDateQuery(query.date_to)
  void loadCurrentPage()
}, { immediate: true })

watch([resultsElement, loadMoreSentinel], () => refreshLoadMoreObservation())

onMounted(() => {
  loadMoreObserver = new IntersectionObserver(handleLoadMoreIntersection, { root: resultsElement.value, rootMargin: '240px 0px' })
  refreshLoadMoreObservation()
})

onBeforeUnmount(() => {
  controller?.abort()
  detailController?.abort()
  loadMoreObserver?.disconnect()
})

async function loadCurrentPage(): Promise<boolean> { return loadOrders(1) }

async function loadOrders(targetPage: number, append = false): Promise<boolean> {
  controller?.abort()
  const request = new AbortController()
  controller = request
  const shouldAppend = append && orders.value.length > 0
  loading.value = !shouldAppend
  loadingMore.value = shouldAppend
  loadMoreError.value = ''
  if (!shouldAppend) loadError.value = ''
  try {
    const response = await listInboundOrders({ page: targetPage, page_size: 50, search: state.search || undefined, status: state.status || undefined, date_from: dateFromInput.value || undefined, date_to: dateToInput.value || undefined }, request.signal)
    if (controller !== request) return false
    orders.value = shouldAppend ? mergeOrders(orders.value, response.items) : response.items
    total.value = response.total
    totalPages.value = response.total_pages
    state.page = response.page
    loaded.value = true
    return true
  } catch (cause) {
    if (cause instanceof DOMException && cause.name === 'AbortError') return false
    const message = cause instanceof ApiError ? cause.message : '请检查服务连接后重试'
    if (shouldAppend) loadMoreError.value = message
    else loadError.value = message
    return false
  } finally {
    if (controller === request) {
      controller = null
      loading.value = false
      loadingMore.value = false
      void nextTick().then(refreshLoadMoreObservation)
    }
  }
}

function mergeOrders(current: InboundOrderResponse[], incoming: InboundOrderResponse[]): InboundOrderResponse[] {
  const ids = new Set(current.map((order) => order.id))
  const merged = [...current]
  for (const order of incoming) {
    if (ids.has(order.id)) continue
    ids.add(order.id)
    merged.push(order)
  }
  return merged
}

function handleLoadMoreIntersection(entries: IntersectionObserverEntry[]): void {
  if (entries.some((entry) => entry.isIntersecting)) void loadNextPage()
}

async function loadNextPage(): Promise<void> {
  if (requestPending.value || !hasMoreOrders.value) return
  await loadOrders(state.page + 1, true)
}

function refreshLoadMoreObservation(): void {
  const sentinel = loadMoreSentinel.value
  if (!sentinel || !loadMoreObserver) return
  loadMoreObserver.disconnect()
  loadMoreObserver.observe(sentinel)
}

async function refreshCurrent(): Promise<void> { await loadCurrentPage() }
function updateQuery(next: Record<string, string | undefined>): void { void router.replace({ query: next }) }
function applySearch(value: string): void { updateQuery({ search: value.trim() || undefined, status: state.status || undefined, date_from: dateFromInput.value || undefined, date_to: dateToInput.value || undefined }) }
function applyFilters(value: InboundOrderFilterValue): void {
  const dateFrom = localInputToIso(value.dateFrom)
  const dateTo = localInputToIso(value.dateTo)
  if ((value.dateFrom && !dateFrom) || (value.dateTo && !dateTo) || (dateFrom && dateTo && dateFrom > dateTo)) { loadError.value = '请输入有效的创建时间范围'; return }
  filterDialogOpen.value = false
  updateQuery({ search: state.search || undefined, status: value.status || undefined, date_from: dateFrom || undefined, date_to: dateTo || undefined })
}
function clearFilters(): void { updateQuery({}) }

function openDetail(order: InboundOrderResponse): void { selected.value = order; void loadDetail() }
function closeDetail(): void { detailController?.abort(); selected.value = null; detailError.value = '' }
async function loadDetail(): Promise<void> {
  if (!selected.value) return
  const orderId = selected.value.id
  detailController?.abort()
  const request = new AbortController()
  detailController = request
  detailLoading.value = true
  detailError.value = ''
  try {
    const detail = await getInboundOrder(orderId, request.signal)
    if (detailController === request && selected.value?.id === orderId) selected.value = detail
  } catch (cause) {
    if (detailController === request && !(cause instanceof DOMException && cause.name === 'AbortError')) detailError.value = cause instanceof ApiError ? cause.message : '无法加载详情'
  } finally { if (detailController === request) detailLoading.value = false }
}

function isStatus(value: unknown): value is InboundOrderStatus { return value === 'pending' || value === 'approved' || value === 'rejected' }
function validDateQuery(value: unknown): string { const current = typeof value === 'string' ? value : ''; return current && !Number.isNaN(new Date(current).getTime()) ? new Date(current).toISOString() : '' }
function localInputToIso(value: string): string { if (!value) return ''; const date = new Date(value); return Number.isNaN(date.getTime()) ? '' : date.toISOString() }
function isoToLocalInput(value: string): string { if (!value) return ''; const date = new Date(value); if (Number.isNaN(date.getTime())) return ''; const offset = date.getTimezoneOffset() * 60_000; return new Date(date.getTime() - offset).toISOString().slice(0, 19) }
function statusLabel(status: InboundOrderStatus): string { return { pending: '待审批', approved: '已入库', rejected: '已拒绝' }[status] }
function money(value: number): string { return new Intl.NumberFormat('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(value) }
function totalAmount(order: InboundOrderResponse): number { return order.items.reduce((sum, item) => sum + item.quantity * item.unit_price, 0) }
function quantityLabel(order: InboundOrderResponse): string {
  const units = new Set(order.items.map((item) => item.item_unit).filter(Boolean))
  if (units.size !== 1) return '按明细分别计量'
  return `合计 ${order.items.reduce((sum, item) => sum + item.quantity, 0)} ${Array.from(units)[0]}`
}
function formatDate(value: string): string { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : new Intl.DateTimeFormat('zh-CN', { dateStyle: 'medium', timeStyle: 'short' }).format(date) }
function statusTime(order: InboundOrderResponse): string | null { return order.status === 'approved' ? order.approved_at : order.status === 'rejected' ? order.rejected_at : null }
function approvalLabel(order: InboundOrderResponse): string { const time = statusTime(order); return time ? `${statusLabel(order.status)}于 ${formatDate(time)}` : '等待审批，库存尚未增加' }
function jsonValue(value: unknown): string { return typeof value === 'string' ? value : JSON.stringify(value) }
</script>
