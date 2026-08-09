<!-- 本组件拥有替代关系网络的搜索、观察范围、画布控制、详情和编辑编排；它复用页面已加载的数据。 -->
<template>
  <ModalDialog
    :open="open"
    :title="$t('substitutes.networkTitle')"
    :description="$t('substitutes.networkDescription')"
    network-workspace
    :restore-focus="!transitioning"
    :auto-focus="false"
    @close="requestClose"
  >
    <div class="substitute-network-workspace">
      <div class="substitute-network-toolbar">
        <div class="substitute-network-toolbar__search-wrap">
          <SearchField
            v-model="searchInput"
            class="substitute-network-toolbar__search"
            :label="$t('substitutes.searchNodeLabel')"
            name="substitute_network_search"
            :placeholder="$t('substitutes.searchNodePlaceholder')"
            hide-label
            @search="applySearch"
          />
          <div
            v-if="showSearchResults"
            v-overlay-scrollbar
            class="substitute-network-search-results"
            role="listbox"
            :aria-label="$t('substitutes.searchResultsLabel')"
          >
            <button
              v-for="node in searchResults"
              :key="node.id"
              type="button"
              role="option"
              @click="selectSearchResult(node.id)"
            >
              <strong>{{ node.name }}</strong>
              <span>{{ $t('substitutes.nodeConnections', { sku: node.sku, n: node.degree }) }}</span>
            </button>
            <p v-if="!searchResults.length">{{ $t('substitutes.noSearchResults') }}</p>
          </div>
        </div>

        <div class="substitute-network-toolbar__range" :aria-label="$t('substitutes.rangeLabel')">
          <button
            type="button"
            :class="{ active: rangeMode === 'all' }"
            :aria-pressed="rangeMode === 'all'"
            @click="rangeMode = 'all'"
          >
            {{ $t('substitutes.rangeAll') }}
          </button>
          <button
            type="button"
            :class="{ active: rangeMode === 'direct' }"
            :aria-pressed="rangeMode === 'direct'"
            :disabled="selectedId === null"
            @click="rangeMode = 'direct'"
          >
            {{ $t('substitutes.rangeDirect') }}
          </button>
        </div>

        <span class="substitute-network-toolbar__count"
          >{{ $t('substitutes.networkStats', { nodes: displayedNodes.length, edges: displayedEdges.length }) }}</span
        >
        <span v-if="refreshing" class="substitute-network-toolbar__refreshing" role="status"
          >{{ $t('substitutes.networkRefreshing') }}</span
        >

        <div class="substitute-network-toolbar__controls">
          <button
            class="icon-button"
            type="button"
            :title="$t('substitutes.fitCanvas')"
            :aria-label="$t('substitutes.fitCanvas')"
            :disabled="!displayedNodes.length"
            @click="canvas?.fit()"
          >
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M9 4H4v5M15 4h5v5M9 20H4v-5M15 20h5v-5" />
            </svg>
          </button>
          <button
            class="icon-button"
            type="button"
            :title="$t('substitutes.zoomOut')"
            :aria-label="$t('substitutes.zoomOutAria')"
            :disabled="!displayedNodes.length"
            @click="canvas?.zoomOut()"
          >
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <circle cx="10.5" cy="10.5" r="6.5" />
              <path d="M7.5 10.5h6M15.5 15.5l5 5" />
            </svg>
          </button>
          <button
            class="icon-button"
            type="button"
            :title="$t('substitutes.zoomIn')"
            :aria-label="$t('substitutes.zoomInAria')"
            :disabled="!displayedNodes.length"
            @click="canvas?.zoomIn()"
          >
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <circle cx="10.5" cy="10.5" r="6.5" />
              <path d="M10.5 7.5v6M7.5 10.5h6M15.5 15.5l5 5" />
            </svg>
          </button>
          <button
            class="icon-button"
            type="button"
            :title="$t('substitutes.resetLayoutTitle')"
            :aria-label="$t('substitutes.resetLayoutAria')"
            :disabled="!displayedNodes.length"
            @click="canvas?.resetLayout()"
          >
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M20 7v5h-5" />
              <path d="M18.2 16a7 7 0 1 1 .8-7l1 3" />
            </svg>
          </button>
          <details class="substitute-network-legend">
            <summary class="icon-button" :title="$t('substitutes.viewLegend')" :aria-label="$t('substitutes.viewLegendAria')">
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <circle cx="12" cy="12" r="8" />
                <path d="M12 11v5M12 8h.01" />
              </svg>
            </summary>
            <div>
              <span><i class="substitute-network-legend__node"></i>{{ $t('substitutes.legendNode') }}</span>
              <span><i class="substitute-network-legend__selected"></i>{{ $t('substitutes.currentNode') }}</span>
              <span><i class="substitute-network-legend__edge"></i>{{ $t('substitutes.directionLabel') }}</span>
              <small>{{ $t('substitutes.legendHint') }}</small>
            </div>
          </details>
        </div>
      </div>

      <div class="substitute-network-stage">
        <div v-if="!graph.nodes.length" class="substitute-network-empty">
          <strong>{{ $t('substitutes.emptyNetworkTitle') }}</strong>
          <span>{{ $t('substitutes.emptyNetworkHint') }}</span>
        </div>
        <div
          v-else-if="graph.nodes.length > 300 && selectedId === null"
          class="substitute-network-empty"
        >
          <strong>{{ $t('substitutes.largeNetworkTitle') }}</strong>
          <span>{{ $t('substitutes.largeNetworkHint', { n: graph.nodes.length }) }}</span>
        </div>
        <SubstituteNetworkCanvas
          v-else
          ref="canvas"
          class="substitute-network-workspace__canvas"
          :nodes="displayedNodes"
          :edges="displayedEdges"
          :selected-id="selectedId"
          :matched-ids="searchResults.map((node) => node.id)"
          :initial-view-state="viewState"
          @select="selectNode"
          @view-change="viewState = $event"
        />

        <SubstituteNetworkNodeDetail
          v-if="selectedId !== null"
          :graph="graph"
          :selected-id="selectedId"
          :can-manage="canManage"
          @edit="editNode"
        />
      </div>

      <div v-if="scaleNotice" class="substitute-network-scale-notice" role="status">
        <span>{{ scaleNotice }}</span>
        <button
          v-if="graph.nodes.length > 180 && graph.nodes.length <= 300"
          class="text-button"
          type="button"
          @click="showAllLargeGraph = !showAllLargeGraph"
        >
          {{ showAllLargeGraph ? $t('substitutes.showCore') : $t('substitutes.showAll') }}
        </button>
      </div>
    </div>
  </ModalDialog>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { SubstituteRelationResponse } from "../../api/substitutes";
import type { SubstituteEditorTarget } from "../../pages/substitutes/model";
import {
  buildSubstituteNetwork,
  getDirectRelationNodeIds,
  getRenderableNodeIds,
  searchSubstituteNetworkNodes,
  type SubstituteNetworkNode,
} from "../../pages/substitutes/networkModel";
import ModalDialog from "../ModalDialog.vue";
import SearchField from "../SearchField.vue";
import SubstituteNetworkCanvas, {
  type SubstituteNetworkViewState,
} from "./SubstituteNetworkCanvas.vue";
import SubstituteNetworkNodeDetail from "./SubstituteNetworkNodeDetail.vue";
import "./SubstituteNetworkDialog.scss";

interface CanvasApi {
  fit: () => void;
  focusNode: (nodeId: number) => void;
  zoomIn: () => void;
  zoomOut: () => void;
  resetLayout: () => void;
  getViewState: () => SubstituteNetworkViewState;
}

const props = defineProps<{
  open: boolean;
  relations: SubstituteRelationResponse[];
  canManage: boolean;
  refreshing?: boolean;
}>();

const emit = defineEmits<{
  close: [];
  edit: [target: SubstituteEditorTarget];
}>();

const { t } = useI18n();
const canvas = ref<CanvasApi | null>(null);
const searchInput = ref("");
const activeSearch = ref("");
const selectedId = ref<number | null>(null);
const rangeMode = ref<"all" | "direct">("all");
const viewState = ref<SubstituteNetworkViewState | null>(null);
const transitioning = ref(false);
const showAllLargeGraph = ref(false);
const graph = computed(() => buildSubstituteNetwork(props.relations));
const searchResults = computed(() => searchSubstituteNetworkNodes(graph.value, activeSearch.value));
const showSearchResults = computed(
  () =>
    Boolean(activeSearch.value) &&
    (searchResults.value.length !== 1 || searchResults.value[0]?.id !== selectedId.value),
);
const renderableIds = computed(() => {
  if (rangeMode.value === "direct" && selectedId.value !== null)
    return getDirectRelationNodeIds(graph.value, selectedId.value);
  if (showAllLargeGraph.value && graph.value.nodes.length <= 300)
    return new Set(graph.value.nodes.map((node) => node.id));
  return getRenderableNodeIds(graph.value, selectedId.value);
});
const displayedNodes = computed(() =>
  graph.value.nodes.filter((node) => renderableIds.value.has(node.id)),
);
const displayedEdges = computed(() =>
  graph.value.edges.filter(
    (edge) => renderableIds.value.has(edge.sourceId) && renderableIds.value.has(edge.targetId),
  ),
);
const scaleNotice = computed(() => {
  if (graph.value.nodes.length > 300 && selectedId.value !== null)
    return t("substitutes.scaleNoticeLocal", {
      n: displayedNodes.value.length,
      total: graph.value.nodes.length,
    });
  if (graph.value.nodes.length > 180)
    return showAllLargeGraph.value
      ? t("substitutes.scaleNoticeAll", { n: displayedNodes.value.length })
      : t("substitutes.scaleNoticePartial", { n: displayedNodes.value.length });
  return "";
});

watch(graph, (currentGraph) => {
  if (selectedId.value !== null && !currentGraph.nodeById.has(selectedId.value)) {
    selectedId.value = null;
    rangeMode.value = "all";
  }
  if (currentGraph.nodes.length <= 180 || currentGraph.nodes.length > 300)
    showAllLargeGraph.value = false;
});

watch(
  () => props.open,
  (open) => {
    if (!open) {
      return;
    }
    transitioning.value = false;
    if (selectedId.value !== null) nextTick(() => canvas.value?.focusNode(selectedId.value!));
  },
);

function applySearch(value: string): void {
  activeSearch.value = value.trim();
  if (searchResults.value.length === 1) selectSearchResult(searchResults.value[0].id);
}

function selectSearchResult(nodeId: number): void {
  selectedId.value = nodeId;
  nextTick(() => canvas.value?.focusNode(nodeId));
}

function selectNode(nodeId: number | null): void {
  selectedId.value = nodeId;
  if (nodeId === null && rangeMode.value === "direct") rangeMode.value = "all";
}

function requestClose(): void {
  transitioning.value = false;
  viewState.value = canvas.value?.getViewState() ?? viewState.value;
  emit("close");
}

function editNode(node: SubstituteNetworkNode): void {
  transitioning.value = true;
  viewState.value = canvas.value?.getViewState() ?? viewState.value;
  emit("edit", { id: node.id, name: node.name, sku: node.sku });
}
</script>
