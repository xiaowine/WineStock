<!--
  本文件拥有库存总览页面的数据加载、趋势周期和只读指标编排，属于 frontend 页面层。
  它只通过 HTTP API 读取服务端统计，不自行推导库存、价值或呆滞规则。
-->
<template>
  <section class="route-page dashboard-page">
    <header class="content-header dashboard-page__header">
      <div class="dashboard-page__heading">
        <h1>{{ $title($route.meta.title) }}</h1>
        <p>{{ $t('dashboard.subtitle') }}</p>
      </div>
      <button
        class="icon-button dashboard-page__refresh"
        :class="{ 'dashboard-page__refresh--pending': showDashboardRefreshing }"
        type="button"
        :title="$t('dashboard.refresh')"
        :aria-label="$t('dashboard.refresh')"
        :aria-busy="refreshing"
        :disabled="refreshing"
        @click="refreshDashboard"
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M20 7v5h-5" />
          <path d="M18.2 16a7 7 0 1 1 .8-7l1 3" />
        </svg>
      </button>
      <span v-if="showDashboardRefreshing" class="visually-hidden" role="status">
        {{ $t('dashboard.refreshing') }}
      </span>
    </header>

    <section
      v-if="contactEntryVisible"
      class="dashboard-contact-banner"
      :aria-label="$t('dashboard.contactFeedback')"
    >
      <div class="dashboard-contact-banner__icon" aria-hidden="true">
        <svg viewBox="0 0 24 24" focusable="false">
          <path d="M4 5.5h16v11H8l-4 3v-14Z" />
          <path d="M8 10h8M8 13h5" />
        </svg>
      </div>
      <div class="dashboard-contact-banner__copy">
        <strong>{{ $t('dashboard.contactBannerTitle') }}</strong>
        <span>{{ $t('dashboard.contactBannerText') }}</span>
      </div>
      <button class="secondary-button" type="button" @click="openContactDialog">
        {{ $t('dashboard.contactFeedback') }}
      </button>
    </section>

    <div v-if="showInitialLoading" class="dashboard-page__initial-state" role="status">
      {{ $t('dashboard.loadingOverview') }}
    </div>

    <section
      v-else-if="initialError"
      class="dashboard-page__state dashboard-page__state--error"
      role="alert"
    >
      <h2>{{ $t('dashboard.loadFailedTitle') }}</h2>
      <p>{{ initialError }}</p>
      <button class="secondary-button" type="button" @click="loadDashboard">
        {{ $t('dashboard.retry') }}
      </button>
    </section>

    <template v-else-if="overview">
      <section class="dashboard-summary" :aria-label="$t('dashboard.summary')">
        <article class="dashboard-summary-card">
          <span>{{ $t('dashboard.itemKinds') }}</span>
          <strong>{{ formatInteger(overview.total_items) }}</strong>
          <small>{{ $t('dashboard.activeItems') }}</small>
        </article>
        <article class="dashboard-summary-card">
          <span>{{ $t('dashboard.totalStock') }}</span>
          <strong>{{ formatNumber(overview.total_quantity) }}</strong>
          <small>{{ $t('dashboard.batchRemaining') }}</small>
        </article>
        <article class="dashboard-summary-card">
          <span>{{ $t('dashboard.stockValue') }}</span>
          <strong>{{ formatNumber(overview.total_value) }}</strong>
          <small>{{ $t('dashboard.valueEstimate') }}</small>
        </article>
        <article class="dashboard-summary-card dashboard-summary-card--activity">
          <span>{{ $t('dashboard.recentActivity') }}</span>
          <div>
            <p>
              <small>{{ $t('dashboard.inbound') }}</small
              ><strong>{{ formatNumber(overview.inbound_3d) }}</strong>
            </p>
            <p>
              <small>{{ $t('dashboard.outbound') }}</small
              ><strong>{{ formatNumber(overview.outbound_3d) }}</strong>
            </p>
          </div>
        </article>
      </section>

      <section class="dashboard-panel dashboard-trend-panel">
        <header class="dashboard-panel__header">
          <div>
            <h2>{{ $t('dashboard.trendTitle') }}</h2>
            <p>{{ $t('dashboard.trendSubtitle') }}</p>
          </div>
          <div
            class="dashboard-period-control"
            role="group"
            :aria-label="$t('dashboard.trendPeriodRange')"
          >
            <button
              v-for="option in periodOptions"
              :key="option"
              type="button"
              :class="{ 'dashboard-period-control__active': trendDays === option }"
              :aria-pressed="trendDays === option"
              :disabled="loadingTrends"
              @click="selectTrendDays(option)"
            >
              {{ $t('dashboard.days', { n: option }) }}
            </button>
          </div>
        </header>
        <DashboardTrendChart :daily="trends" :days="trendDays" :loading="loadingTrends" />
      </section>

      <section class="dashboard-panel dashboard-slow-moving">
        <header class="dashboard-panel__header">
          <div>
            <h2>{{ $t('dashboard.slowMovingTitle') }}</h2>
            <p>{{ $t('dashboard.slowMovingSubtitle') }}</p>
          </div>
          <span class="dashboard-panel__count">
            {{ $t('dashboard.itemCount', { n: overview.slow_moving_items.length }) }}
          </span>
        </header>

        <div v-if="visibleSlowMovingItems.length === 0" class="dashboard-slow-moving__empty">
          {{ $t('dashboard.noSlowMovingItems') }}
        </div>

        <template v-else>
          <div v-overlay-scrollbar class="dashboard-slow-moving__table-wrap">
            <table class="dashboard-slow-moving__table">
              <thead>
                <tr>
                  <th scope="col">{{ $t('dashboard.item') }}</th>
                  <th scope="col">{{ $t('dashboard.currentStock') }}</th>
                  <th scope="col">{{ $t('dashboard.stockValue') }}</th>
                  <th scope="col">{{ $t('dashboard.inactive') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="item in visibleSlowMovingItems" :key="item.item_id">
                  <td>
                    <strong :title="item.item_name">{{ item.item_name }}</strong
                    ><small>#{{ item.item_id }}</small>
                  </td>
                  <td>{{ formatNumber(item.quantity) }}</td>
                  <td>{{ formatNumber(item.value) }}</td>
                  <td>
                    <span class="dashboard-age-pill">{{
                      $t('dashboard.days', { n: item.days_since_last_movement })
                    }}</span>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <div class="dashboard-slow-moving__mobile-list">
            <article v-for="item in visibleSlowMovingItems" :key="item.item_id">
              <header>
                <div>
                  <strong :title="item.item_name">{{ item.item_name }}</strong
                  ><small>#{{ item.item_id }}</small>
                </div>
                <span class="dashboard-age-pill">{{
                  $t('dashboard.days', { n: item.days_since_last_movement })
                }}</span>
              </header>
              <dl>
                <div>
                  <dt>{{ $t('dashboard.currentStock') }}</dt>
                  <dd>{{ formatNumber(item.quantity) }}</dd>
                </div>
                <div>
                  <dt>{{ $t('dashboard.stockValue') }}</dt>
                  <dd>{{ formatNumber(item.value) }}</dd>
                </div>
              </dl>
            </article>
          </div>

          <p
            v-if="overview.slow_moving_items.length > visibleSlowMovingItems.length"
            class="dashboard-slow-moving__footnote"
          >
            {{ $t('dashboard.showingFirstN', { n: visibleSlowMovingItems.length }) }}
          </p>
        </template>
      </section>
    </template>
  </section>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import {
  getDashboardOverview,
  getDashboardTrends,
  type DailyTrend,
  type DashboardOverviewResponse,
} from "../api/dashboard";
import { ApiConfigurationError, ApiError, ApiNetworkError, ApiResponseError } from "../api/errors";
import { useI18n } from "vue-i18n";
import DashboardTrendChart from "../components/dashboard/DashboardTrendChart.vue";
import { openContactDialog } from "../contact/contactDialog";
import { contactEntryVisible } from "../contact/contactPreferences";
import { useStablePendingIndicator } from "../composables/useStablePendingIndicator";
import { notice } from "../notices/notice";

const { t } = useI18n();

const SLOW_MOVING_LIMIT = 8;
const periodOptions = [7, 30, 90] as const;
type TrendDays = (typeof periodOptions)[number];

const overview = ref<DashboardOverviewResponse | null>(null);
const trends = ref<DailyTrend[]>([]);
const trendDays = ref<TrendDays>(30);
const loadingOverview = ref(false);
const loadingTrends = ref(false);
const initialError = ref("");
let overviewAbortController: AbortController | null = null;
let trendsAbortController: AbortController | null = null;

const refreshing = computed(() => loadingOverview.value || loadingTrends.value);
const waitingForInitialData = computed(() => overview.value === null && refreshing.value);
const showInitialLoading = useStablePendingIndicator(waitingForInitialData, {
  showDelayMs: 200,
  minimumVisibleMs: 350,
});
const waitingForDashboardRefresh = computed(() => overview.value !== null && refreshing.value);
const showDashboardRefreshing = useStablePendingIndicator(waitingForDashboardRefresh, {
  showDelayMs: 200,
  minimumVisibleMs: 350,
});
const visibleSlowMovingItems = computed(
  () => overview.value?.slow_moving_items.slice(0, SLOW_MOVING_LIMIT) ?? [],
);

onMounted(loadDashboard);
onBeforeUnmount(() => {
  overviewAbortController?.abort();
  trendsAbortController?.abort();
});

/** 并行加载摘要和趋势；首次失败显示页面级重试，刷新失败保留已有数据。 */
async function loadDashboard(): Promise<void> {
  initialError.value = "";
  const results = await Promise.allSettled([loadOverview(), loadTrends()]);
  const failure = results.find((result) => result.status === "rejected");
  if (failure?.status === "rejected" && overview.value === null) {
    initialError.value = dashboardErrorMessage(failure.reason, t("dashboard.loadOverviewFailed"));
  }
}

/** 手动刷新全部总览数据；已有内容在请求期间继续保留。 */
async function refreshDashboard(): Promise<void> {
  const results = await Promise.allSettled([loadOverview(), loadTrends()]);
  const failure = results.find((result) => result.status === "rejected");
  if (failure?.status === "rejected") {
    notice.error(dashboardErrorMessage(failure.reason, t("dashboard.refreshFailed")));
    return;
  }
  notice.success(t("dashboard.refreshSuccess"));
}

async function loadOverview(): Promise<void> {
  overviewAbortController?.abort();
  const controller = new AbortController();
  overviewAbortController = controller;
  loadingOverview.value = true;
  try {
    overview.value = await getDashboardOverview(controller.signal);
  } finally {
    if (overviewAbortController === controller) {
      overviewAbortController = null;
      loadingOverview.value = false;
    }
  }
}

async function loadTrends(): Promise<void> {
  trendsAbortController?.abort();
  const controller = new AbortController();
  trendsAbortController = controller;
  loadingTrends.value = true;
  try {
    const response = await getDashboardTrends(trendDays.value, controller.signal);
    trends.value = response.daily;
  } finally {
    if (trendsAbortController === controller) {
      trendsAbortController = null;
      loadingTrends.value = false;
    }
  }
}

function selectTrendDays(days: TrendDays): void {
  if (trendDays.value === days) {
    return;
  }
  trendDays.value = days;
  void loadTrends().catch((error) => {
    notice.error(dashboardErrorMessage(error, t("dashboard.loadTrendsFailed")));
  });
}

function formatInteger(value: number): string {
  return new Intl.NumberFormat("zh-CN", { maximumFractionDigits: 0 }).format(value);
}

function formatNumber(value: number): string {
  return new Intl.NumberFormat("zh-CN", { maximumFractionDigits: 2 }).format(value);
}

function dashboardErrorMessage(error: unknown, fallback: string): string {
  if (error instanceof DOMException && error.name === "AbortError") {
    return fallback;
  }
  if (error instanceof ApiError) {
    if (error.code === "permission_denied") {
      return t("dashboard.permissionDenied");
    }
    return error.message;
  }
  if (error instanceof ApiConfigurationError) {
    return error.message;
  }
  if (error instanceof ApiNetworkError) {
    return t("dashboard.cannotConnect");
  }
  if (error instanceof ApiResponseError) {
    return t("dashboard.invalidResponseFormat");
  }
  return fallback;
}
</script>

<style lang="scss" src="./DashboardPage.scss"></style>
