<!--
  本组件拥有合并草稿页中单条出库明细的数量与扣减方式编辑区；
  批次数据加载、成本估算与写回时机由出库装配层持有，本组件只渲染与派发。
-->
<template>
  <section class="outbound-allocation-section">
    <header>
      <strong>{{ $t('stockDraft.outboundQuantityThisTime') }}</strong
      ><span>{{ $t('stockDraft.quantityRevalidatedAtApproval') }}</span>
    </header>
    <label class="outbound-allocation-quantity">
      <span>{{ $t('stockDraft.quantityWithUnit', { unit: line.item.unit }) }} *</span>
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
        :aria-label="$t('stockDraft.outboundQuantityAria', { name: line.item.name })"
      />
      <span
        v-if="validation && !quantityValid"
        :id="`outbound-allocation-quantity-error-${line.lineId}`"
        class="visually-hidden"
        role="alert"
        >{{ $t('stockDraft.outboundQuantityPositive') }}</span
      >
    </label>
  </section>
  <section class="outbound-allocation-section">
    <header>
      <strong>{{ $t('stockDraft.deductionMode') }}</strong
      ><span>{{ $t('stockDraft.deductionModeHint') }}</span>
    </header>
    <fieldset class="outbound-allocation-editor">
      <label>
        <input v-model="draft.mode" type="radio" value="fifo" />
        <span class="outbound-radio-indicator" aria-hidden="true"></span>
        <span
          ><strong>{{ $t('stockDraft.fifoAllocation') }}</strong
          ><small>{{ $t('stockDraft.fifoAllocationHint') }}</small></span
        >
      </label>
      <label>
        <input v-model="draft.mode" type="radio" value="specific_batch" />
        <span class="outbound-radio-indicator" aria-hidden="true"></span>
        <span
          ><strong>{{ $t('stockDraft.specificBatch') }}</strong
          ><small>{{ $t('stockDraft.specificBatchHint') }}</small></span
        >
      </label>
    </fieldset>
  </section>
  <section v-if="draft.mode === 'fifo'" class="outbound-allocation-section">
    <header>
      <strong>{{ $t('stockDraft.deductionScope') }}</strong
      ><span>{{ $t('stockDraft.deductionScopeHint') }}</span>
    </header>
    <label class="outbound-location">
      <span>{{ $t('stockDraft.limitLocation') }}</span>
      <SelectControl v-model="draft.locationId" :aria-label="$t('stockDraft.limitLocation')" compact>
        <option :value="null">{{ $t('stockDraft.allLocations') }}</option>
        <option v-for="location in locations" :key="location.id" :value="location.id">
          {{ location.name }}
        </option>
      </SelectControl>
    </label>
  </section>
  <section v-else class="outbound-allocation-section">
    <header>
      <strong>{{ $t('stockDraft.selectBatch') }}</strong
      ><span>{{ $t('stockDraft.batchSnapshotHint') }}</span>
    </header>
    <div v-overlay-scrollbar class="outbound-batches" @scroll.passive="handleScroll">
      <div v-for="batch in batches" :key="batch.id" class="outbound-batch">
        <label>
          <input v-model="draft.batchId" type="radio" :value="batch.id" />
          <span class="outbound-radio-indicator" aria-hidden="true"></span>
          <span>
            <strong>{{ batch.batch_no }}</strong>
            <small>
              {{ batch.location_name }} · {{ $t('stockDraft.remaining') }}
              {{ batch.remaining_quantity }} {{ line.item.unit }}
              {{ batch.expires_at ? ` · ${$t('stockDraft.expiryWithDate', { date: batch.expires_at })}` : "" }}
              · {{ $t('stockDraft.costPerUnit', { cost: formatMoney(batch.unit_cost), unit: line.item.unit }) }}
            </small>
          </span>
        </label>
      </div>
      <p v-if="batchPending">{{ $t('stockDraft.loadingBatches') }}</p>
      <p v-else-if="batchMore">{{ $t('stockDraft.scrollToLoadMore') }}</p>
      <p v-else>{{ $t('stockDraft.allBatchesLoaded') }}</p>
    </div>
  </section>
  <p class="outbound-cost-hint">{{ costHint }}</p>
</template>

<script setup lang="ts">
import { computed, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { ItemBatchStockResponse } from "../../api/items";
import type { LocationResponse } from "../../api/locations";
import SelectControl from "../forms/SelectControl.vue";
import { formatMoney } from "../../pages/inbound-draft/presentation";
import type {
  OutboundAllocationDraft,
  OutboundDraftLine,
} from "../../pages/stock-draft/useOutboundDraft";
import { notice } from "../../notices/notice";

const { t } = useI18n();

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
      notice.error(t("stockDraft.loadBatchesFailed"), {
        detail: error,
        onClick: () => emit("retry-batches"),
      });
  },
);

watch(
  () => props.locationError,
  (error) => {
    if (error)
      notice.error(t("stockDraft.loadLocationsFailed"), {
        detail: t("stockDraft.fifoFallbackHint", { error }),
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
