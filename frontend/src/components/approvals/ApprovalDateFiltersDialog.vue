<!-- 本组件拥有审批队列创建时间筛选草稿；它不写 URL、不请求列表或改变审批状态。 -->
<template>
  <ModalDialog :open="open" title="筛选待审批单据" compact @close="emit('close')">
    <div class="approval-filter-fields">
      <DateTimeField v-model="draft.dateFrom" name="approval_date_from" label="创建时间起点" />
      <DateTimeField v-model="draft.dateTo" name="approval_date_to" label="创建时间终点" />
      <p v-if="error" class="form-warning" role="alert">{{ error }}</p>
    </div>
    <template #actions>
      <button class="text-button" type="button" @click="reset">重置</button>
      <button class="secondary-button" type="button" @click="emit('close')">取消</button>
      <button class="primary-button" type="button" @click="apply">应用筛选</button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { reactive, ref, watch } from "vue";
import ModalDialog from "../ModalDialog.vue";
import DateTimeField from "../forms/DateTimeField.vue";

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
    error.value = "创建时间起点不能晚于终点";
    return;
  }
  emit("apply", { ...draft });
}
</script>
