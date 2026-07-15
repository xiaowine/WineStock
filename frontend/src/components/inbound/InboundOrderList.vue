<!-- 本组件拥有入库单桌面/平板行与移动单列条目的呈现；它不请求数据、不管理分页或打开详情会话。 -->
<template>
  <div class="inbound-orders-table" role="table">
    <div class="inbound-orders-table__head" role="row">
      <span>单据与来源</span><span>入库明细</span><span>状态与操作</span>
    </div>
    <article
      v-for="order in orders"
      :key="order.id"
      class="inbound-orders-table__row"
      role="row"
      tabindex="0"
      @click="emit('open', order)"
      @keydown.enter="emit('open', order)"
      @keydown.space.prevent="emit('open', order)"
    >
      <div role="cell"><strong>入库单 #{{ order.id }}</strong><span>{{ order.source || '未记录来源' }}</span><time :datetime="order.created_at">{{ formatDate(order.created_at) }}</time></div>
      <div role="cell"><div class="inbound-order-item-summary"><AuthenticatedImage :file-id="order.items[0].item_image_file_id" :alt="`${order.items[0].item_name} 主图`" :size="34" previewable /><div><strong>{{ order.items[0].item_name }}</strong><small>{{ itemSummary(order) }}</small></div></div><span>{{ order.items.length }} 条明细 · {{ quantityLabel(order) }}</span><strong>¥{{ money(totalAmount(order)) }}</strong></div>
      <div class="inbound-orders-table__decision" role="cell"><span class="inbound-status" :class="`inbound-status--${order.status}`">{{ statusLabel(order.status) }}</span><time v-if="statusTime(order)" :datetime="statusTime(order)!">{{ formatDate(statusTime(order)!) }}</time><button class="icon-button" type="button" title="查看入库单详情" :aria-label="`查看入库单详情：${order.source || `入库单 #${order.id}`}`" @click.stop="emit('open', order)"><svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="8"/><path d="M12 11v5M12 8h.01"/></svg></button></div>
    </article>
  </div>

  <div class="inbound-orders-mobile-list" role="list">
    <article
      v-for="order in orders"
      :key="order.id"
      class="inbound-orders-mobile-list__item"
      role="listitem"
      tabindex="0"
      @click="emit('open', order)"
      @keydown.enter="emit('open', order)"
      @keydown.space.prevent="emit('open', order)"
    >
      <header>
        <strong>入库单 #{{ order.id }}</strong>
        <span class="inbound-status" :class="`inbound-status--${order.status}`">{{ statusLabel(order.status) }}</span>
        <button class="icon-button" type="button" title="查看入库单详情" :aria-label="`查看入库单详情：${order.source || `入库单 #${order.id}`}`" @click.stop="emit('open', order)">
          <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="8"/><path d="M12 11v5M12 8h.01"/></svg>
        </button>
      </header>
      <p class="inbound-orders-mobile-list__source">{{ order.source || '未记录来源' }}</p>
      <time :datetime="order.created_at">{{ formatDate(order.created_at) }}</time>
      <div class="inbound-orders-mobile-list__item-summary">
        <AuthenticatedImage :file-id="order.items[0].item_image_file_id" :alt="`${order.items[0].item_name} 主图`" :size="38" previewable />
        <div><strong>{{ order.items[0].item_name }}</strong><small>{{ itemSummary(order) }}</small></div>
      </div>
      <div class="inbound-orders-mobile-list__metrics"><span>{{ order.items.length }} 条明细 · {{ quantityLabel(order) }}</span><strong>¥{{ money(totalAmount(order)) }}</strong></div>
      <p class="inbound-orders-mobile-list__status-time">{{ statusHint(order) }}</p>
    </article>
  </div>
</template>

<script setup lang="ts">
import type { InboundOrderResponse, InboundOrderStatus } from '../../api/inboundOrders'
import AuthenticatedImage from '../attributes/AuthenticatedImage.vue'
import './InboundOrderList.scss'

defineProps<{
  orders: InboundOrderResponse[]
  formatDate: (value: string) => string
  money: (value: number) => string
  quantityLabel: (order: InboundOrderResponse) => string
  statusHint: (order: InboundOrderResponse) => string
  statusLabel: (status: InboundOrderStatus) => string
  statusTime: (order: InboundOrderResponse) => string | null
  totalAmount: (order: InboundOrderResponse) => number
}>()

const emit = defineEmits<{ open: [order: InboundOrderResponse] }>()

function itemSummary(order: InboundOrderResponse): string {
  const item = order.items[0]
  return `${item.item_sku} · ${item.quantity} ${item.item_unit}${order.items.length > 1 ? ` · 等 ${order.items.length} 项` : ''}`
}
</script>
