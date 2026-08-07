<!-- 本组件拥有选中网络节点的文字关系摘要和列表、编辑入口；它不管理画布或请求数据。 -->
<template>
  <section
    class="substitute-network-detail"
    :class="{ 'substitute-network-detail--expanded': expanded }"
    aria-live="polite"
  >
    <template v-if="node">
      <div class="substitute-network-detail__summary">
        <div class="substitute-network-detail__identity">
          <span>当前节点</span>
          <strong>{{ node.name }}</strong>
          <small>编号 {{ node.sku }}</small>
        </div>
        <div v-overlay-scrollbar class="substitute-network-detail__metrics">
          <span
            ><strong>{{ incoming.length }}</strong> 个上游</span
          >
          <span
            ><strong>{{ outgoing.length }}</strong> 个直接替代</span
          >
          <span
            ><strong>{{ reachableCount }}</strong> 个可到达</span
          >
        </div>
        <div class="substitute-network-detail__actions">
          <button
            class="text-button substitute-network-detail__toggle"
            type="button"
            :aria-expanded="expanded"
            @click="expanded = !expanded"
          >
            {{ expanded ? "收起详情" : "关系详情" }}
          </button>
          <button v-if="canManage" class="primary-button" type="button" @click="emit('edit', node)">
            维护关系
          </button>
        </div>
      </div>

      <div v-overlay-scrollbar class="substitute-network-detail__relations">
        <section>
          <h3>上游关系</h3>
          <p v-if="!incoming.length">没有物品将它设为替代物品。</p>
          <ul v-else>
            <li v-for="relation in incoming" :key="relation.edge.id">
              <strong>{{ relation.node.name }}</strong>
              <span>{{ relation.node.sku }} · 优先级 {{ relation.edge.priority }} → 当前物品</span>
              <small v-if="relation.edge.notes">{{ relation.edge.notes }}</small>
            </li>
          </ul>
        </section>
        <section>
          <h3>直接替代</h3>
          <p v-if="!outgoing.length">当前物品没有配置直接替代项。</p>
          <ul v-else>
            <li v-for="relation in outgoing" :key="relation.edge.id">
              <strong>{{ relation.node.name }}</strong>
              <span>当前物品 → {{ relation.node.sku }} · 优先级 {{ relation.edge.priority }}</span>
              <small v-if="relation.edge.notes">{{ relation.edge.notes }}</small>
            </li>
          </ul>
        </section>
      </div>
    </template>
  </section>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import {
  getReachableDownstreamNodeIds,
  type SubstituteNetworkEdge,
  type SubstituteNetworkGraph,
  type SubstituteNetworkNode,
} from "../../pages/substitutes/networkModel";

const props = defineProps<{
  graph: SubstituteNetworkGraph;
  selectedId: number | null;
  canManage: boolean;
}>();

const emit = defineEmits<{
  edit: [node: SubstituteNetworkNode];
}>();

interface RelatedNode {
  edge: SubstituteNetworkEdge;
  node: SubstituteNetworkNode;
}

const expanded = ref(false);
const node = computed(() =>
  props.selectedId === null ? null : (props.graph.nodeById.get(props.selectedId) ?? null),
);
const incoming = computed(() =>
  relatedNodes(props.graph.incomingById.get(props.selectedId ?? -1) ?? [], "sourceId"),
);
const outgoing = computed(() =>
  relatedNodes(props.graph.outgoingById.get(props.selectedId ?? -1) ?? [], "targetId"),
);
const reachableCount = computed(() =>
  props.selectedId === null ? 0 : getReachableDownstreamNodeIds(props.graph, props.selectedId).size,
);

watch(
  () => props.selectedId,
  () => {
    expanded.value = false;
  },
);

function relatedNodes(
  edges: readonly SubstituteNetworkEdge[],
  key: "sourceId" | "targetId",
): RelatedNode[] {
  return edges.flatMap((edge) => {
    const relatedNode = props.graph.nodeById.get(edge[key]);
    return relatedNode ? [{ edge, node: relatedNode }] : [];
  });
}
</script>

<style lang="scss" src="./SubstituteNetworkDialog.scss"></style>
