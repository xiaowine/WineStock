<!-- 本组件只读呈现入库审批明细和增加库存后果；它不修改单据或执行审批。 -->
<template>
  <section class="approval-detail-lines">
    <h3>
      入库物品 <span>{{ order.items.length }} 条</span>
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
        <b>¥{{ money(item.quantity * item.unit_price) }}</b>
      </header>
      <dl>
        <div>
          <dt>数量</dt>
          <dd>{{ item.quantity }} {{ item.item_unit }}</dd>
        </div>
        <div>
          <dt>单价</dt>
          <dd>¥{{ money(item.unit_price) }}</dd>
        </div>
        <div>
          <dt>目标库位</dt>
          <dd>{{ item.location_name }}</dd>
        </div>
        <div>
          <dt>批次</dt>
          <dd>{{ item.batch_no || "自动生成" }}</dd>
        </div>
        <div>
          <dt>有效期</dt>
          <dd>{{ item.expires_at || "未设置" }}</dd>
        </div>
      </dl>
    </article>
  </section>
</template>
<script setup lang="ts">
import type { InboundOrderResponse } from "../../api/inboundOrders";
import AuthenticatedImage from "../attributes/AuthenticatedImage.vue";
defineProps<{ order: InboundOrderResponse }>();
function money(value: number): string {
  return new Intl.NumberFormat("zh-CN", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(value);
}
</script>
