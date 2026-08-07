<!-- 本组件拥有库存审批页的查询、URL、分页、详情会话和审批结果协调；领域差异由审批目录与详情组件提供。 -->
<template>
  <section class="route-page approval-page">
    <header class="content-header approval-page__header">
      <div>
        <h1>{{ $route.meta.title }}</h1>
        <p>{{ catalog.pageSubtitle }}</p>
      </div>
    </header>
    <section class="approval-workspace" :aria-label="`${$route.meta.title}队列`">
      <div class="approval-toolbar">
        <SearchField
          v-model="searchInput"
          :label="`搜索${$route.meta.title}`"
          name="approval_search"
          :placeholder="catalog.searchPlaceholder"
          hide-label
          @search="applySearch"
        />
        <div class="approval-toolbar__meta">
          <span class="approval-count">待审批 {{ total }} 条</span>
          <div class="approval-toolbar__actions">
            <button
              class="icon-button approval-toolbar__filter"
              type="button"
              title="筛选待审批单据"
              :aria-expanded="filterOpen"
              @click="filterOpen = true"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M4 6h16M7 12h10M10 18h4" /></svg
              ><span v-if="filterCount">{{ filterCount }}</span></button
            ><button
              class="icon-button approval-toolbar__refresh"
              :class="{ 'approval-toolbar__refresh--pending': showRefreshing }"
              type="button"
              title="刷新审批队列"
              :disabled="requestPending"
              @click="refresh"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M20 7v5h-5" />
                <path d="M18.2 16a7 7 0 1 1 .8-7l1 3" />
              </svg>
            </button>
          </div>
        </div>
      </div>
      <div
        v-overlay-scrollbar
        class="approval-results"
        :class="{ 'approval-results--refreshing': showRefreshing }"
        :aria-busy="requestPending"
      >
        <section v-if="showInitialLoading" class="approval-state">正在加载待审批单据…</section>
        <section v-else-if="loadError && !loaded" class="approval-state approval-state--error">
          <strong>无法加载审批队列</strong><span>{{ loadError }}</span
          ><button class="secondary-button" type="button" @click="refresh">重试</button>
        </section>
        <section v-else-if="loaded && records.length === 0" class="approval-state">
          <strong>{{ hasFilters ? catalog.noResultLabel : catalog.emptyLabel }}</strong
          ><button v-if="hasFilters" class="text-button" type="button" @click="clearFilters">
            清除筛选
          </button>
        </section>
        <template v-else
          ><p v-if="loadError" class="approval-inline-error" role="status">
            {{ loadError }}
          </p>
          <ApprovalQueueList :records="records" :catalog="catalog" @open="openReview" />
          <div ref="sentinelElement" class="approval-load-more">
            <span v-if="loadingMore">正在加载更多待审批单据…</span
            ><button
              v-else-if="loadMoreError"
              class="secondary-button"
              type="button"
              @click="loadNextPage"
            >
              加载失败，点击重试</button
            ><span v-else-if="hasMore">继续向下滚动加载</span
            ><span v-else>已加载全部 {{ total }} 条待审批单据</span>
          </div></template
        >
      </div>
    </section>
    <ApprovalDateFiltersDialog
      :open="filterOpen"
      :value="filterValue"
      @close="filterOpen = false"
      @apply="applyFilters"
    />
    <ApprovalReviewDialog
      :open="selected !== null"
      :record="selected"
      :catalog="catalog"
      :detail-loading="detailLoading"
      :detail-error="detailError"
      :action-busy="actionBusy"
      :action-error="actionError"
      @close="closeReview"
      @reload="loadDetail"
      @act="performAction"
    />
  </section>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { ApiError } from "../../api/errors";
import { authSession, refreshAuthSession } from "../../auth/session";
import { notice } from "../../notices/notice";
import {
  approvalId,
  getApprovalCatalog,
  type ApprovalKind,
  type ApprovalRecord,
} from "../../pages/approvals/catalog";
import { getDefaultAppRouteName } from "../../router/navigation";
import { useStablePendingIndicator } from "../../composables/useStablePendingIndicator";
import SearchField from "../SearchField.vue";
import ApprovalDateFiltersDialog, {
  type ApprovalDateFilterValue,
} from "./ApprovalDateFiltersDialog.vue";
import ApprovalQueueList from "./ApprovalQueueList.vue";
import ApprovalReviewDialog from "./ApprovalReviewDialog.vue";
import "./StockApprovalWorkspace.scss";

const props = defineProps<{ kind: ApprovalKind }>();
const route = useRoute(),
  router = useRouter(),
  catalog = computed(() => getApprovalCatalog(props.kind));
const records = ref<ApprovalRecord[]>([]),
  total = ref(0),
  totalPages = ref(0),
  currentPage = ref(1),
  loaded = ref(false);
const loading = ref(false),
  loadingMore = ref(false),
  loadError = ref(""),
  loadMoreError = ref("");
const searchInput = ref(""),
  dateFrom = ref(""),
  dateTo = ref(""),
  filterOpen = ref(false);
const selected = ref<ApprovalRecord | null>(null),
  detailLoading = ref(false),
  detailError = ref(""),
  actionBusy = ref(false),
  actionError = ref("");
const sentinelElement = ref<HTMLElement | null>(null);
let listController: AbortController | null = null,
  detailController: AbortController | null = null,
  observer: IntersectionObserver | null = null;

const requestPending = computed(() => loading.value || loadingMore.value);
const hasMore = computed(() => currentPage.value < totalPages.value);
const hasFilters = computed(() => Boolean(searchInput.value || dateFrom.value || dateTo.value));
const filterCount = computed(() => [dateFrom.value, dateTo.value].filter(Boolean).length);
const filterValue = computed<ApprovalDateFilterValue>(() => ({
  dateFrom: isoToLocal(dateFrom.value),
  dateTo: isoToLocal(dateTo.value),
}));
const showInitialLoading = useStablePendingIndicator(
  computed(() => loading.value && !loaded.value),
  { showDelayMs: 200, minimumVisibleMs: 350 },
);
const showRefreshing = useStablePendingIndicator(
  computed(() => loading.value && loaded.value),
  { showDelayMs: 200, minimumVisibleMs: 350 },
);

watch(
  () => route.fullPath,
  () => {
    searchInput.value = stringQuery(route.query.search);
    dateFrom.value = dateQuery(route.query.date_from);
    dateTo.value = dateQuery(route.query.date_to);
    void loadPage(1);
  },
  { immediate: true },
);
onMounted(() => {
  observer = new IntersectionObserver(
    (entries) => {
      if (entries.some((entry) => entry.isIntersecting)) void loadNextPage();
    },
    { rootMargin: "240px 0px" },
  );
  observeSentinel();
});
onBeforeUnmount(() => {
  listController?.abort();
  detailController?.abort();
  observer?.disconnect();
});

async function loadPage(page: number, append = false): Promise<boolean> {
  listController?.abort();
  const request = new AbortController();
  listController = request;
  const shouldAppend = append && records.value.length > 0;
  loading.value = !shouldAppend;
  loadingMore.value = shouldAppend;
  loadMoreError.value = "";
  if (!shouldAppend) loadError.value = "";
  try {
    const response = await catalog.value.list(
      {
        page,
        page_size: 50,
        search: stringQuery(route.query.search) || undefined,
        date_from: dateFrom.value || undefined,
        date_to: dateTo.value || undefined,
      },
      request.signal,
    );
    if (listController !== request) return false;
    records.value = shouldAppend ? mergeRecords(records.value, response.items) : response.items;
    total.value = response.total;
    totalPages.value = response.total_pages;
    currentPage.value = response.page;
    loaded.value = true;
    return true;
  } catch (cause) {
    if (cause instanceof DOMException && cause.name === "AbortError") return false;
    if (handlePermissionFailure(cause)) return false;
    const message = errorMessage(cause, "请检查服务连接后重试");
    if (shouldAppend) loadMoreError.value = message;
    else loadError.value = message;
    return false;
  } finally {
    if (listController === request) {
      listController = null;
      loading.value = false;
      loadingMore.value = false;
      void nextTick().then(observeSentinel);
    }
  }
}
async function refresh(): Promise<void> {
  if ((await loadPage(1)) && loaded.value) notice.success("审批队列已刷新");
}
async function loadNextPage(): Promise<void> {
  if (!requestPending.value && hasMore.value) await loadPage(currentPage.value + 1, true);
}
function observeSentinel(): void {
  if (!observer || !sentinelElement.value) return;
  observer.disconnect();
  observer.observe(sentinelElement.value);
}
function applySearch(value: string): void {
  updateQuery(value.trim(), dateFrom.value, dateTo.value);
}
function applyFilters(value: ApprovalDateFilterValue): void {
  const from = localToIso(value.dateFrom),
    to = localToIso(value.dateTo);
  if ((value.dateFrom && !from) || (value.dateTo && !to) || (from && to && from > to)) {
    notice.warning("请输入有效的创建时间范围");
    return;
  }
  filterOpen.value = false;
  updateQuery(stringQuery(route.query.search), from, to);
}
function clearFilters(): void {
  void router.replace({ query: {} });
}
function updateQuery(search: string, from: string, to: string): void {
  void router.replace({
    query: {
      search: search || undefined,
      date_from: from || undefined,
      date_to: to || undefined,
    },
  });
}

function openReview(record: ApprovalRecord): void {
  selected.value = record;
  actionError.value = "";
  void loadDetail();
}
function closeReview(): void {
  if (actionBusy.value) return;
  detailController?.abort();
  selected.value = null;
  detailError.value = "";
  actionError.value = "";
}
async function loadDetail(): Promise<void> {
  if (!selected.value) return;
  const id = approvalId(selected.value);
  detailController?.abort();
  const request = new AbortController();
  detailController = request;
  detailLoading.value = true;
  detailError.value = "";
  try {
    const detail = await catalog.value.get(id, request.signal);
    if (detailController === request && selected.value && approvalId(selected.value) === id) {
      selected.value = detail;
      if (detail.order.status !== "pending") {
        notice.info("该单据已由其他操作处理");
        await advanceAfterAction(id);
      }
    }
  } catch (cause) {
    if (
      !(cause instanceof DOMException && cause.name === "AbortError") &&
      detailController === request
    ) {
      if (cause instanceof ApiError && cause.status === 404) {
        notice.info("该单据已不存在或不可读取");
        await advanceAfterAction(id);
      } else if (!handlePermissionFailure(cause))
        detailError.value = errorMessage(cause, "无法加载完整单据");
    }
  } finally {
    if (detailController === request) {
      detailController = null;
      detailLoading.value = false;
    }
  }
}
async function performAction(action: "approve" | "reject"): Promise<void> {
  if (!selected.value || actionBusy.value) return;
  const id = approvalId(selected.value);
  actionBusy.value = true;
  actionError.value = "";
  try {
    const result =
      action === "approve" ? await catalog.value.approve(id) : await catalog.value.reject(id);
    const title =
      action === "approve"
        ? `${result.kind === "inbound" ? "入库单" : "出库单"} #${id} 已通过`
        : `${result.kind === "inbound" ? "入库单" : "出库单"} #${id} 已拒绝`;
    notice.success(title, {
      detail:
        action === "approve"
          ? result.kind === "inbound"
            ? "库存已增加。"
            : "库存已扣减。"
          : "库存未变更。",
    });
    await advanceAfterAction(id);
  } catch (cause) {
    if (handlePermissionFailure(cause)) return;
    if (cause instanceof ApiError && cause.status === 409 && cause.code === "order_not_pending") {
      notice.info("该单据已由其他审批人处理");
      await advanceAfterAction(id);
      return;
    }
    actionError.value = actionErrorMessage(cause);
    notice.error(action === "approve" ? "审批通过失败" : "拒绝单据失败", {
      detail: actionError.value,
    });
  } finally {
    actionBusy.value = false;
  }
}
async function advanceAfterAction(id: number): Promise<void> {
  const index = records.value.findIndex((record) => approvalId(record) === id);
  records.value = records.value.filter((record) => approvalId(record) !== id);
  total.value = Math.max(0, total.value - 1);
  const next = records.value[Math.min(Math.max(index, 0), records.value.length - 1)] ?? null;
  selected.value = next;
  detailError.value = "";
  actionError.value = "";
  await reconcileQueue(id);
  if (selected.value) void loadDetail();
}
async function reconcileQueue(removedId: number): Promise<void> {
  try {
    const response = await catalog.value.list({
      page: 1,
      page_size: 50,
      search: stringQuery(route.query.search) || undefined,
      date_from: dateFrom.value || undefined,
      date_to: dateTo.value || undefined,
    });
    records.value = mergeRecords(
      response.items.filter((record) => approvalId(record) !== removedId),
      records.value,
    );
    total.value = response.total;
    totalPages.value = response.total_pages;
    currentPage.value = 1;
  } catch {
    notice.warning("队列暂未完全同步", {
      detail: "当前审批结果已保存，可手动刷新队列。",
    });
  }
}
function mergeRecords(first: ApprovalRecord[], second: ApprovalRecord[]): ApprovalRecord[] {
  const ids = new Set<number>();
  return [...first, ...second].filter((record) => {
    const id = approvalId(record);
    if (ids.has(id)) return false;
    ids.add(id);
    return true;
  });
}
function actionErrorMessage(cause: unknown): string {
  if (!(cause instanceof ApiError)) return "无法连接服务，请稍后重试";
  if (cause.code === "insufficient_stock")
    return "库存不足或指定批次不可用，服务端已回滚整张出库单。";
  if (cause.code === "inbound_file_unavailable")
    return "入库附件已不可用，请拒绝后由创建方重新提交。";
  if (cause.code === "order_not_pending") return "该单据已由其他操作处理。";
  return cause.message;
}
function handlePermissionFailure(cause: unknown): boolean {
  if (!(cause instanceof ApiError) || cause.status !== 403) return false;
  notice.warning("审批权限已变化", {
    detail: "正在返回当前会话仍可访问的页面。",
  });
  detailController?.abort();
  selected.value = null;
  detailError.value = "";
  actionError.value = "";
  void refreshAuthSession().finally(() =>
    router.replace({
      name: getDefaultAppRouteName(authSession.value?.user.permissions),
    }),
  );
  return true;
}
function errorMessage(cause: unknown, fallback: string): string {
  return cause instanceof ApiError ? cause.message : fallback;
}
function stringQuery(value: unknown): string {
  return typeof value === "string" ? value : "";
}
function dateQuery(value: unknown): string {
  const raw = stringQuery(value);
  return raw && !Number.isNaN(new Date(raw).getTime()) ? new Date(raw).toISOString() : "";
}
function localToIso(value: string): string {
  if (!value) return "";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? "" : date.toISOString();
}
function isoToLocal(value: string): string {
  if (!value) return "";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "";
  return new Date(date.getTime() - date.getTimezoneOffset() * 60_000).toISOString().slice(0, 19);
}
</script>
