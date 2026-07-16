// 本文件拥有替代关系网络的节点、边和邻接派生纯函数；它不请求 API 或管理画布状态。
import type { SubstituteRelationResponse } from "../../api/substitutes";

export interface SubstituteNetworkNode {
  id: number;
  name: string;
  sku: string;
  incomingCount: number;
  outgoingCount: number;
  degree: number;
}

export interface SubstituteNetworkEdge {
  id: string;
  sourceId: number;
  targetId: number;
  priority: number;
  notes: string | null;
  createdAt: string;
}

export interface SubstituteNetworkGraph {
  nodes: SubstituteNetworkNode[];
  edges: SubstituteNetworkEdge[];
  nodeById: Map<number, SubstituteNetworkNode>;
  incomingById: Map<number, SubstituteNetworkEdge[]>;
  outgoingById: Map<number, SubstituteNetworkEdge[]>;
}

interface MutableNode {
  id: number;
  name: string;
  sku: string;
  incomingCount: number;
  outgoingCount: number;
}

/** 将全量有向关系转换为稳定、去重的网络结构。 */
export function buildSubstituteNetwork(
  relations: readonly SubstituteRelationResponse[],
): SubstituteNetworkGraph {
  const mutableNodes = new Map<number, MutableNode>();
  const edgeById = new Map<string, SubstituteNetworkEdge>();

  for (const relation of relations) {
    ensureNode(mutableNodes, relation.item_id, relation.item_name, relation.item_sku);
    ensureNode(
      mutableNodes,
      relation.substitute_item_id,
      relation.substitute_item_name,
      relation.substitute_item_sku,
    );

    const edgeId = `${relation.item_id}:${relation.substitute_item_id}`;
    if (edgeById.has(edgeId)) continue;

    edgeById.set(edgeId, {
      id: edgeId,
      sourceId: relation.item_id,
      targetId: relation.substitute_item_id,
      priority: relation.priority,
      notes: relation.notes,
      createdAt: relation.created_at,
    });
    mutableNodes.get(relation.item_id)!.outgoingCount += 1;
    mutableNodes.get(relation.substitute_item_id)!.incomingCount += 1;
  }

  const nodes = Array.from(mutableNodes.values())
    .map((node): SubstituteNetworkNode => ({
      ...node,
      degree: node.incomingCount + node.outgoingCount,
    }))
    .sort(compareNetworkNodes);
  const edges = Array.from(edgeById.values()).sort(
    (left, right) =>
      left.sourceId - right.sourceId ||
      left.priority - right.priority ||
      left.targetId - right.targetId,
  );
  const nodeById = new Map(nodes.map((node) => [node.id, node]));
  const incomingById = new Map<number, SubstituteNetworkEdge[]>();
  const outgoingById = new Map<number, SubstituteNetworkEdge[]>();

  for (const node of nodes) {
    incomingById.set(node.id, []);
    outgoingById.set(node.id, []);
  }
  for (const edge of edges) {
    incomingById.get(edge.targetId)?.push(edge);
    outgoingById.get(edge.sourceId)?.push(edge);
  }

  return { nodes, edges, nodeById, incomingById, outgoingById };
}

/** 返回当前节点和一跳上下游组成的节点集合。 */
export function getDirectRelationNodeIds(
  graph: SubstituteNetworkGraph,
  nodeId: number,
): Set<number> {
  if (!graph.nodeById.has(nodeId)) return new Set();
  const nodeIds = new Set<number>([nodeId]);
  for (const edge of graph.incomingById.get(nodeId) ?? []) nodeIds.add(edge.sourceId);
  for (const edge of graph.outgoingById.get(nodeId) ?? []) nodeIds.add(edge.targetId);
  return nodeIds;
}

/** 返回从指定节点沿替代方向能够到达的全部下游节点。 */
export function getReachableDownstreamNodeIds(
  graph: SubstituteNetworkGraph,
  nodeId: number,
): Set<number> {
  const visited = new Set<number>();
  const pending = [nodeId];
  while (pending.length) {
    const current = pending.shift()!;
    for (const edge of graph.outgoingById.get(current) ?? []) {
      if (visited.has(edge.targetId) || edge.targetId === nodeId) continue;
      visited.add(edge.targetId);
      pending.push(edge.targetId);
    }
  }
  return visited;
}

/** 按名称、SKU 和连接度搜索网络节点。 */
export function searchSubstituteNetworkNodes(
  graph: SubstituteNetworkGraph,
  search: string,
  limit = 12,
): SubstituteNetworkNode[] {
  const keyword = search.trim().toLocaleLowerCase("zh-CN");
  if (!keyword) return [];
  return graph.nodes
    .filter(
      (node) =>
        node.name.toLocaleLowerCase("zh-CN").includes(keyword) ||
        node.sku.toLocaleLowerCase("zh-CN").includes(keyword),
    )
    .sort((left, right) => right.degree - left.degree || compareNetworkNodes(left, right))
    .slice(0, limit);
}

/** 根据规模和中心节点选择当前允许渲染的节点集合。 */
export function getRenderableNodeIds(
  graph: SubstituteNetworkGraph,
  selectedId: number | null,
): Set<number> {
  if (graph.nodes.length <= 180) return new Set(graph.nodes.map((node) => node.id));
  if (selectedId !== null && graph.nodeById.has(selectedId)) {
    const direct = getDirectRelationNodeIds(graph, selectedId);
    if (graph.nodes.length > 300) return direct;
    const expanded = new Set(direct);
    for (const nodeId of direct) {
      for (const relatedId of getDirectRelationNodeIds(graph, nodeId)) expanded.add(relatedId);
    }
    return expanded;
  }
  if (graph.nodes.length > 300) return new Set();
  return new Set(
    graph.nodes
      .slice()
      .sort((left, right) => right.degree - left.degree || left.id - right.id)
      .slice(0, 80)
      .map((node) => node.id),
  );
}

function ensureNode(nodes: Map<number, MutableNode>, id: number, name: string, sku: string): void {
  if (!nodes.has(id)) nodes.set(id, { id, name, sku, incomingCount: 0, outgoingCount: 0 });
}

function compareNetworkNodes(
  left: Pick<SubstituteNetworkNode, "name" | "id">,
  right: Pick<SubstituteNetworkNode, "name" | "id">,
): number {
  return left.name.localeCompare(right.name, "zh-CN") || left.id - right.id;
}
