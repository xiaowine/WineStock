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
            :label="$t('items.searchItems')"
            :name="searchName"
            :placeholder="$t('items.selectionSearchPlaceholder')"
            @update:model-value="emit('update:search-input', $event)"
            @search="emit('search', $event)"
          />
        </div>
        <button
          v-if="canCreateItem"
          class="icon-button icon-button--primary item-selection-dialog__create"
          type="button"
          :title="$t('items.createItem')"
          :aria-label="$t('items.createItem')"
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
        {{ $t('items.loadingItems') }}
      </div>
      <div v-else-if="items.length === 0" class="item-selection-dialog__state">
        {{ $t('items.noConfigurableItems') }}
      </div>

      <div
        v-if="items.length > 0"
        :ref="captureList"
        v-overlay-scrollbar
        class="item-selection-dialog__list"
        :aria-label="$t('items.selectableItems')"
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
            :alt="$t('items.mainImageName', { name: item.name })"
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
                ? $t('items.continueConfiguring', { name: item.name })
                : $t('items.addAndConfigure', { name: item.name })
            "
            :title="
              selectedItemIds.has(item.id)
                ? $t('items.continueConfiguringShort')
                : $t('items.addAndConfigureShort')
            "
            @click="selectItem(item, $event)"
          >
            <svg v-if="selectedItemIds.has(item.id)" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M4 17.5V20h2.5L19 7.5 16.5 5 4 17.5Z" />
            </svg>
            <svg v-else viewBox="0 0 24 24" aria-hidden="true">
              <path d="M12 5v14M5 12h14" />
            </svg>
            <span>{{
              selectedItemIds.has(item.id)
                ? $t('items.continueConfiguringShort')
                : $t('items.addAndConfigureShort')
            }}</span>
          </button>
        </article>
        <div
          v-if="loadingItems"
          class="item-selection-dialog__state item-selection-dialog__state--tail"
          role="status"
        >
          {{ $t('items.loadingMoreItems') }}
        </div>
        <div
          v-else-if="itemsExhausted"
          class="item-selection-dialog__state item-selection-dialog__state--tail"
        >
          {{ $t('items.allItemsLoaded') }}
        </div>
      </div>
    </div>
  </ModalDialog>
</template>

<script setup lang="ts">
import { watch } from "vue";
import { useI18n } from "vue-i18n";
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
const { t } = useI18n();

watch(
  () => props.itemError,
  (error) => {
    if (error) {
      notice.error(t("items.loadItemsFailed"), {
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
