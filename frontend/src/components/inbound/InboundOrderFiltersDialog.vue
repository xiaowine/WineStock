<!-- 本组件拥有入库单筛选草稿和日期范围校验；它不请求接口或修改路由。 -->
<template>
  <ModalDialog
    :open="open"
    :title="$t('orders.filterInbound')"
    :description="$t('orders.filterInboundDescription')"
    @close="emit('close')"
  >
    <form
      id="inbound-order-filter-form"
      class="inbound-order-filter-form"
      novalidate
      @submit.prevent="submit"
    >
      <FormSelect v-model="status" :label="$t('orders.processingStatus')" validation-key="status">
        <option value="">{{ $t("orders.allStatuses") }}</option>
        <option value="pending">{{ $t("orders.statusPending") }}</option>
        <option value="approved">{{ $t("orders.statusApprovedInbound") }}</option>
        <option value="rejected">{{ $t("orders.statusRejected") }}</option>
      </FormSelect>
      <DateTimeField
        v-model="dateFrom"
        :label="$t('orders.startTime')"
        validation-key="dateRange"
        :error="errors.dateRange"
      />
      <DateTimeField
        v-model="dateTo"
        :label="$t('orders.endTime')"
        validation-key="dateRange"
        :error="errors.dateRange"
      />
    </form>
    <template #actions
      ><button class="text-button inbound-order-filter-form__reset" type="button" @click="reset">
        {{ $t("orders.reset") }}</button
      ><button class="secondary-button" type="button" @click="emit('close')">
        {{ $t("orders.cancel") }}</button
      ><button class="primary-button" type="submit" form="inbound-order-filter-form">
        {{ $t("orders.applyFilter") }}
      </button></template
    >
  </ModalDialog>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { InboundOrderStatus } from "../../api/inboundOrders";
import DateTimeField from "../forms/DateTimeField.vue";
import FormSelect from "../forms/FormSelect.vue";
import ModalDialog from "../ModalDialog.vue";
import { useFormValidation } from "../../composables/useFormValidation";
import { notice } from "../../notices/notice";

export interface InboundOrderFilterValue {
  status: InboundOrderStatus | "";
  dateFrom: string;
  dateTo: string;
}
const props = defineProps<{ open: boolean; value: InboundOrderFilterValue }>();
const emit = defineEmits<{ close: []; apply: [value: InboundOrderFilterValue] }>();
const { t } = useI18n();
const status = ref<InboundOrderStatus | "">("");
const dateFrom = ref("");
const dateTo = ref("");
const errors = ref<Record<string, string>>({});
const { clearErrors } = useFormValidation(errors);
watch(
  () => props.open,
  (open) => {
    if (!open) return;
    status.value = props.value.status;
    dateFrom.value = props.value.dateFrom;
    dateTo.value = props.value.dateTo;
    clearErrors();
  },
  { immediate: true },
);
function reset(): void {
  status.value = "";
  dateFrom.value = "";
  dateTo.value = "";
  clearErrors();
}
function submit(): void {
  const dateRangeError =
    dateFrom.value && dateTo.value && dateFrom.value > dateTo.value
      ? t("orders.startAfterEndDate")
      : "";
  errors.value = dateRangeError ? { dateRange: dateRangeError } : {};
  if (dateRangeError) {
    notice.warning(t("orders.checkFilterConditions"), { detail: dateRangeError });
    return;
  }
  if (!dateRangeError)
    emit("apply", { status: status.value, dateFrom: dateFrom.value, dateTo: dateTo.value });
}
</script>

<style lang="scss" src="./InboundOrderFiltersDialog.scss"></style>
