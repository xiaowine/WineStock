<!-- 本组件拥有审批队列创建时间筛选草稿；它不写 URL、不请求列表或改变审批状态。 -->
<template>
  <ModalDialog :open="open" :title="$t('approvals.filterTitle')" compact @close="emit('close')">
    <div class="approval-filter-fields">
      <DateTimeField v-model="draft.dateFrom" name="approval_date_from" :label="$t('approvals.dateFromLabel')" />
      <DateTimeField v-model="draft.dateTo" name="approval_date_to" :label="$t('approvals.dateToLabel')" />
    </div>
    <template #actions>
      <button class="text-button" type="button" @click="reset">{{ $t('common.reset') }}</button>
      <button class="secondary-button" type="button" @click="emit('close')">{{ $t('common.cancel') }}</button>
      <button class="primary-button" type="button" @click="apply">{{ $t('approvals.applyFilters') }}</button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import ModalDialog from "../ModalDialog.vue";
import DateTimeField from "../forms/DateTimeField.vue";
import { notice } from "../../notices/notice";

/** 审批队列创建时间筛选值。 */
export interface ApprovalDateFilterValue {
  dateFrom: string;
  dateTo: string;
}

const props = defineProps<{ open: boolean; value: ApprovalDateFilterValue }>();
const emit = defineEmits<{
  close: [];
  apply: [value: ApprovalDateFilterValue];
}>();
const draft = reactive<ApprovalDateFilterValue>({ dateFrom: "", dateTo: "" });
const error = ref("");
const { t } = useI18n();

watch(
  () => props.open,
  (open) => {
    if (!open) return;
    draft.dateFrom = props.value.dateFrom;
    draft.dateTo = props.value.dateTo;
    error.value = "";
  },
);

function reset(): void {
  draft.dateFrom = "";
  draft.dateTo = "";
  error.value = "";
}
function apply(): void {
  if (draft.dateFrom && draft.dateTo && draft.dateFrom > draft.dateTo) {
    error.value = t("approvals.dateFromAfterTo");
    notice.warning(t("approvals.checkFilterConditions"), { detail: error.value });
    return;
  }
  emit("apply", { ...draft });
}
</script>
