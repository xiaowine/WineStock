<!--
  本文件拥有审计日志页面的路由筛选、服务端分页、刷新和详情编排。
  它只读取审计事件，不修改业务对象，也不根据当前对象覆盖历史详情。
-->
<template>
  <section class="route-page events-page">
    <header class="content-header events-page__header">
      <div>
        <h1>{{ $route.meta.title }}</h1>
        <p>追踪用户与库存业务产生的关键操作和历史变更。</p>
      </div>
    </header>

    <section class="events-workspace" aria-label="审计日志列表">
      <div class="events-toolbar">
        <label class="events-toolbar__field events-toolbar__entity">
          <span>实体类型</span>
          <SelectControl v-model="entitySelectValue" name="event_entity_type" :disabled="requestPending" @change="changeEntityType">
            <option value="">全部实体</option>
            <option v-for="option in eventEntityOptions" :key="option.value" :value="option.value">{{ option.label }}</option>
            <option :value="CUSTOM_EVENT_FILTER">其它类型…</option>
          </SelectControl>
        </label>

        <label class="events-toolbar__field events-toolbar__action">
          <span>动作</span>
          <SelectControl v-model="actionSelectValue" name="event_action" :disabled="requestPending" @change="changeAction">
            <option value="">全部动作</option>
            <option v-for="option in eventActionOptions" :key="option.value" :value="option.value">{{ option.label }}</option>
            <option :value="CUSTOM_EVENT_FILTER">其它动作…</option>
          </SelectControl>
        </label>

        <FormInput
          v-model="dateFromInput"
          class="events-toolbar__date"
          label="开始时间"
          type="datetime-local"
          step="1"
          :disabled="requestPending"
          @change="applyDateRange"
        />
        <FormInput
          v-model="dateToInput"
          class="events-toolbar__date"
          label="结束时间"
          type="datetime-local"
          step="1"
          :disabled="requestPending"
          @change="applyDateRange"
        />

        <div class="events-toolbar__meta">
          <span class="events-toolbar__count">{{ total }} 条</span>
          <div class="events-toolbar__actions">
            <button
              class="icon-button events-toolbar__filter"
              type="button"
              title="更多筛选"
              aria-label="更多筛选"
              :aria-expanded="filterDialogOpen"
              @click="filterDialogOpen = true"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 6h16M7 12h10M10 18h4" /></svg>
              <span v-if="advancedFilterCount" aria-hidden="true">{{ advancedFilterCount }}</span>
            </button>
            <button
              class="icon-button events-toolbar__refresh"
              :class="{ 'events-toolbar__refresh--pending': showStableRefreshing }"
              type="button"
              title="刷新审计日志"
              aria-label="刷新审计日志"
              :aria-busy="requestPending"
              :disabled="requestPending"
              @click="refreshCurrentPage"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M20 7v5h-5" /><path d="M18.2 16a7 7 0 1 1 .8-7l1 3" /></svg>
            </button>
          </div>
        </div>
      </div>

      <div v-if="activeFilters.length" class="events-active-filters" aria-label="当前筛选">
        <span v-for="filter in activeFilters" :key="filter.key">
          {{ filter.label }}
          <button type="button" :aria-label="`清除筛选：${filter.label}`" @click="clearFilter(filter.key)">×</button>
        </span>
        <button class="text-button" type="button" @click="clearAllFilters">清除全部</button>
      </div>

      <div
        class="events-results"
        :class="{ 'events-results--refreshing': showStableRefreshing }"
        :aria-busy="requestPending"
      >
        <section v-if="loadError && !loaded" class="events-state events-state--error" role="alert">
          <strong>无法加载审计日志</strong>
          <span>{{ loadError }}</span>
          <button class="secondary-button" type="button" @click="loadCurrentPage">重试</button>
        </section>
        <section v-else-if="showInitialLoading && !loaded" class="events-state" role="status">正在加载审计日志…</section>
        <section v-else-if="events.length === 0" class="events-state">
          <strong>{{ activeFilters.length ? '没有符合筛选条件的审计事件' : '暂无审计事件' }}</strong>
          <span>{{ activeFilters.length ? '可以调整或清除筛选条件。' : '业务操作产生审计记录后会显示在这里。' }}</span>
        </section>
        <template v-else>
          <p v-if="loadError" class="events-inline-error" role="alert">{{ loadError }}</p>
          <div class="events-table" role="table" aria-label="审计事件">
            <div class="events-table__head" role="row">
              <span role="columnheader">时间与操作人</span>
              <span role="columnheader">对象与变更摘要</span>
              <span role="columnheader">动作与详情</span>
            </div>
            <article
              v-for="event in events"
              :key="event.id"
              class="events-table__row"
              role="row"
              tabindex="0"
              @click="selectedEvent = event"
              @keydown.enter="selectedEvent = event"
            >
              <div class="events-table__identity" role="cell">
                <strong :title="event.timestamp">{{ formatLocalTimestamp(event.timestamp) }}</strong>
                <button class="events-table__context-link" type="button" :disabled="event.user_id === null" @click.stop="filterByActor(event)">
                  {{ actorLabel(event) }}
                </button>
              </div>
              <div class="events-table__summary" role="cell">
                <button class="events-table__target" type="button" :disabled="event.entity_id === null" @click.stop="filterByEntity(event)">
                  {{ entityTargetLabel(event) }}
                </button>
                <span :title="eventSummary(event)">{{ eventSummary(event) }}</span>
              </div>
              <div class="events-table__decision" role="cell">
                <div>
                  <button type="button" class="event-action-pill" :class="`event-action-pill--${eventActionTone(event.action)}`" @click.stop="filterByAction(event.action)">
                    {{ eventActionLabel(event.action) }}
                  </button>
                  <span>事件 #{{ event.id }}</span>
                </div>
                <button class="icon-button" type="button" title="查看审计详情" :aria-label="`查看事件 ${event.id} 详情`" @click.stop="selectedEvent = event">
                  <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9" /><path d="M12 11v6M12 7h.01" /></svg>
                </button>
              </div>
            </article>
          </div>

          <div class="events-mobile-list">
            <article v-for="event in events" :key="event.id" class="event-mobile-item">
              <header>
                <div>
                  <time :datetime="event.timestamp">{{ formatLocalTimestamp(event.timestamp) }}</time>
                  <span class="event-action-pill" :class="`event-action-pill--${eventActionTone(event.action)}`">{{ eventActionLabel(event.action) }}</span>
                </div>
                <button class="icon-button" type="button" title="查看审计详情" :aria-label="`查看事件 ${event.id} 详情`" @click="selectedEvent = event">
                  <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9" /><path d="M12 11v6M12 7h.01" /></svg>
                </button>
              </header>
              <button class="event-mobile-item__target" type="button" :disabled="event.entity_id === null" @click="filterByEntity(event)">{{ entityTargetLabel(event) }}</button>
              <button class="event-mobile-item__actor" type="button" :disabled="event.user_id === null" @click="filterByActor(event)">{{ actorLabel(event) }}</button>
              <p>{{ eventSummary(event) }}</p>
              <small>事件 #{{ event.id }}</small>
            </article>
          </div>
        </template>
      </div>

      <footer v-if="loaded" class="events-pagination" aria-label="审计日志分页">
        <button class="secondary-button" type="button" :disabled="requestPending || page <= 1" @click="changePage(page - 1)">上一页</button>
        <span v-if="totalPages > 0">第 {{ page }} / {{ totalPages }} 页 · 共 {{ total }} 条</span>
        <span v-else>暂无审计事件</span>
        <button class="secondary-button" type="button" :disabled="requestPending || totalPages === 0 || page >= totalPages" @click="changePage(page + 1)">下一页</button>
      </footer>
    </section>

    <EventFilterDialog
      :open="filterDialogOpen"
      :value="advancedFilterValue"
      @close="closeFilterDialog"
      @apply="applyAdvancedFilters"
    />
    <EventDetailDialog
      :event="selectedEvent"
      @close="selectedEvent = null"
      @related="showRelatedEvents"
    />
  </section>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, reactive, ref, watch } from 'vue'
import { useRoute, useRouter, type LocationQueryRaw } from 'vue-router'
import { listEvents, type EventListQuery, type EventLogResponse } from '../api/events'
import { ApiConfigurationError, ApiError, ApiNetworkError, ApiResponseError } from '../api/errors'
import EventDetailDialog from '../components/events/EventDetailDialog.vue'
import EventFilterDialog, { type EventAdvancedFilterValue } from '../components/events/EventFilterDialog.vue'
import FormInput from '../components/forms/FormInput.vue'
import SelectControl from '../components/forms/SelectControl.vue'
import { useStablePendingIndicator } from '../composables/useStablePendingIndicator'
import { notice } from '../notices/notice'
import {
  CUSTOM_EVENT_FILTER, eventActionLabel, eventActionOptions, eventActionTone,
  eventEntityLabel, eventEntityOptions, isKnownAction, isKnownEntityType,
} from './events/catalog'
import { eventSummary } from './events/details'

interface EventPageState {
  entityType: string
  action: string
  entityId: number | null
  userId: number | null
  dateFrom: string
  dateTo: string
  page: number
  pageSize: number
}

type FilterKey = 'entityType' | 'action' | 'entityId' | 'userId' | 'dateFrom' | 'dateTo'

const route = useRoute()
const router = useRouter()
const state = reactive<EventPageState>(defaultState())
const events = ref<EventLogResponse[]>([])
const total = ref(0)
const totalPages = ref(0)
const loaded = ref(false)
const requestPending = ref(false)
const loadError = ref('')
const filterDialogOpen = ref(false)
const selectedEvent = ref<EventLogResponse | null>(null)
const entitySelectValue = ref('')
const actionSelectValue = ref('')
const dateFromInput = ref('')
const dateToInput = ref('')
let requestController: AbortController | null = null

const page = computed(() => state.page)
const showInitialLoading = useStablePendingIndicator(computed(() => requestPending.value && !loaded.value), { showDelayMs: 200, minimumVisibleMs: 350 })
const showStableRefreshing = useStablePendingIndicator(computed(() => requestPending.value && loaded.value), { showDelayMs: 200, minimumVisibleMs: 350 })
const advancedFilterCount = computed(() => [state.entityId, state.userId, state.pageSize !== 50, !isKnownEntityType(state.entityType) && Boolean(state.entityType), !isKnownAction(state.action) && Boolean(state.action)].filter(Boolean).length)
const advancedFilterValue = computed<EventAdvancedFilterValue>(() => ({
  entityId: state.entityId,
  userId: state.userId,
  customEntityType: state.entityType && !isKnownEntityType(state.entityType) ? state.entityType : '',
  customAction: state.action && !isKnownAction(state.action) ? state.action : '',
  dateFrom: isoToLocalInput(state.dateFrom),
  dateTo: isoToLocalInput(state.dateTo),
  pageSize: state.pageSize,
}))
const activeFilters = computed(() => {
  const values: Array<{ key: FilterKey; label: string }> = []
  if (state.entityType) values.push({ key: 'entityType', label: `实体：${eventEntityLabel(state.entityType)}` })
  if (state.action) values.push({ key: 'action', label: `动作：${eventActionLabel(state.action)}` })
  if (state.entityId !== null) values.push({ key: 'entityId', label: `实体 ID：#${state.entityId}` })
  if (state.userId !== null) values.push({ key: 'userId', label: `操作人：#${state.userId}` })
  if (state.dateFrom) values.push({ key: 'dateFrom', label: `开始：${formatLocalTimestamp(state.dateFrom)}` })
  if (state.dateTo) values.push({ key: 'dateTo', label: `结束：${formatLocalTimestamp(state.dateTo)}` })
  return values
})

watch(
  () => route.fullPath,
  () => {
    Object.assign(state, stateFromQuery(route.query))
    syncInputsFromState()
    const normalizedQuery = queryFromState(state)
    if (queryFingerprint(route.query) !== queryFingerprint(normalizedQuery)) {
      void router.replace({ name: 'events', query: normalizedQuery })
      return
    }
    void loadCurrentPage()
  },
  { immediate: true },
)

onBeforeUnmount(() => requestController?.abort())

async function loadCurrentPage(): Promise<boolean> {
  requestController?.abort()
  const controller = new AbortController()
  requestController = controller
  requestPending.value = true
  loadError.value = ''
  try {
    const response = await listEvents(eventQuery(), controller.signal)
    events.value = response.items
    total.value = response.total
    totalPages.value = response.total_pages
    loaded.value = true
    if (response.total_pages > 0 && state.page > response.total_pages) {
      await navigate({ page: response.total_pages }, true)
    }
    return true
  } catch (error) {
    if (error instanceof DOMException && error.name === 'AbortError') return false
    loadError.value = eventErrorMessage(error)
    if (loaded.value) notice.error('刷新审计日志失败', { detail: loadError.value })
    return false
  } finally {
    if (requestController === controller) requestPending.value = false
  }
}

async function refreshCurrentPage(): Promise<void> {
  if (await loadCurrentPage()) notice.success('审计日志已刷新')
}

function eventQuery(): EventListQuery {
  return {
    page: state.page,
    page_size: state.pageSize,
    entity_type: state.entityType || undefined,
    entity_id: state.entityId ?? undefined,
    action: state.action || undefined,
    user_id: state.userId ?? undefined,
    date_from: state.dateFrom || undefined,
    date_to: state.dateTo || undefined,
  }
}

function changeEntityType(value: unknown): void {
  const next = String(value ?? '')
  if (next === CUSTOM_EVENT_FILTER) {
    filterDialogOpen.value = true
    syncInputsFromState()
    return
  }
  void navigate({ entityType: next, page: 1 })
}

function changeAction(value: unknown): void {
  const next = String(value ?? '')
  if (next === CUSTOM_EVENT_FILTER) {
    filterDialogOpen.value = true
    syncInputsFromState()
    return
  }
  void navigate({ action: next, page: 1 })
}

function applyDateRange(): void {
  const dateFrom = localInputToIso(dateFromInput.value)
  const dateTo = localInputToIso(dateToInput.value)
  if ((dateFromInput.value && !dateFrom) || (dateToInput.value && !dateTo)) {
    notice.warning('请输入有效的审计时间')
    return
  }
  if (dateFrom && dateTo && dateFrom > dateTo) {
    notice.warning('开始时间不能晚于结束时间')
    return
  }
  void navigate({ dateFrom, dateTo, page: 1 })
}

function applyAdvancedFilters(value: EventAdvancedFilterValue): void {
  const entityType = value.customEntityType || (isKnownEntityType(state.entityType) ? state.entityType : '')
  const action = value.customAction || (isKnownAction(state.action) ? state.action : '')
  const dateFrom = localInputToIso(value.dateFrom)
  const dateTo = localInputToIso(value.dateTo)
  if ((value.dateFrom && !dateFrom) || (value.dateTo && !dateTo) || (dateFrom && dateTo && dateFrom > dateTo)) {
    notice.warning('请输入有效的审计时间范围')
    return
  }
  filterDialogOpen.value = false
  void navigate({
    entityType,
    action,
    entityId: value.entityId,
    userId: value.userId,
    dateFrom,
    dateTo,
    pageSize: value.pageSize,
    page: 1,
  })
}

function closeFilterDialog(): void {
  filterDialogOpen.value = false
  syncInputsFromState()
}

function filterByActor(event: EventLogResponse): void {
  if (event.user_id !== null) void navigate({ userId: event.user_id, page: 1 })
}

function filterByEntity(event: EventLogResponse): void {
  if (event.entity_id !== null) void navigate({ entityType: event.entity_type, entityId: event.entity_id, page: 1 })
}

function filterByAction(action: string): void {
  void navigate({ action, page: 1 })
}

function showRelatedEvents(event: EventLogResponse): void {
  selectedEvent.value = null
  filterByEntity(event)
}

function clearFilter(key: FilterKey): void {
  const patch: Partial<EventPageState> = { page: 1 }
  if (key === 'entityType') patch.entityType = ''
  if (key === 'action') patch.action = ''
  if (key === 'entityId') patch.entityId = null
  if (key === 'userId') patch.userId = null
  if (key === 'dateFrom') patch.dateFrom = ''
  if (key === 'dateTo') patch.dateTo = ''
  void navigate(patch)
}

function clearAllFilters(): void {
  void navigate({ ...defaultState(), pageSize: state.pageSize })
}

function changePage(nextPage: number): void {
  if (nextPage >= 1 && (totalPages.value === 0 || nextPage <= totalPages.value)) {
    void navigate({ page: nextPage }, false)
  }
}

async function navigate(patch: Partial<EventPageState>, replace = true): Promise<void> {
  const next = { ...state, ...patch }
  const query = queryFromState(next)
  if (queryFingerprint(route.query) === queryFingerprint(query)) {
    await loadCurrentPage()
    return
  }
  if (replace) await router.replace({ name: 'events', query })
  else await router.push({ name: 'events', query })
}

function syncInputsFromState(): void {
  entitySelectValue.value = state.entityType
    ? isKnownEntityType(state.entityType) ? state.entityType : CUSTOM_EVENT_FILTER
    : ''
  actionSelectValue.value = state.action
    ? isKnownAction(state.action) ? state.action : CUSTOM_EVENT_FILTER
    : ''
  dateFromInput.value = isoToLocalInput(state.dateFrom)
  dateToInput.value = isoToLocalInput(state.dateTo)
}

function stateFromQuery(query: Record<string, unknown>): EventPageState {
  return {
    entityType: textQuery(query.entity_type),
    action: textQuery(query.action),
    entityId: positiveIntegerQuery(query.entity_id),
    userId: positiveIntegerQuery(query.user_id),
    dateFrom: validDateQuery(query.date_from),
    dateTo: validDateQuery(query.date_to),
    page: positiveIntegerQuery(query.page) ?? 1,
    pageSize: allowedPageSize(positiveIntegerQuery(query.page_size)),
  }
}

function queryFromState(value: EventPageState): LocationQueryRaw {
  const query: LocationQueryRaw = {}
  if (value.entityType) query.entity_type = value.entityType
  if (value.action) query.action = value.action
  if (value.entityId !== null) query.entity_id = String(value.entityId)
  if (value.userId !== null) query.user_id = String(value.userId)
  if (value.dateFrom) query.date_from = value.dateFrom
  if (value.dateTo) query.date_to = value.dateTo
  if (value.page > 1) query.page = String(value.page)
  if (value.pageSize !== 50) query.page_size = String(value.pageSize)
  return query
}

function queryFingerprint(query: Record<string, unknown>): string {
  return Object.entries(query)
    .flatMap(([key, value]) => (Array.isArray(value) ? value : [value]).map((item) => [key, item == null ? '' : String(item)] as const))
    .sort(([leftKey, leftValue], [rightKey, rightValue]) => leftKey.localeCompare(rightKey) || leftValue.localeCompare(rightValue))
    .map(([key, value]) => `${encodeURIComponent(key)}=${encodeURIComponent(value)}`)
    .join('&')
}

function defaultState(): EventPageState {
  return { entityType: '', action: '', entityId: null, userId: null, dateFrom: '', dateTo: '', page: 1, pageSize: 50 }
}

function textQuery(value: unknown): string {
  const current = Array.isArray(value) ? value[0] : value
  return typeof current === 'string' ? current.trim().slice(0, 64) : ''
}

function positiveIntegerQuery(value: unknown): number | null {
  const current = Array.isArray(value) ? value[0] : value
  const parsed = typeof current === 'string' ? Number(current) : typeof current === 'number' ? current : NaN
  return Number.isInteger(parsed) && parsed > 0 ? parsed : null
}

function validDateQuery(value: unknown): string {
  const current = textQuery(value)
  return current && !Number.isNaN(new Date(current).getTime()) ? new Date(current).toISOString() : ''
}

function allowedPageSize(value: number | null): number {
  return value !== null && [25, 50, 100, 200].includes(value) ? value : 50
}

function localInputToIso(value: string): string {
  if (!value) return ''
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? '' : date.toISOString()
}

function isoToLocalInput(value: string): string {
  if (!value) return ''
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return ''
  const offset = date.getTimezoneOffset() * 60_000
  return new Date(date.getTime() - offset).toISOString().slice(0, 19)
}

function formatLocalTimestamp(value: string): string {
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : new Intl.DateTimeFormat('zh-CN', {
    year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false,
  }).format(date)
}

function actorLabel(event: EventLogResponse): string {
  if (event.username) return event.user_id === null ? event.username : `${event.username} · #${event.user_id}`
  return event.user_id === null ? '系统/未知操作人' : `用户 #${event.user_id}`
}

function entityTargetLabel(event: EventLogResponse): string {
  return `${eventEntityLabel(event.entity_type)} · ${event.entity_id === null ? '无实体编号' : `#${event.entity_id}`}`
}

function eventErrorMessage(error: unknown): string {
  if (error instanceof ApiError) {
    if (error.code === 'permission_denied') return '当前账号没有查看审计日志的权限'
    return error.message
  }
  if (error instanceof ApiConfigurationError) return error.message
  if (error instanceof ApiNetworkError) return '无法连接到 WineStock 服务'
  if (error instanceof ApiResponseError) return '服务响应格式无效，请检查前后端版本'
  return '加载审计日志失败，请稍后重试'
}
</script>

<style lang="scss" src="./EventsPage.scss"></style>
