<!--
  本组件拥有库位分组树的扁平可访问呈现、展开状态和操作入口。
  它不请求 API，也不决定分组能否被服务端移动或删除。
-->
<template>
  <div class="location-group-tree" role="tree" aria-label="库位分组">
    <div
      class="location-group-tree__row location-group-tree__row--all"
      role="treeitem"
      :aria-selected="selectedGroupId === null"
    >
      <span class="location-group-tree__toggle-placeholder" aria-hidden="true"></span>
      <button
        :ref="(element) => setLabelButton(element, 0)"
        class="location-group-tree__label"
        :class="{ 'is-selected': selectedGroupId === null }"
        type="button"
        @keydown="handleTreeKeydown($event, 0, null)"
        @click="emit('select', null)"
      >
        <span>全部库位</span>
        <small>{{ totalLocations }}</small>
      </button>
    </div>

    <div
      v-for="(row, rowIndex) in rows"
      :key="row.node.id"
      class="location-group-tree__row"
      role="treeitem"
      :aria-level="row.depth + 1"
      :aria-expanded="row.hasChildren ? expandedGroupIds.includes(row.node.id) : undefined"
      :aria-selected="selectedGroupId === row.node.id"
      :style="{ '--location-tree-depth': row.depth }"
    >
      <button
        v-if="row.hasChildren"
        class="location-group-tree__toggle"
        type="button"
        :title="expandedGroupIds.includes(row.node.id) ? '收起分组' : '展开分组'"
        :aria-label="
          expandedGroupIds.includes(row.node.id) ? `收起 ${row.node.name}` : `展开 ${row.node.name}`
        "
        @click="emit('toggle', row.node.id)"
      >
        <svg
          viewBox="0 0 16 16"
          aria-hidden="true"
          :class="{ 'is-expanded': expandedGroupIds.includes(row.node.id) }"
        >
          <path d="m6 3 5 5-5 5" />
        </svg>
      </button>
      <span v-else class="location-group-tree__toggle-placeholder" aria-hidden="true"></span>

      <button
        :ref="(element) => setLabelButton(element, rowIndex + 1)"
        class="location-group-tree__label"
        :class="{ 'is-selected': selectedGroupId === row.node.id }"
        type="button"
        :title="row.node.name"
        @keydown="handleTreeKeydown($event, rowIndex + 1, row)"
        @click="emit('select', row.node.id)"
      >
        <span>{{ row.node.name }}</span>
        <small>{{ row.node.locations.length }}</small>
      </button>

      <span v-if="canManage" class="location-group-tree__actions">
        <button
          type="button"
          :title="row.depth + 1 >= MAX_LOCATION_GROUP_DEPTH ? '已达到 10 层上限' : '新建子分组'"
          :aria-label="
            row.depth + 1 >= MAX_LOCATION_GROUP_DEPTH
              ? `${row.node.name} 已达到 10 层上限`
              : `在 ${row.node.name} 下新建分组`
          "
          :disabled="row.depth + 1 >= MAX_LOCATION_GROUP_DEPTH"
          @click="emit('create-child', row.node)"
        >
          <svg viewBox="0 0 20 20" aria-hidden="true"><path d="M10 4v12M4 10h12" /></svg>
        </button>
        <button
          type="button"
          title="编辑分组"
          :aria-label="`编辑分组 ${row.node.name}`"
          @click="emit('edit', row.node)"
        >
          <svg viewBox="0 0 20 20" aria-hidden="true">
            <path d="m4 14-.5 2.5L6 16l8.5-8.5-2-2L4 14Z" />
            <path d="m11.5 6.5 2 2" />
          </svg>
        </button>
        <button
          type="button"
          title="删除分组"
          :aria-label="`删除分组 ${row.node.name}`"
          @click="emit('delete', row.node)"
        >
          <svg viewBox="0 0 20 20" aria-hidden="true">
            <path d="M4 6h12M8 6V4h4v2M6 6l1 10h6l1-10M9 9v4M11 9v4" />
          </svg>
        </button>
      </span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, type ComponentPublicInstance } from "vue";
import type { LocationGroupTreeNode } from "../../api/locations";

interface LocationGroupTreeRow {
  node: LocationGroupTreeNode;
  depth: number;
  hasChildren: boolean;
}

const MAX_LOCATION_GROUP_DEPTH = 10;

const props = defineProps<{
  nodes: LocationGroupTreeNode[];
  selectedGroupId: number | null;
  expandedGroupIds: number[];
  canManage: boolean;
}>();

const emit = defineEmits<{
  select: [groupId: number | null];
  toggle: [groupId: number];
  "create-child": [group: LocationGroupTreeNode];
  edit: [group: LocationGroupTreeNode];
  delete: [group: LocationGroupTreeNode];
}>();

const rows = computed(() => flattenVisibleGroups(props.nodes, new Set(props.expandedGroupIds)));
const totalLocations = computed(() => countLocations(props.nodes));
const labelButtons = new Map<number, HTMLButtonElement>();

function setLabelButton(element: Element | ComponentPublicInstance | null, index: number): void {
  const button = element instanceof HTMLButtonElement ? element : null;
  if (button) labelButtons.set(index, button);
  else labelButtons.delete(index);
}

/** 使用方向键在可见树节点间移动；展开和收起仍由父级状态统一管理。 */
function handleTreeKeydown(
  event: KeyboardEvent,
  index: number,
  row: LocationGroupTreeRow | null,
): void {
  const lastIndex = rows.value.length;
  if (event.key === "ArrowDown") {
    event.preventDefault();
    labelButtons.get(Math.min(index + 1, lastIndex))?.focus();
    return;
  }
  if (event.key === "ArrowUp") {
    event.preventDefault();
    labelButtons.get(Math.max(index - 1, 0))?.focus();
    return;
  }
  if (event.key === "Home") {
    event.preventDefault();
    labelButtons.get(0)?.focus();
    return;
  }
  if (event.key === "End") {
    event.preventDefault();
    labelButtons.get(lastIndex)?.focus();
    return;
  }
  if (!row) return;
  const expanded = props.expandedGroupIds.includes(row.node.id);
  if (event.key === "ArrowRight" && row.hasChildren) {
    event.preventDefault();
    if (!expanded) emit("toggle", row.node.id);
    else labelButtons.get(Math.min(index + 1, lastIndex))?.focus();
    return;
  }
  if (event.key === "ArrowLeft") {
    event.preventDefault();
    if (expanded) {
      emit("toggle", row.node.id);
      return;
    }
    const parentIndex = findVisibleParentIndex(index - 1, row.depth);
    labelButtons.get(parentIndex)?.focus();
  }
}

function findVisibleParentIndex(startIndex: number, depth: number): number {
  for (let index = startIndex; index > 0; index -= 1) {
    const candidate = rows.value[index - 1];
    if (candidate && candidate.depth < depth) return index;
  }
  return 0;
}

function flattenVisibleGroups(
  nodes: LocationGroupTreeNode[],
  expanded: ReadonlySet<number>,
  depth = 0,
): LocationGroupTreeRow[] {
  const rows: LocationGroupTreeRow[] = [];
  for (const node of nodes) {
    rows.push({ node, depth, hasChildren: node.children.length > 0 });
    if (expanded.has(node.id)) {
      rows.push(...flattenVisibleGroups(node.children, expanded, depth + 1));
    }
  }
  return rows;
}

function countLocations(nodes: LocationGroupTreeNode[]): number {
  return nodes.reduce(
    (total, node) => total + node.locations.length + countLocations(node.children),
    0,
  );
}
</script>

<style lang="scss" src="./LocationGroupTree.scss"></style>
