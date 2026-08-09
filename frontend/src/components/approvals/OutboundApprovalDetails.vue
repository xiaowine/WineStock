<!-- 本组件只读呈现出库审批明细、批次/FIFO 规则和扣减库存后果；它不预测库存或执行审批。 -->
<template>
  <section class="approval-detail-lines">
    <h3>
      {{ $t('approvals.outboundItemsLabel') }} <span>{{ $t('approvals.lineCount', { n: order.items.length }) }}</span>
    </h3>
    <article v-for="item in order.items" :key="item.id">
      <header>
        <AuthenticatedImage
          :file-id="item.item_image_file_id"
          :alt="$t('approvals.itemMainImage', { name: item.item_name })"
          :size="52"
          previewable
        />
        <div>
          <strong>{{ item.item_name }}</strong
          ><small>{{ item.item_sku }} · {{ item.item_unit }} · {{ $t('approvals.itemRef', { n: item.item_id }) }}</small>
        </div>
        <b>{{ item.quantity }} {{ item.item_unit }}</b>
      </header>
      <dl>
        <div>
          <dt>{{ $t('approvals.requestedQuantity') }}</dt>
          <dd>{{ item.quantity }} {{ item.item_unit }}</dd>
        </div>
        <div>
          <dt>{{ $t('approvals.restrictedLocation') }}</dt>
          <dd>{{ item.location_name || $t('approvals.allLocations') }}</dd>
        </div>
        <div>
          <dt>{{ $t('approvals.deductionMode') }}</dt>
          <dd>
            {{
              item.batch_id
                ? $t('approvals.specifiedBatch', { n: item.batch_id })
                : $t('approvals.fifoAllocated')
            }}
          </dd>
        </div>
      </dl>
    </article>
    <p class="approval-fifo-note">{{ $t('approvals.fifoNote') }}</p>
  </section>
</template>
<script setup lang="ts">
import type { OutboundOrderResponse } from "../../api/outboundOrders";
import AuthenticatedImage from "../attributes/AuthenticatedImage.vue";
defineProps<{ order: OutboundOrderResponse }>();
</script>
