<!-- 本组件拥有替代关系网络的 SVG 渲染、节点选择、拖动、缩放和平移；它不请求业务数据。 -->
<template>
  <div
    ref="host"
    class="substitute-network-canvas"
    :class="{ 'substitute-network-canvas--settling': settling }"
  >
    <p :id="instructionsId" class="visually-hidden">
      使用方向键在节点间移动，按 Enter 选择节点，按 Escape
      清除选择。可拖动画布或节点，滚轮可以缩放。
    </p>
    <svg
      ref="svgElement"
      role="application"
      aria-label="替代关系星链网络"
      :aria-describedby="instructionsId"
      :viewBox="`0 0 ${canvasSize.width} ${canvasSize.height}`"
      @wheel.prevent="handleWheel"
      @pointerdown="handleCanvasPointerDown"
      @pointermove="handlePointerMove"
      @pointerup="handlePointerUp"
      @pointercancel="handlePointerUp"
    >
      <defs>
        <marker
          :id="markerId"
          markerWidth="8"
          markerHeight="8"
          refX="7"
          refY="4"
          orient="auto"
          markerUnits="strokeWidth"
        >
          <path d="M0,0 L8,4 L0,8 Z" />
        </marker>
        <marker
          :id="activeMarkerId"
          markerWidth="8"
          markerHeight="8"
          refX="7"
          refY="4"
          orient="auto"
          markerUnits="strokeWidth"
        >
          <path d="M0,0 L8,4 L0,8 Z" />
        </marker>
      </defs>

      <rect class="substitute-network-canvas__backdrop" width="100%" height="100%" />

      <g :transform="contentTransform">
        <g class="substitute-network-canvas__edges" aria-hidden="true">
          <g
            v-for="edge in edges"
            :key="edge.id"
            class="substitute-network-edge"
            :class="edgeClass(edge)"
          >
            <path
              :d="edgePath(edge)"
              :marker-end="`url(#${isEdgeActive(edge) ? activeMarkerId : markerId})`"
            />
            <g :transform="edgeLabelTransform(edge)">
              <rect x="-12" y="-9" width="24" height="18" rx="4" />
              <text text-anchor="middle" dominant-baseline="central">{{ edge.priority }}</text>
            </g>
          </g>
        </g>

        <g class="substitute-network-canvas__nodes">
          <g
            v-for="node in nodes"
            :key="node.id"
            :ref="(element) => setNodeElement(node.id, element)"
            class="substitute-network-node"
            :class="nodeClass(node)"
            :transform="nodeTransform(node.id)"
            role="button"
            :tabindex="focusedNodeId === node.id ? 0 : -1"
            :aria-label="`${node.name}，编号 ${node.sku}，${node.incomingCount} 个上游，${node.outgoingCount} 个直接替代`"
            :aria-pressed="selectedId === node.id"
            :style="nodeStyle(node.id)"
            @pointerdown.stop="handleNodePointerDown($event, node.id)"
            @keydown="handleNodeKeydown($event, node.id)"
            @focus="focusedNodeId = node.id"
          >
            <title>{{ node.name }} · 编号 {{ node.sku }}</title>
            <circle class="substitute-network-node__hit" :r="nodeHitRadius(node)" />
            <circle class="substitute-network-node__surface" :r="nodeRadius(node)" />
            <rect
              class="substitute-network-node__label"
              :x="-nodeLabelWidth / 2"
              :y="nodeRadius(node) + 7"
              :width="nodeLabelWidth"
              :height="showSkuLabels ? 38 : 25"
              rx="5"
            />
            <text
              class="substitute-network-node__name"
              text-anchor="middle"
              :y="nodeRadius(node) + 21"
            >
              {{ truncate(node.name, compactLabels ? 8 : 12) }}
            </text>
            <text
              v-if="showSkuLabels"
              class="substitute-network-node__sku"
              text-anchor="middle"
              :y="nodeRadius(node) + 34"
            >
              编号 · {{ truncate(node.sku, 13) }}
            </text>
          </g>
        </g>
      </g>
    </svg>

    <span v-if="showSettling" class="substitute-network-canvas__status" role="status"
      >正在整理关系网络…</span
    >
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, useId, watch } from "vue";
import { useSubstituteNetworkLayout } from "../../composables/useSubstituteNetworkLayout";
import { useStablePendingIndicator } from "../../composables/useStablePendingIndicator";
import type {
  SubstituteNetworkEdge,
  SubstituteNetworkNode,
} from "../../pages/substitutes/networkModel";
import "./SubstituteNetworkCanvas.scss";

export interface SubstituteNetworkViewState {
  offsetX: number;
  offsetY: number;
  scale: number;
}

const props = withDefaults(
  defineProps<{
    nodes: SubstituteNetworkNode[];
    edges: SubstituteNetworkEdge[];
    selectedId: number | null;
    matchedIds?: number[];
    initialViewState?: SubstituteNetworkViewState | null;
  }>(),
  {
    matchedIds: () => [],
    initialViewState: null,
  },
);

const emit = defineEmits<{
  select: [nodeId: number | null];
  "view-change": [state: SubstituteNetworkViewState];
}>();

const host = ref<HTMLElement | null>(null);
const svgElement = ref<SVGSVGElement | null>(null);
const canvasSize = ref({ width: 800, height: 560 });
const viewState = ref<SubstituteNetworkViewState>({ offsetX: 400, offsetY: 280, scale: 1 });
const focusedNodeId = ref<number | null>(null);
const nodeElements = new Map<number, Element>();
const activePointers = new Map<number, { x: number; y: number }>();
const {
  positions,
  settling,
  mergeGraph,
  reset: resetLayoutPositions,
  moveNode,
  releaseNode,
  stop,
} = useSubstituteNetworkLayout();
const showSettling = useStablePendingIndicator(settling, {
  showDelayMs: 200,
  minimumVisibleMs: 350,
});
const componentId = useId().replace(/[^a-zA-Z0-9_-]/g, "");
const instructionsId = `${componentId}-instructions`;
const markerId = `${componentId}-arrow`;
const activeMarkerId = `${componentId}-arrow-active`;
let resizeObserver: ResizeObserver | null = null;
let dragNodeId: number | null = null;
let dragMoved = false;
let panStart: { pointerId: number; x: number; y: number; offsetX: number; offsetY: number } | null =
  null;
let pinchStart: {
  distance: number;
  scale: number;
  centerX: number;
  centerY: number;
  offsetX: number;
  offsetY: number;
} | null = null;
let hasAppliedInitialView = false;
let hasAutoFitAfterLayout = false;
let userAdjustedView = false;

const contentTransform = computed(
  () =>
    `translate(${viewState.value.offsetX} ${viewState.value.offsetY}) scale(${viewState.value.scale})`,
);
const matchedIdSet = computed(() => new Set(props.matchedIds));
const directNodeIds = computed(() => {
  if (props.selectedId === null) return new Set<number>();
  const result = new Set<number>([props.selectedId]);
  for (const edge of props.edges) {
    if (edge.sourceId === props.selectedId) result.add(edge.targetId);
    if (edge.targetId === props.selectedId) result.add(edge.sourceId);
  }
  return result;
});
const compactLabels = computed(() => canvasSize.value.width < 620);
const showSkuLabels = computed(
  () => !compactLabels.value && viewState.value.scale >= 0.7 && props.nodes.length <= 180,
);
const nodeLabelWidth = computed(() => (compactLabels.value ? 90 : 120));

watch(
  () =>
    `${props.nodes.map((node) => node.id).join(",")}|${props.edges.map((edge) => edge.id).join(",")}`,
  async () => {
    mergeGraph(props.nodes, props.edges);
    if (
      focusedNodeId.value === null ||
      !props.nodes.some((node) => node.id === focusedNodeId.value)
    )
      focusedNodeId.value = props.nodes[0]?.id ?? null;
    await nextTick();
    if (!hasAppliedInitialView) applyInitialView();
  },
  { immediate: true },
);

watch(settling, (current, previous) => {
  if (previous && !current && !hasAutoFitAfterLayout && !userAdjustedView) {
    hasAutoFitAfterLayout = true;
    fit();
  }
});

onMounted(() => {
  resizeObserver = new ResizeObserver(([entry]) => {
    const width = Math.max(1, entry.contentRect.width);
    const height = Math.max(1, entry.contentRect.height);
    const changed =
      Math.abs(width - canvasSize.value.width) > 8 ||
      Math.abs(height - canvasSize.value.height) > 8;
    canvasSize.value = { width, height };
    if (!hasAppliedInitialView) applyInitialView();
    else if (changed && !userAdjustedView) window.requestAnimationFrame(() => fit());
  });
  if (host.value) resizeObserver.observe(host.value);
});

onBeforeUnmount(() => {
  resizeObserver?.disconnect();
  stop();
});

function applyInitialView(): void {
  if (!props.nodes.length || canvasSize.value.width <= 1) return;
  hasAppliedInitialView = true;
  if (props.initialViewState) setViewState(props.initialViewState);
  else window.requestAnimationFrame(() => fit());
}

function fit(): void {
  const visiblePositions = props.nodes.map((node) => positions.value[node.id]).filter(Boolean);
  if (!visiblePositions.length) return;
  const minimumX = Math.min(...visiblePositions.map((position) => position.x)) - 72;
  const maximumX = Math.max(...visiblePositions.map((position) => position.x)) + 72;
  const minimumY = Math.min(...visiblePositions.map((position) => position.y)) - 72;
  const maximumY = Math.max(...visiblePositions.map((position) => position.y)) + 88;
  const contentWidth = Math.max(1, maximumX - minimumX);
  const contentHeight = Math.max(1, maximumY - minimumY);
  const scale = clamp(
    Math.min(canvasSize.value.width / contentWidth, canvasSize.value.height / contentHeight) * 0.92,
    0.45,
    1.45,
  );
  setViewState({
    scale,
    offsetX: canvasSize.value.width / 2 - ((minimumX + maximumX) / 2) * scale,
    offsetY: canvasSize.value.height / 2 - ((minimumY + maximumY) / 2) * scale,
  });
}

function focusNode(nodeId: number): void {
  const position = positions.value[nodeId];
  if (!position) return;
  const scale = Math.max(0.85, viewState.value.scale);
  setViewState({
    scale,
    offsetX: canvasSize.value.width / 2 - position.x * scale,
    offsetY: canvasSize.value.height / 2 - position.y * scale,
  });
  focusedNodeId.value = nodeId;
  nextTick(() => (nodeElements.get(nodeId) as SVGGraphicsElement | undefined)?.focus());
}

function zoomIn(): void {
  zoomAt(canvasSize.value.width / 2, canvasSize.value.height / 2, viewState.value.scale * 1.2);
}

function zoomOut(): void {
  zoomAt(canvasSize.value.width / 2, canvasSize.value.height / 2, viewState.value.scale / 1.2);
}

function resetLayout(): void {
  resetLayoutPositions();
  window.setTimeout(fit, 180);
}

function getViewState(): SubstituteNetworkViewState {
  return { ...viewState.value };
}

function setViewState(state: SubstituteNetworkViewState): void {
  viewState.value = { ...state, scale: clamp(state.scale, 0.45, 2.2) };
  emit("view-change", getViewState());
}

function handleWheel(event: WheelEvent): void {
  userAdjustedView = true;
  const point = localPoint(event.clientX, event.clientY);
  zoomAt(point.x, point.y, viewState.value.scale * Math.exp(-event.deltaY * 0.0015));
}

function zoomAt(screenX: number, screenY: number, requestedScale: number): void {
  const oldScale = viewState.value.scale;
  const scale = clamp(requestedScale, 0.45, 2.2);
  const graphX = (screenX - viewState.value.offsetX) / oldScale;
  const graphY = (screenY - viewState.value.offsetY) / oldScale;
  setViewState({ scale, offsetX: screenX - graphX * scale, offsetY: screenY - graphY * scale });
}

function handleCanvasPointerDown(event: PointerEvent): void {
  if (event.button !== 0) return;
  activePointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
  svgElement.value?.setPointerCapture(event.pointerId);
  if (activePointers.size === 2) {
    beginPinch();
    panStart = null;
    return;
  }
  panStart = {
    pointerId: event.pointerId,
    x: event.clientX,
    y: event.clientY,
    offsetX: viewState.value.offsetX,
    offsetY: viewState.value.offsetY,
  };
  dragMoved = false;
}

function handleNodePointerDown(event: PointerEvent, nodeId: number): void {
  if (event.button !== 0) return;
  activePointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
  svgElement.value?.setPointerCapture(event.pointerId);
  dragNodeId = nodeId;
  dragMoved = false;
  focusedNodeId.value = nodeId;
}

function handlePointerMove(event: PointerEvent): void {
  if (!activePointers.has(event.pointerId)) return;
  const previous = activePointers.get(event.pointerId)!;
  activePointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
  if (activePointers.size >= 2) {
    updatePinch();
    return;
  }
  if (dragNodeId !== null) {
    if (Math.hypot(event.clientX - previous.x, event.clientY - previous.y) > 1) {
      dragMoved = true;
      userAdjustedView = true;
    }
    const point = graphPoint(event.clientX, event.clientY);
    moveNode(dragNodeId, point.x, point.y);
    return;
  }
  if (panStart?.pointerId === event.pointerId) {
    const deltaX = event.clientX - panStart.x;
    const deltaY = event.clientY - panStart.y;
    if (Math.hypot(deltaX, deltaY) > 3) {
      dragMoved = true;
      userAdjustedView = true;
    }
    setViewState({
      ...viewState.value,
      offsetX: panStart.offsetX + deltaX,
      offsetY: panStart.offsetY + deltaY,
    });
  }
}

function handlePointerUp(event: PointerEvent): void {
  if (!activePointers.has(event.pointerId)) return;
  activePointers.delete(event.pointerId);
  if (dragNodeId !== null) {
    const nodeId = dragNodeId;
    releaseNode(nodeId);
    dragNodeId = null;
    if (!dragMoved) emit("select", nodeId);
  } else if (panStart?.pointerId === event.pointerId && !dragMoved) {
    emit("select", null);
  }
  panStart = null;
  pinchStart = null;
}

function beginPinch(): void {
  const [first, second] = Array.from(activePointers.values());
  const firstPoint = localPoint(first.x, first.y);
  const secondPoint = localPoint(second.x, second.y);
  pinchStart = {
    distance: Math.max(1, Math.hypot(secondPoint.x - firstPoint.x, secondPoint.y - firstPoint.y)),
    scale: viewState.value.scale,
    centerX: (firstPoint.x + secondPoint.x) / 2,
    centerY: (firstPoint.y + secondPoint.y) / 2,
    offsetX: viewState.value.offsetX,
    offsetY: viewState.value.offsetY,
  };
}

function updatePinch(): void {
  if (!pinchStart) beginPinch();
  if (!pinchStart) return;
  const [first, second] = Array.from(activePointers.values());
  const firstPoint = localPoint(first.x, first.y);
  const secondPoint = localPoint(second.x, second.y);
  const centerX = (firstPoint.x + secondPoint.x) / 2;
  const centerY = (firstPoint.y + secondPoint.y) / 2;
  const distance = Math.max(
    1,
    Math.hypot(secondPoint.x - firstPoint.x, secondPoint.y - firstPoint.y),
  );
  const scale = clamp(pinchStart.scale * (distance / pinchStart.distance), 0.45, 2.2);
  const graphX = (pinchStart.centerX - pinchStart.offsetX) / pinchStart.scale;
  const graphY = (pinchStart.centerY - pinchStart.offsetY) / pinchStart.scale;
  setViewState({ scale, offsetX: centerX - graphX * scale, offsetY: centerY - graphY * scale });
}

function handleNodeKeydown(event: KeyboardEvent, nodeId: number): void {
  if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    emit("select", nodeId);
    return;
  }
  if (event.key === "Escape") {
    event.preventDefault();
    event.stopPropagation();
    emit("select", null);
    return;
  }
  const direction = directionForKey(event.key);
  if (!direction) return;
  event.preventDefault();
  const current = positions.value[nodeId];
  if (!current) return;
  const candidate = props.nodes
    .filter((node) => node.id !== nodeId && positions.value[node.id])
    .map((node) => {
      const position = positions.value[node.id];
      const deltaX = position.x - current.x;
      const deltaY = position.y - current.y;
      const forward = deltaX * direction.x + deltaY * direction.y;
      const sideways = Math.abs(deltaX * direction.y - deltaY * direction.x);
      return { node, forward, score: Math.hypot(deltaX, deltaY) + sideways * 1.4 };
    })
    .filter((candidate) => candidate.forward > 0)
    .sort((left, right) => left.score - right.score)[0];
  if (candidate) focusNode(candidate.node.id);
}

function nodeClass(node: SubstituteNetworkNode): Record<string, boolean> {
  const selected = props.selectedId === node.id;
  const related = props.selectedId !== null && directNodeIds.value.has(node.id);
  return {
    "substitute-network-node--selected": selected,
    "substitute-network-node--related": related && !selected,
    "substitute-network-node--muted": props.selectedId !== null && !related,
    "substitute-network-node--matched": matchedIdSet.value.has(node.id),
  };
}

function edgeClass(edge: SubstituteNetworkEdge): Record<string, boolean> {
  return {
    "substitute-network-edge--active": isEdgeActive(edge),
    "substitute-network-edge--muted": props.selectedId !== null && !isEdgeActive(edge),
  };
}

function isEdgeActive(edge: SubstituteNetworkEdge): boolean {
  return (
    props.selectedId !== null &&
    (edge.sourceId === props.selectedId || edge.targetId === props.selectedId)
  );
}

function nodeTransform(nodeId: number): string {
  const position = positions.value[nodeId] ?? { x: 0, y: 0 };
  return `translate(${position.x} ${position.y})`;
}

function nodeRadius(node: SubstituteNetworkNode): number {
  if (node.degree >= 6) return 25;
  if (node.degree >= 3) return 22;
  return 19;
}

function nodeHitRadius(node: SubstituteNetworkNode): number {
  return Math.max(nodeRadius(node) + 7, 22 / viewState.value.scale);
}

function nodeStyle(nodeId: number): Record<string, string> {
  const palette = [
    ["#f3e8ea", "#8b4753"],
    ["#e8f0f5", "#4d7087"],
    ["#e8f3ee", "#4f7663"],
    ["#f4eee4", "#8a6a3f"],
    ["#eeeaf5", "#6f5a8d"],
    ["#f2e9e3", "#8a5d46"],
    ["#e7f1f1", "#467777"],
  ];
  const [soft, strong] = palette[Math.abs(nodeId) % palette.length];
  return { "--node-soft": soft, "--node-strong": strong };
}

function edgePath(edge: SubstituteNetworkEdge): string {
  const source = positions.value[edge.sourceId];
  const target = positions.value[edge.targetId];
  if (!source || !target) return "";
  const sourceNode = props.nodes.find((node) => node.id === edge.sourceId);
  const targetNode = props.nodes.find((node) => node.id === edge.targetId);
  const sourceRadius = sourceNode ? nodeRadius(sourceNode) + 3 : 23;
  const targetRadius = targetNode ? nodeRadius(targetNode) + 8 : 27;
  const deltaX = target.x - source.x;
  const deltaY = target.y - source.y;
  const distance = Math.max(1, Math.hypot(deltaX, deltaY));
  const startX = source.x + (deltaX / distance) * sourceRadius;
  const startY = source.y + (deltaY / distance) * sourceRadius;
  const endX = target.x - (deltaX / distance) * targetRadius;
  const endY = target.y - (deltaY / distance) * targetRadius;
  const curve = edgeCurve(edge, deltaX, deltaY, distance);
  const controlX = (startX + endX) / 2 + curve.x;
  const controlY = (startY + endY) / 2 + curve.y;
  return `M ${startX} ${startY} Q ${controlX} ${controlY} ${endX} ${endY}`;
}

function edgeLabelTransform(edge: SubstituteNetworkEdge): string {
  const source = positions.value[edge.sourceId];
  const target = positions.value[edge.targetId];
  if (!source || !target) return "translate(0 0)";
  const deltaX = target.x - source.x;
  const deltaY = target.y - source.y;
  const distance = Math.max(1, Math.hypot(deltaX, deltaY));
  const curve = edgeCurve(edge, deltaX, deltaY, distance);
  return `translate(${(source.x + target.x) / 2 + curve.x * 0.5} ${(source.y + target.y) / 2 + curve.y * 0.5})`;
}

function edgeCurve(
  edge: SubstituteNetworkEdge,
  deltaX: number,
  deltaY: number,
  distance: number,
): { x: number; y: number } {
  const reverseExists = props.edges.some(
    (candidate) => candidate.sourceId === edge.targetId && candidate.targetId === edge.sourceId,
  );
  if (!reverseExists) return { x: (-deltaY / distance) * 7, y: (deltaX / distance) * 7 };
  const direction = edge.sourceId < edge.targetId ? 1 : -1;
  return { x: (-deltaY / distance) * 30 * direction, y: (deltaX / distance) * 30 * direction };
}

function setNodeElement(nodeId: number, element: Element | { $el?: unknown } | null): void {
  const resolved =
    element instanceof Element ? element : element?.$el instanceof Element ? element.$el : null;
  if (resolved) nodeElements.set(nodeId, resolved);
  else nodeElements.delete(nodeId);
}

function graphPoint(clientX: number, clientY: number): { x: number; y: number } {
  const point = localPoint(clientX, clientY);
  return {
    x: (point.x - viewState.value.offsetX) / viewState.value.scale,
    y: (point.y - viewState.value.offsetY) / viewState.value.scale,
  };
}

function localPoint(clientX: number, clientY: number): { x: number; y: number } {
  const bounds = svgElement.value?.getBoundingClientRect();
  return { x: clientX - (bounds?.left ?? 0), y: clientY - (bounds?.top ?? 0) };
}

function directionForKey(key: string): { x: number; y: number } | null {
  if (key === "ArrowLeft") return { x: -1, y: 0 };
  if (key === "ArrowRight") return { x: 1, y: 0 };
  if (key === "ArrowUp") return { x: 0, y: -1 };
  if (key === "ArrowDown") return { x: 0, y: 1 };
  return null;
}

function truncate(value: string, maximum: number): string {
  return value.length > maximum ? `${value.slice(0, maximum - 1)}…` : value;
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.min(maximum, Math.max(minimum, value));
}

defineExpose({ fit, focusNode, zoomIn, zoomOut, resetLayout, getViewState });
</script>
