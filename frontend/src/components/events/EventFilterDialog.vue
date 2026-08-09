<!-- 本组件拥有操作日志高级筛选草稿和本地校验；它不请求事件 API。 -->
<template>
  <ModalDialog
    :open="open"
    :title="$t('events.filterTitle')"
    :description="$t('events.filterDescription')"
    @close="emit('close')"
  >
    <form id="event-filter-form" class="event-filter-form" novalidate @submit.prevent="submit">
      <FormInput
        v-model="entityId"
        :label="$t('events.entityIdLabel')"
        type="number"
        min="1"
        step="1"
        :placeholder="$t('events.exampleId', { n: 42 })"
        :error="errors.entityId"
        validation-key="entityId"
      />
      <FormInput
        v-model="userId"
        :label="$t('events.userIdLabel')"
        type="number"
        min="1"
        step="1"
        :placeholder="$t('events.exampleId', { n: 1 })"
        :error="errors.userId"
        validation-key="userId"
      />
      <DateTimeField
        v-model="dateFrom"
        :label="$t('events.dateFromLabel')"
        validation-key="dateRange"
        :error="errors.dateRange"
      />
      <DateTimeField
        v-model="dateTo"
        :label="$t('events.dateToLabel')"
        validation-key="dateRange"
        :error="errors.dateRange"
      />
      <FormInput
        v-model="customEntityType"
        :label="$t('events.customEntityTypeLabel')"
        maxlength="64"
        :placeholder="$t('events.customValuePlaceholder', { value: 'custom_event' })"
        :hint="$t('events.hintEntityType')"
        :error="errors.customEntityType"
        validation-key="customEntityType"
      />
      <FormInput
        v-model="customAction"
        :label="$t('events.customActionLabel')"
        maxlength="64"
        :placeholder="$t('events.customValuePlaceholder', { value: 'archived' })"
        :hint="$t('events.hintAction')"
        :error="errors.customAction"
        validation-key="customAction"
      />
      <label class="event-filter-form__page-size">
        <span>{{ $t('events.pageSizeLabel') }}</span>
        <SelectControl v-model="pageSize" name="event_page_size">
          <option :value="25">{{ $t('events.pageSizeOption', { n: 25 }) }}</option>
          <option :value="50">{{ $t('events.pageSizeOption', { n: 50 }) }}</option>
          <option :value="100">{{ $t('events.pageSizeOption', { n: 100 }) }}</option>
          <option :value="200">{{ $t('events.pageSizeOption', { n: 200 }) }}</option>
        </SelectControl>
      </label>
    </form>

    <template #actions>
      <button class="text-button event-filter-form__reset" type="button" @click="reset">
        {{ $t('common.reset') }}
      </button>
      <button class="secondary-button" type="button" @click="emit('close')">
        {{ $t('common.cancel') }}
      </button>
      <button class="primary-button" type="submit" form="event-filter-form">
        {{ $t('events.applyFilters') }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import DateTimeField from "../forms/DateTimeField.vue";
import FormInput from "../forms/FormInput.vue";
import SelectControl from "../forms/SelectControl.vue";
import ModalDialog from "../ModalDialog.vue";
import { useFormValidation } from "../../composables/useFormValidation";
import { notice } from "../../notices/notice";

export interface EventAdvancedFilterValue {
  entityId: number | null;
  userId: number | null;
  customEntityType: string;
  customAction: string;
  dateFrom: string;
  dateTo: string;
  pageSize: number;
}

const props = defineProps<{
  open: boolean;
  value: EventAdvancedFilterValue;
}>();

const emit = defineEmits<{
  close: [];
  apply: [value: EventAdvancedFilterValue];
}>();

const entityId = ref<number | null>(null);
const userId = ref<number | null>(null);
const customEntityType = ref("");
const customAction = ref("");
const dateFrom = ref("");
const dateTo = ref("");
const pageSize = ref(50);
const errors = ref<Record<string, string>>({});
const { t } = useI18n();
const { clearErrors } = useFormValidation(errors);

watch(
  () => props.open,
  (open) => {
    if (!open) return;
    entityId.value = props.value.entityId;
    userId.value = props.value.userId;
    customEntityType.value = props.value.customEntityType;
    customAction.value = props.value.customAction;
    dateFrom.value = props.value.dateFrom;
    dateTo.value = props.value.dateTo;
    pageSize.value = props.value.pageSize;
    clearErrors();
  },
  { immediate: true },
);

function reset(): void {
  entityId.value = null;
  userId.value = null;
  customEntityType.value = "";
  customAction.value = "";
  dateFrom.value = "";
  dateTo.value = "";
  pageSize.value = 50;
  clearErrors();
}

function submit(): void {
  const nextErrors: Record<string, string> = {};
  if (!validPositiveInteger(entityId.value)) nextErrors.entityId = t("events.errorEntityIdPositive");
  if (!validPositiveInteger(userId.value)) nextErrors.userId = t("events.errorUserIdPositive");
  if (customEntityType.value && !customEntityType.value.trim())
    nextErrors.customEntityType = t("events.errorEntityTypeBlank");
  if (customAction.value && !customAction.value.trim())
    nextErrors.customAction = t("events.errorActionBlank");
  if (
    dateFrom.value &&
    dateTo.value &&
    new Date(dateFrom.value).getTime() > new Date(dateTo.value).getTime()
  ) {
    nextErrors.dateRange = t("events.errorDateRangeInverted");
  }
  errors.value = nextErrors;
  if (Object.keys(nextErrors).length > 0) {
    notice.warning(t("events.filterInvalidNotice"), { detail: Object.values(nextErrors)[0] });
    return;
  }
  emit("apply", {
    entityId: entityId.value,
    userId: userId.value,
    customEntityType: customEntityType.value.trim(),
    customAction: customAction.value.trim(),
    dateFrom: dateFrom.value,
    dateTo: dateTo.value,
    pageSize: pageSize.value,
  });
}

function validPositiveInteger(value: number | null): boolean {
  return value === null || (Number.isInteger(value) && value > 0);
}
</script>

<style lang="scss" src="./EventFilterDialog.scss"></style>
