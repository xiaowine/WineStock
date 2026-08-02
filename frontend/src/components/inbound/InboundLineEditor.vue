<!-- 本组件拥有单条入库明细的完整编辑区；它不选择物品、不提交整张入库单，也不管理路由。 -->
<template>
  <div class="inbound-line-editor">
    <section class="inbound-line-editor__operation-fields" aria-label="本次入库参数">
      <label>
        <span>数量 *</span>
        <input
          v-model.number="line.quantity"
          :name="'quantity_' + line.lineId"
          :data-line-id="line.lineId"
          data-field="quantity"
          :class="{
            'inbound-control--error': validationAttempted && !validQuantity(line.quantity),
          }"
          :aria-invalid="validationAttempted && !validQuantity(line.quantity) ? true : undefined"
          :aria-describedby="
            validationAttempted && !validQuantity(line.quantity)
              ? `inbound-${line.lineId}-quantity-error`
              : undefined
          "
          type="number"
          min="0.01"
          step="0.01"
          inputmode="decimal"
          :aria-label="line.item.name + ' 入库数量'"
          autofocus
        />
        <span
          v-if="validationAttempted && !validQuantity(line.quantity)"
          :id="`inbound-${line.lineId}-quantity-error`"
          class="visually-hidden"
          role="alert"
          >请输入大于 0 的入库数量</span
        >
      </label>
      <label>
        <span>单价 *</span>
        <input
          v-model.number="line.unitPrice"
          :name="'unit_price_' + line.lineId"
          :data-line-id="line.lineId"
          data-field="unitPrice"
          :class="{
            'inbound-control--error': validationAttempted && !validUnitPrice(line.unitPrice),
          }"
          :aria-invalid="validationAttempted && !validUnitPrice(line.unitPrice) ? true : undefined"
          :aria-describedby="
            validationAttempted && !validUnitPrice(line.unitPrice)
              ? `inbound-${line.lineId}-unit-price-error`
              : undefined
          "
          type="number"
          min="0"
          step="0.01"
          inputmode="decimal"
          :aria-label="line.item.name + ' 入库单价'"
        />
        <span
          v-if="validationAttempted && !validUnitPrice(line.unitPrice)"
          :id="`inbound-${line.lineId}-unit-price-error`"
          class="visually-hidden"
          role="alert"
          >请输入不小于 0 的入库单价</span
        >
      </label>
      <label>
        <span>入库库位 *</span>
        <SelectControl
          v-model="line.locationId"
          :name="'location_' + line.lineId"
          :data-line-id="line.lineId"
          data-field="locationId"
          :aria-invalid="validationAttempted && line.locationId === null ? true : undefined"
          :aria-label="line.item.name + ' 入库库位'"
          compact
        >
          <option :value="null">请选择</option>
          <optgroup v-for="group in locationGroups" :key="group.name" :label="group.name">
            <option v-for="location in group.locations" :key="location.id" :value="location.id">
              {{ location.name }}
            </option>
          </optgroup>
        </SelectControl>
      </label>
    </section>

    <div class="inbound-line-editor__base-fields">
      <label>
        <span>批次号</span>
        <input
          v-model="line.batchNo"
          :name="'batch_no_' + line.lineId"
          type="text"
          maxlength="128"
          placeholder="留空后由服务端生成"
        />
      </label>
      <label>
        <span>有效期</span>
        <input v-model="line.expiresAt" :name="'expires_at_' + line.lineId" type="date" />
      </label>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, watch } from "vue";
import type { LocationResponse } from "../../api/locations";
import {
  validQuantity,
  validUnitPrice,
  type InboundDraftLine,
} from "../../pages/inbound-draft/model";
import SelectControl from "../forms/SelectControl.vue";
import { notice } from "../../notices/notice";

const props = defineProps<{
  line: InboundDraftLine;
  locations: LocationResponse[];
  locationError: string;
  validationAttempted: boolean;
}>();

const emit = defineEmits<{
  "retry-locations": [];
}>();

watch(
  () => props.locationError,
  (error) => {
    if (error)
      notice.error("加载库位失败", {
        detail: error,
        onClick: () => emit("retry-locations"),
      });
  },
);

const locationGroups = computed(() => {
  const groups = new Map<string, LocationResponse[]>();
  for (const location of props.locations) {
    const list = groups.get(location.group_name) ?? [];
    list.push(location);
    groups.set(location.group_name, list);
  }
  return Array.from(groups, ([name, locations]) => ({ name, locations }));
});
</script>
