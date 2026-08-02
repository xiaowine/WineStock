<!--
  本组件拥有合并草稿页中单条出库明细的数量与扣减方式编辑区；
  批次数据加载、成本估算与写回时机由出库装配层持有，本组件只渲染与派发。
-->
<template>
  <section class="outbound-allocation-section">
    <header><strong>本次出库数量</strong><span>审批时仍会按实际库存重新校验。</span></header>
    <label class="outbound-allocation-quantity">
      <span>数量（{{ line.item.unit }}） *</span>
      <input
        v-model="draft.quantity"
        data-outbound-allocation-quantity
        inputmode="decimal"
        type="number"
        min="0.01"
        step="0.01"
        :class="{ error: validation && !quantityValid }"
        :aria-invalid="validation && !quantityValid ? true : undefined"
        :aria-describedby="
          validation && !quantityValid
            ? `outbound-allocation-quantity-error-${line.lineId}`
            : undefined
        "
        :aria-label="`${line.item.name} 出库数量`"
      />
      <span
        v-if="validation && !quantityValid"
        :id="`outbound-allocation-quantity-error-${line.lineId}`"
        class="visually-hidden"
        role="alert"
        >请输入大于 0 的出库数量</span
      >
    </label>
  </section>
  <section class="outbound-allocation-section">
    <header><strong>扣减方式</strong><span>选择实际出库时扣减库存的规则。</span></header>
    <fieldset class="outbound-allocation-editor">
      <label>
        <input v-model="draft.mode" type="radio" value="fifo" />
        <span class="outbound-radio-indicator" aria-hidden="true"></span>
        <span
          ><strong>按先进先出分配</strong><small>从指定库位或全部库存按 FIFO 扣减。</small></span
        >
      </label>
      <label>
        <input v-model="draft.mode" type="radio" value="specific_batch" />
        <span class="outbound-radio-indicator" aria-hidden="true"></span>
        <span><strong>指定批次</strong><small>从选定批次扣减，库位随批次确定。</small></span>
      </label>
    </fieldset>
  </section>
  <section v-if="draft.mode === 'fifo'" class="outbound-allocation-section">
    <header><strong>扣减范围</strong><span>不限制时，审批可从全部库位按 FIFO 分配。</span></header>
    <label class="outbound-location">
      <span>限制库位（可选）</span>
      <SelectControl v-model="draft.locationId" aria-label="限制库位" compact>
        <option :value="null">全部库位</option>
        <option v-for="location in locations" :key="location.id" :value="location.id">
          {{ location.name }}
        </option>
      </SelectControl>
    </label>
  </section>
  <section v-else class="outbound-allocation-section">
    <header>
      <strong>选择批次</strong><span>批次可用数量仅为当前快照，实际出库时仍会校验库存。</span>
    </header>
    <div class="outbound-batches" @scroll.passive="handleScroll">
      <div v-for="batch in batches" :key="batch.id" class="outbound-batch">
        <label>
          <input v-model="draft.batchId" type="radio" :value="batch.id" />
          <span class="outbound-radio-indicator" aria-hidden="true"></span>
          <span>
            <strong>{{ batch.batch_no }}</strong>
            <small>
              {{ batch.location_name }} · 剩余 {{ batch.remaining_quantity }} {{ line.item.unit }}
              {{ batch.expires_at ? ` · 有效期 ${batch.expires_at}` : "" }} · 成本 ¥{{
                formatMoney(batch.unit_cost)
              }}
              / {{ line.item.unit }}
            </small>
          </span>
        </label>
      </div>
      <p v-if="batchPending">正在加载批次…</p>
      <p v-else-if="batchMore">继续向下滚动加载</p>
      <p v-else>已加载全部批次</p>
    </div>
  </section>
  <p class="outbound-cost-hint">{{ costHint }}</p>
</template>

<script setup lang="ts">
import { computed, watch } from "vue";
import type { ItemBatchStockResponse } from "../../api/items";
import type { LocationResponse } from "../../api/locations";
import SelectControl from "../forms/SelectControl.vue";
import { formatMoney } from "../../pages/inbound-draft/presentation";
import type {
  OutboundAllocationDraft,
  OutboundDraftLine,
} from "../../pages/stock-draft/useOutboundDraft";
import { notice } from "../../notices/notice";

const props = defineProps<{
  line: OutboundDraftLine;
  draft: OutboundAllocationDraft;
  batches: ItemBatchStockResponse[];
  batchError: string;
  batchPending: boolean;
  batchMore: boolean;
  locations: LocationResponse[];
  locationError: string;
  validation: boolean;
  costHint: string;
}>();

const emit = defineEmits<{
  "retry-batches": [];
  "load-more-batches": [];
}>();

watch(
  () => props.batchError,
  (error) => {
    if (error)
      notice.error("加载批次失败", {
        detail: error,
        onClick: () => emit("retry-batches"),
      });
  },
);

watch(
  () => props.locationError,
  (error) => {
    if (error)
      notice.error("加载库位失败", {
        detail: `${error}，仍可按全部库位 FIFO 分配。`,
      });
  },
);

const quantityValid = computed(() => {
  const quantity = Number(props.draft.quantity);
  return Number.isFinite(quantity) && quantity > 0;
});

function handleScroll(event: Event): void {
  const element = event.currentTarget as HTMLElement;
  if (element.scrollHeight - element.scrollTop - element.clientHeight < 100)
    emit("load-more-batches");
}
</script>
