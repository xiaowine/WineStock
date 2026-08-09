<!--
  本文件拥有操作日志页面的路由筛选、服务端分页、刷新和详情编排。
  它只读取审计事件，不修改业务对象，也不根据当前对象覆盖历史详情。
-->
<template>
  <section class="route-page events-page">
    <header class="content-header events-page__header">
      <div>
        <h1>{{ $title($route.meta.title) }}</h1>
        <p>{{ $t("events.subtitle") }}</p>
      </div>
    </header>

    <section class="events-workspace" :aria-label="$t('events.workspaceAriaLabel')">
      <div class="events-toolbar">
        <label class="events-toolbar__field events-toolbar__entity">
          <span>{{ $t('events.entityTypeLabel') }}</span>
          <SelectControl
            v-model="entitySelectValue"
            name="event_entity_type"
            :disabled="requestPending"
            @change="changeEntityType"
          >
            <option value="">{{ $t('events.allEntities') }}</option>
            <option v-for="option in eventEntityOptions" :key="option.value" :value="option.value">
              {{ $t(option.label) }}
            </option>
            <option :value="CUSTOM_EVENT_FILTER">{{ $t('events.otherTypes') }}</option>
          </SelectControl>
        </label>

        <label class="events-toolbar__field events-toolbar__action">
          <span>{{ $t('events.actionLabel') }}</span>
          <SelectControl
            v-model="actionSelectValue"
            name="event_action"
            :disabled="requestPending"
            @change="changeAction"
          >
            <option value="">{{ $t('events.allActions') }}</option>
            <option v-for="option in eventActionOptions" :key="option.value" :value="option.value">
              {{ $t(option.label) }}
            </option>
            <option :value="CUSTOM_EVENT_FILTER">{{ $t('events.otherActions') }}</option>
          </SelectControl>
        </label>

        <div class="events-toolbar__meta">
          <span class="events-toolbar__count">{{ $t('events.recordCount', { n: total }) }}</span>
          <div class="events-toolbar__actions">
            <button
              class="icon-button events-toolbar__filter"
              type="button"
              :title="$t('events.moreFilters')"
              :aria-label="$t('events.moreFilters')"
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
              :title="$t('events.refreshLog')"
              :aria-label="$t('events.refreshLog')"
              :aria-busy="requestPending"
              :disabled="requestPending"
              @click="refreshCurrentPage"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M20 7v5h-5" />
                <path d="M18.2 16a7 7 0 1 1 .8-7l1 3" />
              </svg>
            </button>
          </div>
        </div>
      </div>

      <div v-if="activeFilters.length" class="events-active-filters" :aria-label="$t('events.activeFiltersAria')">
        <div class="events-active-filters__chips">
          <span
            v-for="filter in activeFilters"
            :key="filter.key"
            :title="filter.title ?? filter.label"
          >
            {{ filter.label }}
            <button
              type="button"
              :aria-label="$t('events.clearFilter', { label: filter.label })"
              @click="clearFilter(filter.key)"
            >
              ×
            </button>
          </span>
        </div>
        <button class="text-button" type="button" @click="clearAllFilters">{{ $t('events.clearAll') }}</button>
      </div>

      <div
        v-overlay-scrollbar
        class="events-results"
        :class="{ 'events-results--refreshing': showStableRefreshing }"
        :aria-busy="requestPending"
      >
        <section v-if="loadError && !loaded" class="events-state events-state--error" role="alert">
          <strong>{{ $t('events.loadFailedTitle') }}</strong>
          <span>{{ loadError }}</span>
          <button class="secondary-button" type="button" @click="loadCurrentPage">{{ $t('common.retry') }}</button>
        </section>
        <section v-else-if="showInitialLoading && !loaded" class="events-state" role="status">
          {{ $t('events.loadingLog') }}
        </section>
        <section v-else-if="events.length === 0" class="events-state">
          <strong>{{
            activeFilters.length ? $t("events.noMatchingRecords") : $t("events.noRecords")
          }}</strong>
          <span>{{
            activeFilters.length ? $t("events.noMatchingRecordsHint") : $t("events.noRecordsHint")
          }}</span>
        </section>
        <template v-else>
          <p v-if="loadError" class="events-inline-error" role="alert">{{ loadError }}</p>
          <div class="events-table" role="table" :aria-label="$t('events.recordsAriaLabel')">
            <div class="events-table__head" role="row">
              <span role="columnheader">{{ $t('events.columnTimeActor') }}</span>
              <span role="columnheader">{{ $t('events.columnObjectSummary') }}</span>
              <span role="columnheader">{{ $t('events.columnActionDetails') }}</span>
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
                <strong :title="event.timestamp">{{
                  formatLocalTimestamp(event.timestamp)
                }}</strong>
                <button
                  class="events-table__context-link"
                  type="button"
                  :disabled="event.user_id === null"
                  @click.stop="filterByActor(event)"
                >
                  {{ actorLabel(event) }}
                </button>
              </div>
              <div class="events-table__summary" role="cell">
                <button
                  class="events-table__target"
                  type="button"
                  :disabled="event.entity_id === null"
                  @click.stop="filterByEntity(event)"
                >
                  {{ entityTargetLabel(event) }}
                </button>
                <span :title="eventSummary(event)">{{ eventSummary(event) }}</span>
              </div>
              <div class="events-table__decision" role="cell">
                <div>
                  <button
                    type="button"
                    class="event-action-pill"
                    :class="`event-action-pill--${eventActionTone(event.action)}`"
                    @click.stop="filterByAction(event.action)"
                  >
                    {{ eventActionLabel(event.action) }}
                  </button>
                  <span>{{ $t('events.eventId', { id: event.id }) }}</span>
                </div>
                <button
                  class="icon-button"
                  type="button"
                  :title="$t('events.viewDetails')"
                  :aria-label="$t('events.viewEventDetails', { id: event.id })"
                  @click.stop="selectedEvent = event"
                >
                  <svg viewBox="0 0 24 24" aria-hidden="true">
                    <circle cx="12" cy="12" r="9" />
                    <path d="M12 11v6M12 7h.01" />
                  </svg>
                </button>
              </div>
            </article>
          </div>

          <div class="events-mobile-list">
            <article
              v-for="event in events"
              :key="event.id"
              class="event-mobile-item"
              tabindex="0"
              :aria-label="$t('events.viewEventDetails', { id: event.id })"
              @click="selectedEvent = event"
              @keydown.enter.self="selectedEvent = event"
              @keydown.space.self.prevent="selectedEvent = event"
            >
              <header>
                <div>
                  <time :datetime="event.timestamp">{{
                    formatLocalTimestamp(event.timestamp)
                  }}</time>
                </div>
                <button
                  class="icon-button"
                  type="button"
                  :title="$t('events.viewDetails')"
                  :aria-label="$t('events.viewEventDetails', { id: event.id })"
                  @click.stop="selectedEvent = event"
                >
                  <svg viewBox="0 0 24 24" aria-hidden="true">
                    <circle cx="12" cy="12" r="9" />
                    <path d="M12 11v6M12 7h.01" />
                  </svg>
                </button>
              </header>
              <button
                class="event-mobile-item__target"
                type="button"
                :disabled="event.entity_id === null"
                @click.stop="filterByEntity(event)"
              >
                {{ entityTargetLabel(event) }}
              </button>
              <button
                class="event-mobile-item__actor"
                type="button"
                :disabled="event.user_id === null"
                @click.stop="filterByActor(event)"
              >
                {{ actorLabel(event) }}
              </button>
              <div class="event-mobile-item__action">
                <button
                  type="button"
                  class="event-action-pill"
                  :class="`event-action-pill--${eventActionTone(event.action)}`"
                  @click.stop="filterByAction(event.action)"
                >
                  {{ eventActionLabel(event.action) }}
                </button>
                <small>{{ $t('events.eventId', { id: event.id }) }}</small>
              </div>
              <p>{{ eventSummary(event) }}</p>
            </article>
          </div>

          <div ref="loadMoreSentinel" class="events-load-more" aria-live="polite">
            <span v-if="loadingMore" role="status">{{ $t('events.loadingMore') }}</span>
            <button
              v-else-if="loadMoreError"
              class="secondary-button"
              type="button"
              @click="loadNextPage"
            >
              {{ $t('events.loadMoreFailedRetry') }}
            </button>
            <span v-else-if="hasMoreEvents">{{ $t('events.scrollForMore') }}</span>
            <span v-else>{{ $t('events.allLoaded', { n: total }) }}</span>
          </div>
        </template>
      </div>
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
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";
import { useRoute, useRouter, type LocationQueryRaw } from "vue-router";
import { useI18n } from "vue-i18n";
import { listEvents, type EventListQuery, type EventLogResponse } from "../api/events";
import { ApiConfigurationError, ApiError, ApiNetworkError, ApiResponseError } from "../api/errors";
import { translateMessageOrNull } from "../i18n";
import EventDetailDialog from "../components/events/EventDetailDialog.vue";
import EventFilterDialog, {
  type EventAdvancedFilterValue,
} from "../components/events/EventFilterDialog.vue";
import SelectControl from "../components/forms/SelectControl.vue";
import { useStablePendingIndicator } from "../composables/useStablePendingIndicator";
import { notice } from "../notices/notice";
import {
  CUSTOM_EVENT_FILTER,
  eventActionLabel,
  eventActionOptions,
  eventActionTone,
  eventEntityLabel,
  eventEntityOptions,
  isKnownAction,
  isKnownEntityType,
} from "./events/catalog";
import { eventSummary } from "./events/details";

interface EventPageState {
  entityType: string;
  action: string;
  entityId: number | null;
  userId: number | null;
  dateFrom: string;
  dateTo: string;
  page: number;
  pageSize: number;
}

type FilterKey = "entityType" | "action" | "entityId" | "userId" | "dateFrom" | "dateTo";

const route = useRoute();
const router = useRouter();
const { t } = useI18n();
const state = reactive<EventPageState>(defaultState());
const events = ref<EventLogResponse[]>([]);
const total = ref(0);
const totalPages = ref(0);
const loaded = ref(false);
const loading = ref(false);
const loadingMore = ref(false);
const loadError = ref("");
const loadMoreError = ref("");
const loadMoreSentinel = ref<HTMLElement | null>(null);
const filterDialogOpen = ref(false);
const selectedEvent = ref<EventLogResponse | null>(null);
const entitySelectValue = ref("");
const actionSelectValue = ref("");
let requestController: AbortController | null = null;
let loadMoreObserver: IntersectionObserver | null = null;

const requestPending = computed(() => loading.value || loadingMore.value);
const hasMoreEvents = computed(() => state.page < totalPages.value);
const showInitialLoading = useStablePendingIndicator(
  computed(() => loading.value && !loaded.value),
  { showDelayMs: 200, minimumVisibleMs: 350 },
);
const showStableRefreshing = useStablePendingIndicator(
  computed(() => loading.value && loaded.value),
  { showDelayMs: 200, minimumVisibleMs: 350 },
);
const advancedFilterCount = computed(
  () =>
    [
      state.entityId,
      state.userId,
      Boolean(state.dateFrom),
      Boolean(state.dateTo),
      state.pageSize !== 50,
      !isKnownEntityType(state.entityType) && Boolean(state.entityType),
      !isKnownAction(state.action) && Boolean(state.action),
    ].filter(Boolean).length,
);
const advancedFilterValue = computed<EventAdvancedFilterValue>(() => ({
  entityId: state.entityId,
  userId: state.userId,
  customEntityType:
    state.entityType && !isKnownEntityType(state.entityType) ? state.entityType : "",
  customAction: state.action && !isKnownAction(state.action) ? state.action : "",
  dateFrom: isoToLocalInput(state.dateFrom),
  dateTo: isoToLocalInput(state.dateTo),
  pageSize: state.pageSize,
}));
const activeFilters = computed(() => {
  const values: Array<{ key: FilterKey; label: string; title?: string }> = [];
  if (state.entityType)
    values.push({
      key: "entityType",
      label: t("events.filterEntity", { label: eventEntityLabel(state.entityType) }),
    });
  if (state.action)
    values.push({
      key: "action",
      label: t("events.filterAction", { label: eventActionLabel(state.action) }),
    });
  if (state.entityId !== null)
    values.push({ key: "entityId", label: t("events.filterEntityId", { id: state.entityId }) });
  if (state.userId !== null)
    values.push({ key: "userId", label: t("events.filterActorId", { id: state.userId }) });
  if (state.dateFrom)
    values.push({
      key: "dateFrom",
      label: t("events.filterFrom", { time: formatFilterTimestamp(state.dateFrom) }),
      title: t("events.filterFrom", { time: formatLocalTimestamp(state.dateFrom) }),
    });
  if (state.dateTo)
    values.push({
      key: "dateTo",
      label: t("events.filterTo", { time: formatFilterTimestamp(state.dateTo) }),
      title: t("events.filterTo", { time: formatLocalTimestamp(state.dateTo) }),
    });
  return values;
});

watch(
  () => route.fullPath,
  () => {
    Object.assign(state, stateFromQuery(route.query));
    syncInputsFromState();
    const normalizedQuery = queryFromState(state);
    if (queryFingerprint(route.query) !== queryFingerprint(normalizedQuery)) {
      void router.replace({ name: "events", query: normalizedQuery });
      return;
    }
    void loadCurrentPage();
  },
  { immediate: true },
);

watch(loadMoreSentinel, (element, previousElement) => {
  if (previousElement) loadMoreObserver?.unobserve(previousElement);
  if (element) loadMoreObserver?.observe(element);
});

onMounted(() => {
  loadMoreObserver = new IntersectionObserver(handleLoadMoreIntersection, {
    rootMargin: "240px 0px",
  });
  if (loadMoreSentinel.value) loadMoreObserver.observe(loadMoreSentinel.value);
});

onBeforeUnmount(() => {
  requestController?.abort();
  loadMoreObserver?.disconnect();
});

async function loadCurrentPage(): Promise<boolean> {
  return loadEvents(1);
}

async function loadEvents(targetPage: number, append = false): Promise<boolean> {
  requestController?.abort();
  const controller = new AbortController();
  requestController = controller;
  const shouldAppend = append && events.value.length > 0;
  loading.value = !shouldAppend;
  loadingMore.value = shouldAppend;
  loadMoreError.value = "";
  if (!shouldAppend) loadError.value = "";
  try {
    const response = await listEvents(eventQuery(targetPage), controller.signal);
    events.value = shouldAppend ? mergeEvents(events.value, response.items) : response.items;
    total.value = response.total;
    totalPages.value = response.total_pages;
    state.page = response.page;
    loaded.value = true;
    return true;
  } catch (error) {
    if (error instanceof DOMException && error.name === "AbortError") return false;
    const message = eventErrorMessage(error);
    if (shouldAppend) loadMoreError.value = message;
    else loadError.value = message;
    const title = shouldAppend
      ? t("events.loadMoreFailed")
      : loaded.value
        ? t("events.refreshFailed")
        : t("events.loadFailed");
    notice.error(title, { detail: message });
    return false;
  } finally {
    if (requestController === controller) {
      requestController = null;
      loading.value = false;
      loadingMore.value = false;
      void nextTick().then(refreshLoadMoreObservation);
    }
  }
}

async function refreshCurrentPage(): Promise<void> {
  if (await loadCurrentPage()) notice.success(t("events.refreshSuccess"));
}

function eventQuery(targetPage: number): EventListQuery {
  return {
    page: targetPage,
    page_size: state.pageSize,
    entity_type: state.entityType || undefined,
    entity_id: state.entityId ?? undefined,
    action: state.action || undefined,
    user_id: state.userId ?? undefined,
    date_from: state.dateFrom || undefined,
    date_to: state.dateTo || undefined,
  };
}

function handleLoadMoreIntersection(entries: IntersectionObserverEntry[]): void {
  if (entries.some((entry) => entry.isIntersecting)) void loadNextPage();
}

async function loadNextPage(): Promise<void> {
  if (requestPending.value || !hasMoreEvents.value) return;
  await loadEvents(state.page + 1, true);
}

function refreshLoadMoreObservation(): void {
  const sentinel = loadMoreSentinel.value;
  if (!sentinel || !loadMoreObserver) return;
  loadMoreObserver.unobserve(sentinel);
  loadMoreObserver.observe(sentinel);
}

function mergeEvents(
  current: EventLogResponse[],
  incoming: EventLogResponse[],
): EventLogResponse[] {
  const ids = new Set(current.map((event) => event.id));
  const merged = [...current];
  for (const event of incoming) {
    if (ids.has(event.id)) continue;
    ids.add(event.id);
    merged.push(event);
  }
  return merged;
}

function changeEntityType(value: string): void {
  const next = value;
  if (next === CUSTOM_EVENT_FILTER) {
    filterDialogOpen.value = true;
    syncInputsFromState();
    return;
  }
  void navigate({ entityType: next, page: 1 });
}

function changeAction(value: string): void {
  const next = value;
  if (next === CUSTOM_EVENT_FILTER) {
    filterDialogOpen.value = true;
    syncInputsFromState();
    return;
  }
  void navigate({ action: next, page: 1 });
}

function applyAdvancedFilters(value: EventAdvancedFilterValue): void {
  const entityType =
    value.customEntityType || (isKnownEntityType(state.entityType) ? state.entityType : "");
  const action = value.customAction || (isKnownAction(state.action) ? state.action : "");
  const dateFrom = localInputToIso(value.dateFrom);
  const dateTo = localInputToIso(value.dateTo);
  if (
    (value.dateFrom && !dateFrom) ||
    (value.dateTo && !dateTo) ||
    (dateFrom && dateTo && dateFrom > dateTo)
  ) {
    notice.warning(t("events.invalidTimeRange"));
    return;
  }
  filterDialogOpen.value = false;
  void navigate({
    entityType,
    action,
    entityId: value.entityId,
    userId: value.userId,
    dateFrom,
    dateTo,
    pageSize: value.pageSize,
    page: 1,
  });
}

function closeFilterDialog(): void {
  filterDialogOpen.value = false;
  syncInputsFromState();
}

function filterByActor(event: EventLogResponse): void {
  if (event.user_id !== null) void navigate({ userId: event.user_id, page: 1 });
}

function filterByEntity(event: EventLogResponse): void {
  if (event.entity_id !== null)
    void navigate({ entityType: event.entity_type, entityId: event.entity_id, page: 1 });
}

function filterByAction(action: string): void {
  void navigate({ action, page: 1 });
}

function showRelatedEvents(event: EventLogResponse): void {
  selectedEvent.value = null;
  filterByEntity(event);
}

function clearFilter(key: FilterKey): void {
  const patch: Partial<EventPageState> = { page: 1 };
  if (key === "entityType") patch.entityType = "";
  if (key === "action") patch.action = "";
  if (key === "entityId") patch.entityId = null;
  if (key === "userId") patch.userId = null;
  if (key === "dateFrom") patch.dateFrom = "";
  if (key === "dateTo") patch.dateTo = "";
  void navigate(patch);
}

function clearAllFilters(): void {
  void navigate({ ...defaultState(), pageSize: state.pageSize });
}

async function navigate(patch: Partial<EventPageState>): Promise<void> {
  const next = { ...state, ...patch };
  const query = queryFromState(next);
  if (queryFingerprint(route.query) === queryFingerprint(query)) {
    await loadCurrentPage();
    return;
  }
  await router.replace({ name: "events", query });
}

function syncInputsFromState(): void {
  entitySelectValue.value = state.entityType
    ? isKnownEntityType(state.entityType)
      ? state.entityType
      : CUSTOM_EVENT_FILTER
    : "";
  actionSelectValue.value = state.action
    ? isKnownAction(state.action)
      ? state.action
      : CUSTOM_EVENT_FILTER
    : "";
}

function stateFromQuery(query: Record<string, unknown>): EventPageState {
  return {
    entityType: textQuery(query.entity_type),
    action: textQuery(query.action),
    entityId: positiveIntegerQuery(query.entity_id),
    userId: positiveIntegerQuery(query.user_id),
    dateFrom: validDateQuery(query.date_from),
    dateTo: validDateQuery(query.date_to),
    page: 1,
    pageSize: allowedPageSize(positiveIntegerQuery(query.page_size)),
  };
}

function queryFromState(value: EventPageState): LocationQueryRaw {
  const query: LocationQueryRaw = {};
  if (value.entityType) query.entity_type = value.entityType;
  if (value.action) query.action = value.action;
  if (value.entityId !== null) query.entity_id = String(value.entityId);
  if (value.userId !== null) query.user_id = String(value.userId);
  if (value.dateFrom) query.date_from = value.dateFrom;
  if (value.dateTo) query.date_to = value.dateTo;
  if (value.pageSize !== 50) query.page_size = String(value.pageSize);
  return query;
}

function queryFingerprint(query: Record<string, unknown>): string {
  return Object.entries(query)
    .flatMap(([key, value]) =>
      (Array.isArray(value) ? value : [value]).map(
        (item) => [key, item == null ? "" : String(item)] as const,
      ),
    )
    .sort(
      ([leftKey, leftValue], [rightKey, rightValue]) =>
        leftKey.localeCompare(rightKey) || leftValue.localeCompare(rightValue),
    )
    .map(([key, value]) => `${encodeURIComponent(key)}=${encodeURIComponent(value)}`)
    .join("&");
}

function defaultState(): EventPageState {
  return {
    entityType: "",
    action: "",
    entityId: null,
    userId: null,
    dateFrom: "",
    dateTo: "",
    page: 1,
    pageSize: 50,
  };
}

function textQuery(value: unknown): string {
  const current = Array.isArray(value) ? value[0] : value;
  return typeof current === "string" ? current.trim().slice(0, 64) : "";
}

function positiveIntegerQuery(value: unknown): number | null {
  const current = Array.isArray(value) ? value[0] : value;
  const parsed =
    typeof current === "string" ? Number(current) : typeof current === "number" ? current : NaN;
  return Number.isInteger(parsed) && parsed > 0 ? parsed : null;
}

function validDateQuery(value: unknown): string {
  const current = textQuery(value);
  return current && !Number.isNaN(new Date(current).getTime())
    ? new Date(current).toISOString()
    : "";
}

function allowedPageSize(value: number | null): number {
  return value !== null && [25, 50, 100, 200].includes(value) ? value : 50;
}

function localInputToIso(value: string): string {
  if (!value) return "";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? "" : date.toISOString();
}

function isoToLocalInput(value: string): string {
  if (!value) return "";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "";
  const offset = date.getTimezoneOffset() * 60_000;
  return new Date(date.getTime() - offset).toISOString().slice(0, 19);
}

function formatLocalTimestamp(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime())
    ? value
    : new Intl.DateTimeFormat("zh-CN", {
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
        hour12: false,
      }).format(date);
}

function formatFilterTimestamp(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime())
    ? value
    : new Intl.DateTimeFormat("zh-CN", {
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
        hour12: false,
      }).format(date);
}

function actorLabel(event: EventLogResponse): string {
  if (event.username)
    return event.user_id === null ? event.username : `${event.username} · #${event.user_id}`;
  return event.user_id === null
    ? t("events.systemUnknownActor")
    : t("events.userActor", { id: event.user_id });
}

function entityTargetLabel(event: EventLogResponse): string {
  const id = event.entity_id === null ? t("events.noEntityId") : `#${event.entity_id}`;
  return `${eventEntityLabel(event.entity_type)} · ${id}`;
}

function eventErrorMessage(error: unknown): string {
  if (error instanceof ApiError) {
    return translateMessageOrNull(`error.${error.code}`) ?? error.message;
  }
  if (error instanceof ApiConfigurationError) return error.message;
  if (error instanceof ApiNetworkError) return error.message;
  if (error instanceof ApiResponseError) return error.message;
  return t("events.genericLoadFailed");
}
</script>

<style lang="scss" src="./EventsPage.scss"></style>
