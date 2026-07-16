<!-- 本组件只读呈现出库审批明细、批次/FIFO 规则和扣减库存后果；它不预测库存或执行审批。 -->
<template>
  <section class="approval-detail-lines">
    <h3>
      出库物品 <span>{{ order.items.length }} 条</span>
    </h3>
    <article v-for="item in order.items" :key="item.id">
      <header>
        <AuthenticatedImage
          :file-id="item.item_image_file_id"
          :alt="`${item.item_name} 主图`"
          :size="52"
          previewable
        />
        <div>
          <strong>{{ item.item_name }}</strong
          ><small>{{ item.item_sku }} · {{ item.item_unit }} · 物品 #{{ item.item_id }}</small>
        </div>
        <b>{{ item.quantity }} {{ item.item_unit }}</b>
      </header>
      <dl>
        <div>
          <dt>申请数量</dt>
          <dd>{{ item.quantity }} {{ item.item_unit }}</dd>
        </div>
        <div>
          <dt>限制库位</dt>
          <dd>{{ item.location_name || '全部库位' }}</dd>
        </div>
        <div>
          <dt>扣减方式</dt>
          <dd>
            {{ item.batch_id ? `指定批次 #${item.batch_id}` : '审批时按 FIFO 分配' }}
          </dd>
        </div>
      </dl>
    </article>
    <p class="approval-fifo-note">
      未指定批次时按有效期优先，其次按入库时间和批次顺序分配；最终结果以审批事务的实时库存检查为准。
    </p>
  </section>
</template>
<script setup lang="ts">
import type { OutboundOrderResponse } from '../../api/outboundOrders'
import AuthenticatedImage from '../attributes/AuthenticatedImage.vue'
defineProps<{ order: OutboundOrderResponse }>()
</script>
