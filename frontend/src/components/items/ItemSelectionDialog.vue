<!--
  本组件拥有出入库工作台的单物品选择弹窗；它只负责目录搜索、分页和选择呈现，
  不创建草稿、不编辑明细，也不调用业务 API。
-->
<template>
  <ModalDialog
    :open="open"
    :title="title"
    :description="description"
    workspace
    @close="emit('close')"
    @after-close="emit('after-close')"
  >
    <div class="item-selection-dialog">
      <header class="item-selection-dialog__toolbar">
        <div class="item-selection-dialog__search" role="search">
          <SearchField
            :model-value="searchInput"
            label="搜索物品"
            :name="searchName"
            placeholder="名称、编号或模板属性"
            @update:model-value="emit('update:search-input', $event)"
            @search="emit('search', $event)"
          />
        </div>
        <button
          v-if="canCreateItem"
          class="icon-button icon-button--primary item-selection-dialog__create"
          type="button"
          title="新建物品"
          aria-label="新建物品"
          @click="emit('create-item')"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
            <path d="M12 5v14M5 12h14" />
          </svg>
        </button>
      </header>

      <div
        v-if="loadingItems && items.length === 0"
        class="item-selection-dialog__state"
        role="status"
      >
        正在加载物品…
      </div>
      <div v-else-if="items.length === 0" class="item-selection-dialog__state">
        没有找到可配置的物品。
      </div>

      <div
        v-if="items.length > 0"
        :ref="captureList"
        v-overlay-scrollbar
        class="item-selection-dialog__list"
        aria-label="可选择物品"
        @scroll.passive="emit('scroll-items')"
      >
        <article
          v-for="item in items"
          :key="item.id"
          class="item-selection-dialog__item"
          :class="{ 'item-selection-dialog__item--selected': selectedItemIds.has(item.id) }"
        >
          <AuthenticatedImage
            :file-id="item.image_file_id"
            :alt="`${item.name} 主图`"
            :size="40"
            previewable
          />
          <div class="item-selection-dialog__identity">
            <strong :title="item.name">{{ item.name }}</strong>
            <small>{{ item.sku }} · {{ item.unit }}</small>
          </div>
          <button
            class="secondary-button item-selection-dialog__select"
            :class="{ 'item-selection-dialog__select--selected': selectedItemIds.has(item.id) }"
            type="button"
            :data-item-action="item.id"
            :aria-label="
              selectedItemIds.has(item.id)
                ? `继续配置 ${item.name} 的明细`
                : `添加并配置 ${item.name}`
            "
            :title="selectedItemIds.has(item.id) ? '继续配置' : '添加并配置'"
            @click="selectItem(item, $event)"
          >
            <svg v-if="selectedItemIds.has(item.id)" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M4 17.5V20h2.5L19 7.5 16.5 5 4 17.5Z" />
            </svg>
            <svg v-else viewBox="0 0 24 24" aria-hidden="true">
              <path d="M12 5v14M5 12h14" />
            </svg>
            <span>{{ selectedItemIds.has(item.id) ? "继续配置" : "添加并配置" }}</span>
          </button>
        </article>
        <div
          v-if="loadingItems"
          class="item-selection-dialog__state item-selection-dialog__state--tail"
          role="status"
        >
          正在加载更多物品…
        </div>
        <div
          v-else-if="itemsExhausted"
          class="item-selection-dialog__state item-selection-dialog__state--tail"
        >
          已加载全部物品
        </div>
      </div>
    </div>
  </ModalDialog>
</template>

<script setup lang="ts">
import { watch } from "vue";
import type { ItemOptionResponse } from "../../api/items";
import AuthenticatedImage from "../attributes/AuthenticatedImage.vue";
import ModalDialog from "../ModalDialog.vue";
import SearchField from "../SearchField.vue";
import { notice } from "../../notices/notice";

const props = withDefaults(
  defineProps<{
    open: boolean;
    title: string;
    description: string;
    searchName: string;
    items: ItemOptionResponse[];
    searchInput: string;
    loadingItems: boolean;
    itemError: string;
    itemsExhausted: boolean;
    selectedItemIds: ReadonlySet<number>;
    canCreateItem: boolean;
  }>(),
  { canCreateItem: false },
);

const emit = defineEmits<{
  close: [];
  "after-close": [];
  "update:search-input": [value: string];
  search: [value: string];
  "reset-items": [];
  "load-next-items": [];
  "scroll-items": [];
  "list-element": [element: HTMLElement | null];
  "select-item": [item: ItemOptionResponse];
  "create-item": [];
}>();

watch(
  () => props.itemError,
  (error) => {
    if (error) {
      notice.error("加载物品失败", {
        detail: error,
        onClick: () => {
          if (props.items.length === 0) emit("reset-items");
          else emit("load-next-items");
        },
      });
    }
  },
);

function captureList(element: unknown): void {
  emit("list-element", element instanceof HTMLElement ? element : null);
}

function selectItem(item: ItemOptionResponse, event: MouseEvent): void {
  emit("select-item", item);
  if (event.detail > 0 && window.matchMedia("(hover: none), (pointer: coarse)").matches) {
    const trigger = event.currentTarget;
    if (trigger instanceof HTMLButtonElement) trigger.blur();
  }
}
</script>

<style lang="scss" src="./ItemSelectionDialog.scss"></style>
