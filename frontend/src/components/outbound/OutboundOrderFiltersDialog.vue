<template>
  <ModalDialog
    :open="open"
    :title="$t('orders.filterOutbound')"
    :description="$t('orders.filterOutboundDescription')"
    @close="emit('close')"
    ><form id="outbound-filter" class="outbound-filter" novalidate @submit.prevent="submit">
      <FormSelect v-model="status" :label="$t('orders.processingStatus')" validation-key="status"
        ><option value="">{{ $t("orders.allStatuses") }}</option>
        <option value="pending">{{ $t("orders.statusPending") }}</option>
        <option value="approved">{{ $t("orders.statusApprovedOutbound") }}</option>
        <option value="rejected">{{ $t("orders.statusRejected") }}</option></FormSelect
      ><DateTimeField
        v-model="dateFrom"
        :label="$t('orders.startTime')"
        validation-key="dateRange"
        :error="errors.dateRange"
      /><DateTimeField
        v-model="dateTo"
        :label="$t('orders.endTime')"
        validation-key="dateRange"
        :error="errors.dateRange"
      />
    </form>
    <template #actions
      ><button class="text-button" type="button" @click="reset">{{ $t("orders.reset") }}</button
      ><button class="secondary-button" type="button" @click="emit('close')">
        {{ $t("orders.cancel") }}</button
      ><button class="primary-button" type="submit" form="outbound-filter">
        {{ $t("orders.applyFilter") }}
      </button></template
    ></ModalDialog
  >
</template>
<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { OutboundOrderStatus } from "../../api/outboundOrders";
import DateTimeField from "../forms/DateTimeField.vue";
import FormSelect from "../forms/FormSelect.vue";
import ModalDialog from "../ModalDialog.vue";
import { useFormValidation } from "../../composables/useFormValidation";
import { notice } from "../../notices/notice";
export interface OutboundOrderFilterValue {
  status: OutboundOrderStatus | "";
  dateFrom: string;
  dateTo: string;
}
const props = defineProps<{ open: boolean; value: OutboundOrderFilterValue }>();
const emit = defineEmits<{ close: []; apply: [value: OutboundOrderFilterValue] }>();
const { t } = useI18n();
const status = ref<OutboundOrderStatus | "">(""),
  dateFrom = ref(""),
  dateTo = ref("");
const errors = ref<Record<string, string>>({});
const { clearErrors } = useFormValidation(errors);
watch(
  () => props.open,
  (o) => {
    if (o) {
      status.value = props.value.status;
      dateFrom.value = props.value.dateFrom;
      dateTo.value = props.value.dateTo;
      clearErrors();
    }
  },
  { immediate: true },
);
function reset() {
  status.value = "";
  dateFrom.value = "";
  dateTo.value = "";
  clearErrors();
}
function submit() {
  const error =
    dateFrom.value && dateTo.value && dateFrom.value > dateTo.value
      ? t("orders.startAfterEndTime")
      : "";
  errors.value = error ? { dateRange: error } : {};
  if (error) {
    notice.warning(t("orders.checkFilterConditions"), { detail: error });
    return;
  }
  emit("apply", { status: status.value, dateFrom: dateFrom.value, dateTo: dateTo.value });
}
</script>
<style scoped>
.outbound-filter {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 14px;
}
.outbound-filter > :first-child {
  grid-column: 1/-1;
}
@media (max-width: 640px) {
  .outbound-filter {
    grid-template-columns: 1fr;
  }
}
</style>
