<!--
  本文件拥有库存总览的原生 SVG 出入库曲线图，属于 frontend 展示组件层。
  它只投影已获取的每日趋势，不请求 API 或解释库存业务规则。
-->
<template>
  <div class="dashboard-trend-chart" :aria-busy="loading">
    <div class="dashboard-trend-chart__legend" aria-label="趋势图图例">
      <span
        ><i
          class="dashboard-trend-chart__legend-line dashboard-trend-chart__legend-line--inbound"
        />入库</span
      >
      <span
        ><i
          class="dashboard-trend-chart__legend-line dashboard-trend-chart__legend-line--outbound"
        />出库</span
      >
    </div>

    <div v-if="daily.length === 0" class="dashboard-trend-chart__empty">
      <strong>{{ loading ? "正在加载趋势…" : "暂无趋势数据" }}</strong>
      <span>{{ loading ? "数据返回后会自动更新。" : "服务暂未返回每日趋势。" }}</span>
    </div>

    <div
      v-else
      ref="canvasElement"
      class="dashboard-trend-chart__canvas"
      @pointerleave="handlePointerLeave"
    >
      <svg
        :viewBox="`0 0 ${chartWidth} ${chartHeight}`"
        :style="{ height: `${chartHeight}px` }"
        role="img"
        :aria-label="`近 ${days} 天入库和出库数量曲线图`"
        tabindex="0"
        @keydown="handleKeydown"
        @pointerdown="handlePointerDown"
        @pointermove="handlePointerMove"
      >
        <g class="dashboard-trend-chart__grid">
          <template v-for="tick in yTicks" :key="tick.value">
            <line :x1="paddingLeft" :x2="chartWidth - paddingRight" :y1="tick.y" :y2="tick.y" />
            <text :x="paddingLeft - 8" :y="tick.y + 4" text-anchor="end">
              {{ formatCompactNumber(tick.value) }}
            </text>
          </template>
        </g>

        <g class="dashboard-trend-chart__axis-labels">
          <text
            v-for="label in xLabels"
            :key="label.index"
            :x="label.x"
            :y="chartHeight - 7"
            :text-anchor="label.anchor"
          >
            {{ label.text }}
          </text>
        </g>

        <path
          class="dashboard-trend-chart__line dashboard-trend-chart__line--inbound"
          :d="inboundPath"
        />
        <path
          class="dashboard-trend-chart__line dashboard-trend-chart__line--outbound"
          :class="{ 'dashboard-trend-chart__line--coincident': curvesCoincide }"
          :d="outboundPath"
        />

        <g v-if="hoveredPoint" class="dashboard-trend-chart__hover" aria-hidden="true">
          <line
            :x1="hoveredPoint.x"
            :x2="hoveredPoint.x"
            :y1="paddingTop"
            :y2="chartHeight - paddingBottom"
          />
          <circle
            :cx="hoveredPoint.x"
            :cy="hoveredPoint.inboundY"
            r="5"
            class="dashboard-trend-chart__dot--inbound"
          />
          <circle
            :cx="hoveredPoint.x"
            :cy="hoveredPoint.outboundY"
            r="5"
            class="dashboard-trend-chart__dot--outbound"
          />
        </g>
      </svg>

      <div
        v-if="hoveredPoint && !compactChart"
        class="dashboard-trend-chart__tooltip"
        :style="{ left: `${tooltipLeftPercent}%` }"
      >
        <strong>{{ formatFullDate(hoveredPoint.item.date) }}</strong>
        <span>入库 {{ formatNumber(hoveredPoint.item.inbound_quantity) }}</span>
        <span>出库 {{ formatNumber(hoveredPoint.item.outbound_quantity) }}</span>
      </div>

      <div v-if="compactChart" class="dashboard-trend-chart__touch-detail" role="status">
        <template v-if="hoveredPoint">
          <strong>{{ formatFullDate(hoveredPoint.item.date) }}</strong>
          <span
            ><i
              class="dashboard-trend-chart__detail-dot dashboard-trend-chart__detail-dot--inbound"
            />入库 {{ formatNumber(hoveredPoint.item.inbound_quantity) }}</span
          >
          <span
            ><i
              class="dashboard-trend-chart__detail-dot dashboard-trend-chart__detail-dot--outbound"
            />出库 {{ formatNumber(hoveredPoint.item.outbound_quantity) }}</span
          >
        </template>
        <span v-else class="dashboard-trend-chart__touch-hint">轻触曲线查看每日数量</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import type { DailyTrend } from "../../api/dashboard";

const DEFAULT_CHART_WIDTH = 820;
const MINIMUM_CHART_WIDTH = 240;

interface ChartPoint {
  x: number;
  y: number;
}

const props = defineProps<{
  daily: readonly DailyTrend[];
  days: number;
  loading: boolean;
}>();

const canvasElement = ref<HTMLDivElement | null>(null);
const chartWidth = ref(DEFAULT_CHART_WIDTH);
const hoverIndex = ref<number | null>(null);
const touchSelectionActive = ref(false);
let resizeObserver: ResizeObserver | null = null;

const compactChart = computed(() => chartWidth.value < 720);
const narrowChart = computed(() => chartWidth.value < 480);
const chartHeight = computed(() => (narrowChart.value ? 216 : compactChart.value ? 232 : 260));
const paddingLeft = computed(() => (narrowChart.value ? 42 : compactChart.value ? 46 : 54));
const paddingRight = computed(() => (narrowChart.value ? 10 : compactChart.value ? 12 : 18));
const paddingTop = computed(() => (compactChart.value ? 14 : 18));
const paddingBottom = computed(() => (compactChart.value ? 32 : 34));
const yTickCount = computed(() => (compactChart.value ? 4 : 5));
const maximumXLabels = computed(() => (narrowChart.value ? 3 : compactChart.value ? 4 : 6));
const curvesCoincide = computed(() =>
  props.daily.every((item) => item.inbound_quantity === item.outbound_quantity),
);
const maximumValue = computed(() => {
  const maximum = Math.max(
    0,
    ...props.daily.flatMap((item) => [item.inbound_quantity, item.outbound_quantity]),
  );
  return calculateAxisMaximum(maximum);
});
const inboundPoints = computed(() => createPoints("inbound_quantity"));
const outboundPoints = computed(() => createPoints("outbound_quantity"));
const inboundPath = computed(() => createSmoothPath(inboundPoints.value));
const outboundPath = computed(() => createSmoothPath(outboundPoints.value));
const yTicks = computed(() =>
  Array.from({ length: yTickCount.value + 1 }, (_, index) => {
    const ratio = index / yTickCount.value;
    return {
      value: maximumValue.value * (1 - ratio),
      y: paddingTop.value + ratio * (chartHeight.value - paddingTop.value - paddingBottom.value),
    };
  }),
);
const xLabels = computed(() => {
  if (props.daily.length === 0) {
    return [];
  }
  const step = Math.max(1, Math.ceil((props.daily.length - 1) / (maximumXLabels.value - 1)));
  const indexes = Array.from({ length: props.daily.length }, (_, index) => index).filter(
    (index) => index % step === 0 || index === props.daily.length - 1,
  );
  return indexes.map((index, position) => ({
    index,
    x: xForIndex(index),
    text: formatShortDate(props.daily[index]!.date),
    anchor: position === 0 ? "start" : position === indexes.length - 1 ? "end" : "middle",
  }));
});
const hoveredPoint = computed(() => {
  const index = hoverIndex.value;
  if (index === null) {
    return null;
  }
  const item = props.daily[index];
  const inbound = inboundPoints.value[index];
  const outbound = outboundPoints.value[index];
  if (!item || !inbound || !outbound) {
    return null;
  }
  return {
    item,
    x: inbound.x,
    inboundY: inbound.y,
    outboundY: outbound.y,
  };
});
const tooltipLeftPercent = computed(() => {
  if (!hoveredPoint.value) {
    return 50;
  }
  return Math.min(90, Math.max(10, (hoveredPoint.value.x / chartWidth.value) * 100));
});

onMounted(() => {
  if (typeof ResizeObserver !== "undefined") {
    resizeObserver = new ResizeObserver(updateChartWidth);
  }
});

onBeforeUnmount(() => resizeObserver?.disconnect());

watch(
  canvasElement,
  (element, previousElement) => {
    if (previousElement) {
      resizeObserver?.unobserve(previousElement);
    }
    if (element) {
      updateChartWidth();
      resizeObserver?.observe(element);
    }
  },
  { flush: "post" },
);

watch(
  () => props.daily.length,
  (length) => {
    if (length === 0 || (hoverIndex.value !== null && hoverIndex.value >= length)) {
      hoverIndex.value = null;
      touchSelectionActive.value = false;
    }
  },
);

watch(
  () => props.days,
  () => {
    hoverIndex.value = null;
    touchSelectionActive.value = false;
  },
);

function createPoints(field: "inbound_quantity" | "outbound_quantity"): ChartPoint[] {
  return props.daily.map((item, index) => ({
    x: xForIndex(index),
    y: yForValue(item[field]),
  }));
}

function xForIndex(index: number): number {
  const plotWidth = chartWidth.value - paddingLeft.value - paddingRight.value;
  return props.daily.length <= 1
    ? paddingLeft.value + plotWidth / 2
    : paddingLeft.value + (index / (props.daily.length - 1)) * plotWidth;
}

function yForValue(value: number): number {
  const plotHeight = chartHeight.value - paddingTop.value - paddingBottom.value;
  return paddingTop.value + (1 - value / maximumValue.value) * plotHeight;
}

/** 为纵轴保留约一成顶部空间，并把刻度步长归一到易读数值。 */
function calculateAxisMaximum(maximum: number): number {
  if (maximum <= 0) {
    return 1;
  }

  const roughStep = (maximum * 1.1) / yTickCount.value;
  const magnitude = 10 ** Math.floor(Math.log10(roughStep));
  const normalizedStep = roughStep / magnitude;
  const niceStepFactors = [1, 1.2, 1.5, 2, 2.5, 3, 4, 5, 6, 8, 10];
  const niceStep = (niceStepFactors.find((factor) => factor >= normalizedStep) ?? 10) * magnitude;
  return niceStep * yTickCount.value;
}

function createSmoothPath(points: readonly ChartPoint[]): string {
  if (points.length === 0) {
    return "";
  }
  if (points.length === 1) {
    return `M ${points[0]!.x} ${points[0]!.y}`;
  }

  let path = `M ${points[0]!.x} ${points[0]!.y}`;
  for (let index = 1; index < points.length; index += 1) {
    const previous = points[index - 1]!;
    const current = points[index]!;
    const middleX = (previous.x + current.x) / 2;
    path += ` C ${middleX} ${previous.y}, ${middleX} ${current.y}, ${current.x} ${current.y}`;
  }
  return path;
}

function updateChartWidth(): void {
  const width = canvasElement.value?.getBoundingClientRect().width;
  if (width && Number.isFinite(width)) {
    chartWidth.value = Math.max(MINIMUM_CHART_WIDTH, Math.round(width));
  }
}

function selectPointFromPointer(event: PointerEvent): void {
  if (props.daily.length === 0) {
    return;
  }
  const svg = event.currentTarget as SVGSVGElement;
  const rect = svg.getBoundingClientRect();
  const chartX = ((event.clientX - rect.left) / rect.width) * chartWidth.value;
  const ratio = Math.min(
    1,
    Math.max(
      0,
      (chartX - paddingLeft.value) / (chartWidth.value - paddingLeft.value - paddingRight.value),
    ),
  );
  hoverIndex.value = Math.round(ratio * (props.daily.length - 1));
}

function handlePointerDown(event: PointerEvent): void {
  selectPointFromPointer(event);
  touchSelectionActive.value = event.pointerType !== "mouse";
}

function handlePointerMove(event: PointerEvent): void {
  if (event.pointerType === "mouse" || event.buttons === 1) {
    if (event.pointerType === "mouse") {
      touchSelectionActive.value = false;
    }
    selectPointFromPointer(event);
  }
}

function handlePointerLeave(event: PointerEvent): void {
  if (event.pointerType === "mouse" && !touchSelectionActive.value) {
    hoverIndex.value = null;
  }
}

function handleKeydown(event: KeyboardEvent): void {
  if (props.daily.length === 0 || !["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) {
    return;
  }
  event.preventDefault();
  if (event.key === "Home") {
    hoverIndex.value = 0;
    return;
  }
  if (event.key === "End") {
    hoverIndex.value = props.daily.length - 1;
    return;
  }
  const currentIndex = hoverIndex.value ?? (event.key === "ArrowLeft" ? props.daily.length : -1);
  const offset = event.key === "ArrowLeft" ? -1 : 1;
  hoverIndex.value = Math.min(props.daily.length - 1, Math.max(0, currentIndex + offset));
}

function formatShortDate(value: string): string {
  const [, month = "", day = ""] = value.split("-");
  return `${month}/${day}`;
}

function formatFullDate(value: string): string {
  const date = new Date(`${value}T00:00:00`);
  return Number.isNaN(date.getTime())
    ? value
    : new Intl.DateTimeFormat("zh-CN", { month: "long", day: "numeric" }).format(date);
}

function formatNumber(value: number): string {
  return new Intl.NumberFormat("zh-CN", { maximumFractionDigits: 2 }).format(value);
}

function formatCompactNumber(value: number): string {
  return new Intl.NumberFormat("zh-CN", {
    notation: value >= 10_000 ? "compact" : "standard",
    maximumFractionDigits: 2,
  }).format(value);
}
</script>

<style lang="scss" src="./DashboardTrendChart.scss"></style>
