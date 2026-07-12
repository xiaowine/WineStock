<!--
  本文件拥有库存总览页面的数据加载、趋势周期和只读指标编排，属于 frontend 页面层。
  它只通过 HTTP API 读取服务端统计，不自行推导库存、价值或呆滞规则。
-->
<template>
  <section class="route-page dashboard-page">
    <header class="content-header dashboard-page__header">
      <div class="dashboard-page__heading">
        <h1>总览</h1>
        <p>查看当前库存规模、近期流转和需要关注的呆滞物品。</p>
      </div>
      <button
        class="icon-button dashboard-page__refresh"
        :class="{ 'dashboard-page__refresh--pending': showDashboardRefreshing }"
        type="button"
        title="刷新库存总览"
        aria-label="刷新库存总览"
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
        正在刷新库存总览
      </span>
    </header>

    <div v-if="showInitialLoading" class="dashboard-page__initial-state" role="status">
      正在加载库存总览…
    </div>

    <section v-else-if="initialError" class="dashboard-page__state dashboard-page__state--error" role="alert">
      <h2>无法加载总览</h2>
      <p>{{ initialError }}</p>
      <button class="secondary-button" type="button" @click="loadDashboard">重试</button>
    </section>

    <template v-else-if="overview">
      <section class="dashboard-summary" aria-label="库存摘要">
        <article class="dashboard-summary-card">
          <span>物品种类</span>
          <strong>{{ formatInteger(overview.total_items) }}</strong>
          <small>当前有效物品</small>
        </article>
        <article class="dashboard-summary-card">
          <span>库存总量</span>
          <strong>{{ formatNumber(overview.total_quantity) }}</strong>
          <small>全部有效批次余量</small>
        </article>
        <article class="dashboard-summary-card">
          <span>库存价值</span>
          <strong>{{ formatNumber(overview.total_value) }}</strong>
          <small>按当前批次成本估算</small>
        </article>
        <article class="dashboard-summary-card dashboard-summary-card--activity">
          <span>近 3 天流转</span>
          <div>
            <p><small>入库</small><strong>{{ formatNumber(overview.inbound_3d) }}</strong></p>
            <p><small>出库</small><strong>{{ formatNumber(overview.outbound_3d) }}</strong></p>
          </div>
        </article>
      </section>

      <section class="dashboard-panel dashboard-trend-panel">
        <header class="dashboard-panel__header">
          <div>
            <h2>出入库趋势</h2>
            <p>只统计审批通过后生成的库存流水。</p>
          </div>
          <div class="dashboard-period-control" role="group" aria-label="趋势时间范围">
            <button
              v-for="option in periodOptions"
              :key="option"
              type="button"
              :class="{ 'dashboard-period-control__active': trendDays === option }"
              :aria-pressed="trendDays === option"
              :disabled="loadingTrends"
              @click="selectTrendDays(option)"
            >
              {{ option }} 天
            </button>
          </div>
        </header>
        <DashboardTrendChart
          :daily="trends"
          :days="trendDays"
          :loading="loadingTrends"
        />
      </section>

      <section class="dashboard-panel dashboard-slow-moving">
        <header class="dashboard-panel__header">
          <div>
            <h2>呆滞物品</h2>
            <p>当前有库存且 30 天内没有出入库流水的物品。</p>
          </div>
          <span class="dashboard-panel__count">
            {{ overview.slow_moving_items.length }} 项
          </span>
        </header>

        <div v-if="visibleSlowMovingItems.length === 0" class="dashboard-slow-moving__empty">
          当前没有需要关注的呆滞物品。
        </div>

        <template v-else>
          <div class="dashboard-slow-moving__table-wrap">
            <table class="dashboard-slow-moving__table">
              <thead>
                <tr>
                  <th scope="col">物品</th>
                  <th scope="col">当前库存</th>
                  <th scope="col">库存价值</th>
                  <th scope="col">未流转</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="item in visibleSlowMovingItems" :key="item.item_id">
                  <td><strong :title="item.item_name">{{ item.item_name }}</strong><small>#{{ item.item_id }}</small></td>
                  <td>{{ formatNumber(item.quantity) }}</td>
                  <td>{{ formatNumber(item.value) }}</td>
                  <td><span class="dashboard-age-pill">{{ item.days_since_last_movement }} 天</span></td>
                </tr>
              </tbody>
            </table>
          </div>

          <div class="dashboard-slow-moving__mobile-list">
            <article v-for="item in visibleSlowMovingItems" :key="item.item_id">
              <header>
                <div><strong :title="item.item_name">{{ item.item_name }}</strong><small>#{{ item.item_id }}</small></div>
                <span class="dashboard-age-pill">{{ item.days_since_last_movement }} 天</span>
              </header>
              <dl>
                <div><dt>当前库存</dt><dd>{{ formatNumber(item.quantity) }}</dd></div>
                <div><dt>库存价值</dt><dd>{{ formatNumber(item.value) }}</dd></div>
              </dl>
            </article>
          </div>

          <p v-if="overview.slow_moving_items.length > visibleSlowMovingItems.length" class="dashboard-slow-moving__footnote">
            当前展示前 {{ visibleSlowMovingItems.length }} 项。
          </p>
        </template>
      </section>
    </template>
  </section>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import {
  getDashboardOverview,
  getDashboardTrends,
  type DailyTrend,
  type DashboardOverviewResponse,
} from '../api/dashboard'
import {
  ApiConfigurationError,
  ApiError,
  ApiNetworkError,
  ApiResponseError,
} from '../api/errors'
import DashboardTrendChart from '../components/dashboard/DashboardTrendChart.vue'
import { useStablePendingIndicator } from '../composables/useStablePendingIndicator'
import { notice } from '../notices/notice'

const SLOW_MOVING_LIMIT = 8
const periodOptions = [7, 30, 90] as const
type TrendDays = (typeof periodOptions)[number]

const overview = ref<DashboardOverviewResponse | null>(null)
const trends = ref<DailyTrend[]>([])
const trendDays = ref<TrendDays>(30)
const loadingOverview = ref(false)
const loadingTrends = ref(false)
const initialError = ref('')
let overviewAbortController: AbortController | null = null
let trendsAbortController: AbortController | null = null

const refreshing = computed(() => loadingOverview.value || loadingTrends.value)
const waitingForInitialData = computed(() => overview.value === null && refreshing.value)
const showInitialLoading = useStablePendingIndicator(waitingForInitialData, {
  showDelayMs: 200,
  minimumVisibleMs: 350,
})
const waitingForDashboardRefresh = computed(() => overview.value !== null && refreshing.value)
const showDashboardRefreshing = useStablePendingIndicator(waitingForDashboardRefresh, {
  showDelayMs: 200,
  minimumVisibleMs: 350,
})
const visibleSlowMovingItems = computed(() =>
  overview.value?.slow_moving_items.slice(0, SLOW_MOVING_LIMIT) ?? [],
)

onMounted(loadDashboard)
onBeforeUnmount(() => {
  overviewAbortController?.abort()
  trendsAbortController?.abort()
})

/** 并行加载摘要和趋势；首次失败显示页面级重试，刷新失败保留已有数据。 */
async function loadDashboard(): Promise<void> {
  initialError.value = ''
  const results = await Promise.allSettled([loadOverview(), loadTrends()])
  const failure = results.find((result) => result.status === 'rejected')
  if (failure?.status === 'rejected' && overview.value === null) {
    initialError.value = dashboardErrorMessage(failure.reason, '加载库存总览失败')
  }
}

/** 手动刷新全部总览数据；已有内容在请求期间继续保留。 */
async function refreshDashboard(): Promise<void> {
  const results = await Promise.allSettled([loadOverview(), loadTrends()])
  const failure = results.find((result) => result.status === 'rejected')
  if (failure?.status === 'rejected') {
    notice.error(dashboardErrorMessage(failure.reason, '刷新总览失败'))
    return
  }
  notice.success('总览已刷新')
}

async function loadOverview(): Promise<void> {
  overviewAbortController?.abort()
  const controller = new AbortController()
  overviewAbortController = controller
  loadingOverview.value = true
  try {
    overview.value = await getDashboardOverview(controller.signal)
  } finally {
    if (overviewAbortController === controller) {
      overviewAbortController = null
      loadingOverview.value = false
    }
  }
}

async function loadTrends(): Promise<void> {
  trendsAbortController?.abort()
  const controller = new AbortController()
  trendsAbortController = controller
  loadingTrends.value = true
  try {
    const response = await getDashboardTrends(trendDays.value, controller.signal)
    trends.value = response.daily
  } finally {
    if (trendsAbortController === controller) {
      trendsAbortController = null
      loadingTrends.value = false
    }
  }
}

function selectTrendDays(days: TrendDays): void {
  if (trendDays.value === days) {
    return
  }
  trendDays.value = days
  void loadTrends().catch((error) => {
    notice.error(dashboardErrorMessage(error, '加载趋势失败'))
  })
}

function formatInteger(value: number): string {
  return new Intl.NumberFormat('zh-CN', { maximumFractionDigits: 0 }).format(value)
}

function formatNumber(value: number): string {
  return new Intl.NumberFormat('zh-CN', { maximumFractionDigits: 2 }).format(value)
}

function dashboardErrorMessage(error: unknown, fallback: string): string {
  if (error instanceof DOMException && error.name === 'AbortError') {
    return fallback
  }
  if (error instanceof ApiError) {
    if (error.code === 'permission_denied') {
      return '当前账号没有查看库存总览的权限'
    }
    return error.message
  }
  if (error instanceof ApiConfigurationError) {
    return error.message
  }
  if (error instanceof ApiNetworkError) {
    return '无法连接到 WineStock 服务'
  }
  if (error instanceof ApiResponseError) {
    return '服务响应格式无效，请检查前后端版本'
  }
  return fallback
}
</script>

<style lang="scss" src="./DashboardPage.scss"></style>
