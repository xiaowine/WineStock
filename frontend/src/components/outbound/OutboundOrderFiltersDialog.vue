<template>
  <ModalDialog
    :open="open"
    title="筛选出库单"
    description="按处理状态和创建时间缩小出库单范围。"
    @close="emit('close')"
    ><form id="outbound-filter" class="outbound-filter" novalidate @submit.prevent="submit">
      <FormSelect v-model="status" label="处理状态" validation-key="status"
        ><option value="">全部状态</option>
        <option value="pending">待审批</option>
        <option value="approved">已出库</option>
        <option value="rejected">已拒绝</option></FormSelect
      ><DateTimeField
        v-model="dateFrom"
        label="开始时间"
        validation-key="dateRange"
        :error="errors.dateRange"
      /><DateTimeField
        v-model="dateTo"
        label="结束时间"
        validation-key="dateRange"
        :error="errors.dateRange"
      />
    </form>
    <template #actions
      ><button class="text-button" type="button" @click="reset">重置</button
      ><button class="secondary-button" type="button" @click="emit('close')">取消</button
      ><button class="primary-button" type="submit" form="outbound-filter">
        应用筛选
      </button></template
    ></ModalDialog
  >
</template>
<script setup lang="ts">
import { ref, watch } from "vue";
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
      ? "开始时间不能晚于结束时间"
      : "";
  errors.value = error ? { dateRange: error } : {};
  if (error) {
    notice.warning("请检查筛选条件", { detail: error });
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
