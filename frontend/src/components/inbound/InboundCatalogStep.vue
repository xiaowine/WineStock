<!-- 本组件拥有正式入库流程的物品选择步骤；它不保存草稿、不加载模板，也不提交入库单。 -->
<template>
  <section class="inbound-step inbound-catalog-step" aria-labelledby="inbound-catalog-step-title">
    <header class="inbound-step__header">
      <div>
        <h2 id="inbound-catalog-step-title">选择入库物品</h2>
      </div>
      <div class="inbound-step__actions">
        <button
          v-if="canCreateItem"
          class="icon-button inbound-create-item-button"
          type="button"
          title="新建物品"
          aria-label="新建物品"
          @click="$emit('create-item')"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
        </button>
        <button class="secondary-button inbound-step-nav-button" type="button" :disabled="!canContinue" @click="$emit('continue')">
          下一步：填写单据
        </button>
      </div>
    </header>

    <div class="inbound-catalog-step__search" role="search">
      <SearchField
        :model-value="searchInput"
        label="搜索物品"
        name="inbound_item_search"
        placeholder="名称、SKU 或模板属性"
        @update:model-value="$emit('update:search-input', $event)"
        @search="$emit('search', $event)"
      />
    </div>

    <div v-if="itemError && items.length === 0" class="inbound-panel-state inbound-panel-state--error" role="alert">
      <p>{{ itemError }}</p>
      <button class="text-button" type="button" @click="$emit('reset-items')">重试</button>
    </div>
    <div v-else-if="loadingItems && items.length === 0" class="inbound-panel-state" role="status">正在加载物品…</div>
    <div v-else-if="items.length === 0" class="inbound-panel-state">没有找到可加入入库单的物品。</div>

    <div v-if="items.length > 0" class="inbound-catalog-step__list-header" aria-hidden="true">
      <span>物品</span>
      <span>操作</span>
    </div>
    <div
      v-if="items.length > 0"
      :ref="captureList"
      class="inbound-catalog-step__list"
      aria-label="可选物品"
      @scroll.passive="$emit('scroll-items')"
    >
      <article
        v-for="item in items"
        :key="item.id"
        class="inbound-catalog-step__item"
        :class="{ 'inbound-catalog-step__item--selected': draftCounts.has(item.id) }"
      >
        <AuthenticatedImage :file-id="item.image_file_id" :alt="`${item.name} 主图`" :size="34" previewable />
        <div class="inbound-catalog-step__identity">
          <strong :title="item.name">{{ item.name }}</strong>
        </div>
        <span class="inbound-catalog-step__meta">{{ item.sku }} · {{ item.unit }}</span>
        <button
          class="inbound-catalog-step__toggle"
          :class="{ 'inbound-catalog-step__toggle--selected': draftCounts.has(item.id) }"
          type="button"
          :aria-label="draftCounts.has(item.id) ? `将 ${item.name} 移出入库单` : `将 ${item.name} 加入入库单`"
          :aria-pressed="draftCounts.has(item.id)"
          :title="draftCounts.has(item.id) ? '移出入库单' : '加入入库单'"
          @click="toggleItem(item, $event)"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
            <path v-if="draftCounts.has(item.id)" d="M5 12h14" />
            <path v-else d="M12 5v14M5 12h14" />
          </svg>
        </button>
      </article>
      <div v-if="loadingItems" class="inbound-panel-state inbound-catalog-step__load-state" role="status">正在加载更多物品…</div>
      <div v-else-if="itemError" class="inbound-panel-state inbound-panel-state--error inbound-catalog-step__load-state" role="alert">
        <p>{{ itemError }}</p>
        <button class="text-button" type="button" @click="$emit('load-next-items')">重试本页</button>
      </div>
      <div v-else-if="itemsExhausted" class="inbound-panel-state inbound-catalog-step__load-state">已加载全部物品</div>
    </div>
  </section>
</template>

<script setup lang="ts">
import type { ItemOptionResponse } from '../../api/items'
import AuthenticatedImage from '../attributes/AuthenticatedImage.vue'
import SearchField from '../SearchField.vue'

defineProps<{
  items: ItemOptionResponse[]
  searchInput: string
  loadingItems: boolean
  itemError: string
  itemsExhausted: boolean
  draftCounts: Map<number, number>
  canContinue: boolean
  canCreateItem: boolean
}>()

const emit = defineEmits<{
  'update:search-input': [value: string]
  search: [value: string]
  'reset-items': []
  'load-next-items': []
  'scroll-items': []
  'list-element': [element: HTMLElement | null]
  'toggle-item': [item: ItemOptionResponse]
  'create-item': []
  continue: []
}>()

function captureList(element: unknown): void {
  emit('list-element', element instanceof HTMLElement ? element : null)
}

/** 触屏切换后释放粘滞焦点；键盘操作继续保留全局 focus-visible 反馈。 */
function toggleItem(item: ItemOptionResponse, event: MouseEvent): void {
  emit('toggle-item', item)

  if (event.detail > 0 && window.matchMedia('(hover: none), (pointer: coarse)').matches) {
    const trigger = event.currentTarget
    if (trigger instanceof HTMLButtonElement) {
      trigger.blur()
    }
  }
}
</script>
