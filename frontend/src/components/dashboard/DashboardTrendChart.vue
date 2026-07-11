<!--
  本文件拥有库存总览的原生 SVG 出入库曲线图，属于 frontend 展示组件层。
  它只投影已获取的每日趋势，不请求 API 或解释库存业务规则。
-->
<template>
  <div class="dashboard-trend-chart" :aria-busy="loading">
    <div class="dashboard-trend-chart__legend" aria-label="趋势图图例">
      <span><i class="dashboard-trend-chart__legend-line dashboard-trend-chart__legend-line--inbound" />入库</span>
      <span><i class="dashboard-trend-chart__legend-line dashboard-trend-chart__legend-line--outbound" />出库</span>
    </div>

    <div v-if="daily.length === 0" class="dashboard-trend-chart__empty">
      <strong>{{ loading ? '正在加载趋势…' : '暂无趋势数据' }}</strong>
      <span>{{ loading ? '数据返回后会自动更新。' : '服务暂未返回每日趋势。' }}</span>
    </div>

    <div v-else class="dashboard-trend-chart__canvas" @pointerleave="hoverIndex = null">
      <svg
        :viewBox="`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`"
        role="img"
        :aria-label="`近 ${days} 天入库和出库数量曲线图`"
        @pointermove="handlePointerMove"
      >
        <g class="dashboard-trend-chart__grid">
          <template v-for="tick in yTicks" :key="tick.value">
            <line :x1="PADDING_LEFT" :x2="CHART_WIDTH - PADDING_RIGHT" :y1="tick.y" :y2="tick.y" />
            <text :x="PADDING_LEFT - 10" :y="tick.y + 4" text-anchor="end">
              {{ formatCompactNumber(tick.value) }}
            </text>
          </template>
        </g>

        <g class="dashboard-trend-chart__axis-labels">
          <text
            v-for="label in xLabels"
            :key="label.index"
            :x="label.x"
            :y="CHART_HEIGHT - 7"
            :text-anchor="label.anchor"
          >
            {{ label.text }}
          </text>
        </g>

        <path class="dashboard-trend-chart__line dashboard-trend-chart__line--inbound" :d="inboundPath" />
        <path
          class="dashboard-trend-chart__line dashboard-trend-chart__line--outbound"
          :class="{ 'dashboard-trend-chart__line--coincident': curvesCoincide }"
          :d="outboundPath"
        />

        <g v-if="hoveredPoint" class="dashboard-trend-chart__hover" aria-hidden="true">
          <line
            :x1="hoveredPoint.x"
            :x2="hoveredPoint.x"
            :y1="PADDING_TOP"
            :y2="CHART_HEIGHT - PADDING_BOTTOM"
          />
          <circle :cx="hoveredPoint.x" :cy="hoveredPoint.inboundY" r="5" class="dashboard-trend-chart__dot--inbound" />
          <circle :cx="hoveredPoint.x" :cy="hoveredPoint.outboundY" r="5" class="dashboard-trend-chart__dot--outbound" />
        </g>
      </svg>

      <div
        v-if="hoveredPoint"
        class="dashboard-trend-chart__tooltip"
        :style="{ left: `${tooltipLeftPercent}%` }"
      >
        <strong>{{ formatFullDate(hoveredPoint.item.date) }}</strong>
        <span>入库 {{ formatNumber(hoveredPoint.item.inbound_quantity) }}</span>
        <span>出库 {{ formatNumber(hoveredPoint.item.outbound_quantity) }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import type { DailyTrend } from '../../api/dashboard'

const CHART_WIDTH = 820
const CHART_HEIGHT = 260
const PADDING_LEFT = 54
const PADDING_RIGHT = 18
const PADDING_TOP = 18
const PADDING_BOTTOM = 34
const Y_TICK_COUNT = 4

interface ChartPoint {
  x: number
  y: number
}

const props = defineProps<{
  daily: readonly DailyTrend[]
  days: number
  loading: boolean
}>()

const hoverIndex = ref<number | null>(null)
const curvesCoincide = computed(() =>
  props.daily.every((item) => item.inbound_quantity === item.outbound_quantity),
)
const maximumValue = computed(() => {
  const maximum = Math.max(
    0,
    ...props.daily.flatMap((item) => [item.inbound_quantity, item.outbound_quantity]),
  )
  return maximum > 0 ? maximum : 1
})
const inboundPoints = computed(() => createPoints('inbound_quantity'))
const outboundPoints = computed(() => createPoints('outbound_quantity'))
const inboundPath = computed(() => createSmoothPath(inboundPoints.value))
const outboundPath = computed(() => createSmoothPath(outboundPoints.value))
const yTicks = computed(() =>
  Array.from({ length: Y_TICK_COUNT + 1 }, (_, index) => {
    const ratio = index / Y_TICK_COUNT
    return {
      value: maximumValue.value * (1 - ratio),
      y: PADDING_TOP + ratio * (CHART_HEIGHT - PADDING_TOP - PADDING_BOTTOM),
    }
  }),
)
const xLabels = computed(() => {
  if (props.daily.length === 0) {
    return []
  }
  const maximumLabels = 6
  const step = Math.max(1, Math.ceil((props.daily.length - 1) / (maximumLabels - 1)))
  const indexes = Array.from({ length: props.daily.length }, (_, index) => index).filter(
    (index) => index % step === 0 || index === props.daily.length - 1,
  )
  return indexes.map((index, position) => ({
    index,
    x: xForIndex(index),
    text: formatShortDate(props.daily[index]!.date),
    anchor: position === 0 ? 'start' : position === indexes.length - 1 ? 'end' : 'middle',
  }))
})
const hoveredPoint = computed(() => {
  const index = hoverIndex.value
  if (index === null) {
    return null
  }
  const item = props.daily[index]
  const inbound = inboundPoints.value[index]
  const outbound = outboundPoints.value[index]
  if (!item || !inbound || !outbound) {
    return null
  }
  return {
    item,
    x: inbound.x,
    inboundY: inbound.y,
    outboundY: outbound.y,
  }
})
const tooltipLeftPercent = computed(() => {
  if (!hoveredPoint.value) {
    return 50
  }
  return Math.min(90, Math.max(10, (hoveredPoint.value.x / CHART_WIDTH) * 100))
})

function createPoints(field: 'inbound_quantity' | 'outbound_quantity'): ChartPoint[] {
  return props.daily.map((item, index) => ({
    x: xForIndex(index),
    y: yForValue(item[field]),
  }))
}

function xForIndex(index: number): number {
  const plotWidth = CHART_WIDTH - PADDING_LEFT - PADDING_RIGHT
  return props.daily.length <= 1
    ? PADDING_LEFT + plotWidth / 2
    : PADDING_LEFT + (index / (props.daily.length - 1)) * plotWidth
}

function yForValue(value: number): number {
  const plotHeight = CHART_HEIGHT - PADDING_TOP - PADDING_BOTTOM
  return PADDING_TOP + (1 - value / maximumValue.value) * plotHeight
}

function createSmoothPath(points: readonly ChartPoint[]): string {
  if (points.length === 0) {
    return ''
  }
  if (points.length === 1) {
    return `M ${points[0]!.x} ${points[0]!.y}`
  }

  let path = `M ${points[0]!.x} ${points[0]!.y}`
  for (let index = 1; index < points.length; index += 1) {
    const previous = points[index - 1]!
    const current = points[index]!
    const middleX = (previous.x + current.x) / 2
    path += ` C ${middleX} ${previous.y}, ${middleX} ${current.y}, ${current.x} ${current.y}`
  }
  return path
}

function handlePointerMove(event: PointerEvent): void {
  if (props.daily.length === 0) {
    return
  }
  const svg = event.currentTarget as SVGSVGElement
  const rect = svg.getBoundingClientRect()
  const chartX = ((event.clientX - rect.left) / rect.width) * CHART_WIDTH
  const ratio = Math.min(
    1,
    Math.max(0, (chartX - PADDING_LEFT) / (CHART_WIDTH - PADDING_LEFT - PADDING_RIGHT)),
  )
  hoverIndex.value = Math.round(ratio * (props.daily.length - 1))
}

function formatShortDate(value: string): string {
  const [, month = '', day = ''] = value.split('-')
  return `${month}/${day}`
}

function formatFullDate(value: string): string {
  const date = new Date(`${value}T00:00:00`)
  return Number.isNaN(date.getTime())
    ? value
    : new Intl.DateTimeFormat('zh-CN', { month: 'long', day: 'numeric' }).format(date)
}

function formatNumber(value: number): string {
  return new Intl.NumberFormat('zh-CN', { maximumFractionDigits: 2 }).format(value)
}

function formatCompactNumber(value: number): string {
  return new Intl.NumberFormat('zh-CN', {
    notation: value >= 10_000 ? 'compact' : 'standard',
    maximumFractionDigits: 2,
  }).format(value)
}
</script>

<style lang="scss" src="./DashboardTrendChart.scss"></style>
