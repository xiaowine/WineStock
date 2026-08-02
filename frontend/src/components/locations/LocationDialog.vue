<!--
  本组件拥有库位创建和编辑草稿、本地校验与字段错误呈现。
  它不调用库位 API，也不判断库位是否仍被库存批次引用。
-->
<template>
  <ModalDialog
    :open="open"
    :title="location ? '编辑库位' : '新建库位'"
    :description="
      location
        ? '修改所属分组、唯一名称、备注和展示顺序。'
        : '库位会出现在入库明细的存放位置选择器中。'
    "
    :busy="submitting"
    @close="emit('close')"
  >
    <form :id="formId" class="dialog-form" novalidate @submit.prevent="submit">
      <FormSelect
        v-model="groupId"
        label="所属分组"
        validation-key="group_id"
        :error="errors.group_id"
        name="location_group_id"
        required
        :disabled="submitting"
      >
        <option :value="null" disabled>请选择分组</option>
        <option v-for="option in groupOptions" :key="option.id" :value="option.id">
          {{ option.label }}
        </option>
      </FormSelect>

      <FormInput
        v-model="name"
        label="库位名称"
        validation-key="name"
        :error="errors.name"
        name="location_name"
        maxlength="128"
        autocomplete="off"
        hint="未删除库位内全局唯一"
        autofocus
        required
        :disabled="submitting"
      />

      <FormTextarea
        v-model="notes"
        label="备注"
        validation-key="notes"
        :error="errors.notes"
        name="location_notes"
        maxlength="1024"
        :rows="3"
        :disabled="submitting"
      />

      <FormInput
        v-model="sortOrder"
        label="排序"
        validation-key="sort_order"
        :error="errors.sort_order"
        hint="数值越小越靠前"
        name="location_sort_order"
        type="number"
        step="1"
        :disabled="submitting"
      />
    </form>

    <template #actions>
      <button class="secondary-button" type="button" :disabled="submitting" @click="emit('close')">
        取消
      </button>
      <button class="primary-button" type="submit" :form="formId" :disabled="submitting">
        {{ submitting ? "正在保存…" : location ? "保存库位" : "创建库位" }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { ref, useId, watch } from "vue";
import type { LocationResponse, LocationUpdateRequest } from "../../api/locations";
import { useFormValidation } from "../../composables/useFormValidation";
import { notice } from "../../notices/notice";
import ModalDialog from "../ModalDialog.vue";
import FormInput from "../forms/FormInput.vue";
import FormSelect from "../forms/FormSelect.vue";
import FormTextarea from "../forms/FormTextarea.vue";
import type { LocationGroupOption } from "./types";

const props = defineProps<{
  open: boolean;
  location: LocationResponse | null;
  defaultGroupId: number | null;
  groupOptions: LocationGroupOption[];
  submitting: boolean;
  errorMessage: string;
  fieldErrors: Record<string, string>;
}>();

const emit = defineEmits<{
  close: [];
  submit: [request: LocationUpdateRequest];
}>();

const formId = `location-form-${useId()}`;
const groupId = ref<number | null>(null);
const name = ref("");
const notes = ref("");
const sortOrder = ref<number | null>(0);
const errors = ref<Record<string, string>>({});
useFormValidation(errors);

watch(
  () => props.open,
  (open) => {
    if (!open) return;
    groupId.value = props.location?.group_id ?? props.defaultGroupId;
    name.value = props.location?.name ?? "";
    notes.value = props.location?.notes ?? "";
    sortOrder.value = props.location?.sort_order ?? 0;
    errors.value = { ...props.fieldErrors };
  },
);

watch(
  () => props.fieldErrors,
  (fieldErrors) => {
    if (props.open) errors.value = { ...fieldErrors };
  },
  { deep: true },
);

function submit(): void {
  const nextErrors: Record<string, string> = {};
  const normalizedName = name.value.trim();
  if (!groupId.value) nextErrors.group_id = "请选择所属分组";
  if (!normalizedName) nextErrors.name = "请输入库位名称";
  if (!Number.isInteger(sortOrder.value ?? 0)) nextErrors.sort_order = "排序必须是整数";
  errors.value = nextErrors;
  if (Object.keys(nextErrors).length > 0) {
    notice.warning("请检查库位信息", { detail: Object.values(nextErrors)[0] });
    return;
  }
  emit("submit", {
    group_id: groupId.value as number,
    name: normalizedName,
    notes: notes.value.trim() || null,
    sort_order: sortOrder.value ?? 0,
  });
}
</script>
