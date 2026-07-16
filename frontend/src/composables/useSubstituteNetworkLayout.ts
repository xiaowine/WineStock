// 本文件拥有替代关系网络的确定性力导向布局和页面会话位置缓存；它不渲染 SVG。
import { onBeforeUnmount, shallowRef } from "vue";
import type {
  SubstituteNetworkEdge,
  SubstituteNetworkNode,
} from "../pages/substitutes/networkModel";

export interface SubstituteNetworkPosition {
  x: number;
  y: number;
  pinned: boolean;
}

interface Velocity {
  x: number;
  y: number;
}

const MAX_FRAMES = 150;
const MIN_ENERGY = 0.08;
const sessionPositionCache = new Map<number, SubstituteNetworkPosition>();

export function useSubstituteNetworkLayout() {
  const positions = shallowRef<Record<number, SubstituteNetworkPosition>>({});
  const settling = shallowRef(false);
  const velocities = new Map<number, Velocity>();
  let activeNodes: readonly SubstituteNetworkNode[] = [];
  let activeEdges: readonly SubstituteNetworkEdge[] = [];
  let animationFrame = 0;
  let frameCount = 0;
  let stopTimer: number | undefined;

  function mergeGraph(
    nodes: readonly SubstituteNetworkNode[],
    edges: readonly SubstituteNetworkEdge[],
    restartLayout = true,
  ): void {
    const next = Object.fromEntries(
      Array.from(sessionPositionCache, ([nodeId, position]) => [nodeId, { ...position }]),
    ) as Record<number, SubstituteNetworkPosition>;
    Object.assign(next, positions.value);
    const activeIds = new Set(nodes.map((node) => node.id));
    const neighborPositions = new Map<number, SubstituteNetworkPosition[]>();

    for (const edge of edges) {
      const source = next[edge.sourceId];
      const target = next[edge.targetId];
      if (source && !target) appendNeighbor(neighborPositions, edge.targetId, source);
      if (target && !source) appendNeighbor(neighborPositions, edge.sourceId, target);
    }

    for (const node of nodes) {
      if (next[node.id]) continue;
      const neighbors = neighborPositions.get(node.id);
      next[node.id] = neighbors?.length
        ? deterministicNeighborPosition(node.id, neighbors)
        : deterministicPosition(node.id, nodes.length);
    }

    activeNodes = nodes;
    activeEdges = edges.filter(
      (edge) => activeIds.has(edge.sourceId) && activeIds.has(edge.targetId),
    );
    positions.value = next;
    if (restartLayout) restart();
  }

  function restart(): void {
    stop();
    if (!activeNodes.length) return;
    frameCount = 0;
    for (const node of activeNodes) velocities.set(node.id, { x: 0, y: 0 });

    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      for (let index = 0; index < 80; index += 1) step();
      positions.value = { ...positions.value };
      settling.value = false;
      return;
    }

    settling.value = true;
    stopTimer = window.setTimeout(stop, 3_000);
    animationFrame = window.requestAnimationFrame(runFrame);
  }

  function reset(): void {
    const next = { ...positions.value };
    for (const node of activeNodes)
      next[node.id] = deterministicPosition(node.id, activeNodes.length);
    positions.value = next;
    syncActivePositionsToCache();
    restart();
  }

  function moveNode(nodeId: number, x: number, y: number, pinned = true): void {
    const current = positions.value[nodeId];
    if (!current) return;
    positions.value = { ...positions.value, [nodeId]: { x, y, pinned } };
    sessionPositionCache.set(nodeId, { x, y, pinned });
    velocities.set(nodeId, { x: 0, y: 0 });
  }

  function releaseNode(nodeId: number): void {
    const current = positions.value[nodeId];
    if (current) {
      const pinned = { ...current, pinned: true };
      positions.value = { ...positions.value, [nodeId]: pinned };
      sessionPositionCache.set(nodeId, pinned);
    }
  }

  function stop(): void {
    if (animationFrame) window.cancelAnimationFrame(animationFrame);
    window.clearTimeout(stopTimer);
    stopTimer = undefined;
    animationFrame = 0;
    settling.value = false;
    syncActivePositionsToCache();
  }

  function runFrame(): void {
    const energy = step();
    positions.value = { ...positions.value };
    syncActivePositionsToCache();
    frameCount += 1;
    if (energy <= MIN_ENERGY || frameCount >= MAX_FRAMES) {
      stop();
      return;
    }
    animationFrame = window.requestAnimationFrame(runFrame);
  }

  function step(): number {
    const next = positions.value;
    const forces = new Map<number, Velocity>(activeNodes.map((node) => [node.id, { x: 0, y: 0 }]));
    const repulsion = activeNodes.length > 100 ? 7600 : 10500;

    for (let leftIndex = 0; leftIndex < activeNodes.length; leftIndex += 1) {
      const left = activeNodes[leftIndex];
      const leftPosition = next[left.id];
      if (!leftPosition) continue;
      for (let rightIndex = leftIndex + 1; rightIndex < activeNodes.length; rightIndex += 1) {
        const right = activeNodes[rightIndex];
        const rightPosition = next[right.id];
        if (!rightPosition) continue;
        let deltaX = leftPosition.x - rightPosition.x;
        let deltaY = leftPosition.y - rightPosition.y;
        if (deltaX === 0 && deltaY === 0) deltaX = ((left.id % 7) - 3) * 0.1 || 0.1;
        const distanceSquared = Math.max(64, deltaX * deltaX + deltaY * deltaY);
        const distance = Math.sqrt(distanceSquared);
        const force = Math.min(9, repulsion / distanceSquared);
        const forceX = (deltaX / distance) * force;
        const forceY = (deltaY / distance) * force;
        forces.get(left.id)!.x += forceX;
        forces.get(left.id)!.y += forceY;
        forces.get(right.id)!.x -= forceX;
        forces.get(right.id)!.y -= forceY;
      }
    }

    for (const edge of activeEdges) {
      const source = next[edge.sourceId];
      const target = next[edge.targetId];
      if (!source || !target) continue;
      const deltaX = target.x - source.x;
      const deltaY = target.y - source.y;
      const distance = Math.max(1, Math.hypot(deltaX, deltaY));
      const stretch = (distance - 126) * 0.018;
      const forceX = (deltaX / distance) * stretch;
      const forceY = (deltaY / distance) * stretch;
      forces.get(edge.sourceId)!.x += forceX;
      forces.get(edge.sourceId)!.y += forceY;
      forces.get(edge.targetId)!.x -= forceX;
      forces.get(edge.targetId)!.y -= forceY;
    }

    let totalEnergy = 0;
    for (const node of activeNodes) {
      const position = next[node.id];
      if (!position || position.pinned) continue;
      const force = forces.get(node.id)!;
      force.x += -position.x * 0.0022;
      force.y += -position.y * 0.0022;
      const velocity = velocities.get(node.id) ?? { x: 0, y: 0 };
      velocity.x = (velocity.x + force.x) * 0.78;
      velocity.y = (velocity.y + force.y) * 0.78;
      velocity.x = clamp(velocity.x, -12, 12);
      velocity.y = clamp(velocity.y, -12, 12);
      position.x += velocity.x;
      position.y += velocity.y;
      velocities.set(node.id, velocity);
      totalEnergy += Math.abs(velocity.x) + Math.abs(velocity.y);
    }
    return totalEnergy / Math.max(1, activeNodes.length);
  }

  onBeforeUnmount(stop);

  return { positions, settling, mergeGraph, restart, reset, moveNode, releaseNode, stop };

  function syncActivePositionsToCache(): void {
    for (const node of activeNodes) {
      const position = positions.value[node.id];
      if (position) sessionPositionCache.set(node.id, { ...position });
    }
  }
}

function deterministicPosition(nodeId: number, count: number): SubstituteNetworkPosition {
  const angle = seededUnit(nodeId * 97) * Math.PI * 2;
  const ring = 70 + seededUnit(nodeId * 193) * Math.max(120, Math.sqrt(Math.max(1, count)) * 46);
  return { x: Math.cos(angle) * ring, y: Math.sin(angle) * ring, pinned: false };
}

function deterministicNeighborPosition(
  nodeId: number,
  neighbors: readonly SubstituteNetworkPosition[],
): SubstituteNetworkPosition {
  const centerX = neighbors.reduce((total, position) => total + position.x, 0) / neighbors.length;
  const centerY = neighbors.reduce((total, position) => total + position.y, 0) / neighbors.length;
  const angle = seededUnit(nodeId * 311) * Math.PI * 2;
  return { x: centerX + Math.cos(angle) * 68, y: centerY + Math.sin(angle) * 68, pinned: false };
}

function appendNeighbor(
  map: Map<number, SubstituteNetworkPosition[]>,
  nodeId: number,
  position: SubstituteNetworkPosition,
): void {
  const current = map.get(nodeId);
  if (current) current.push(position);
  else map.set(nodeId, [position]);
}

function seededUnit(seed: number): number {
  const value = Math.sin(seed * 12.9898) * 43758.5453;
  return value - Math.floor(value);
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.min(maximum, Math.max(minimum, value));
}
