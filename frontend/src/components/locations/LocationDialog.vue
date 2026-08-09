<!--
  本组件拥有库位创建和编辑草稿、本地校验与字段错误呈现。
  它不调用库位 API，也不判断库位是否仍被库存批次引用。
-->
<template>
  <ModalDialog
    :open="open"
    :title="location ? $t('locations.editLocation') : $t('locations.createLocation')"
    :description="
      location
        ? $t('locations.editLocationDescription')
        : $t('locations.newLocationDescription')
    "
    :busy="submitting"
    @close="emit('close')"
  >
    <form :id="formId" class="dialog-form" novalidate @submit.prevent="submit">
      <FormSelect
        v-model="groupId"
        :label="$t('locations.groupField')"
        validation-key="group_id"
        :error="errors.group_id"
        name="location_group_id"
        required
        :disabled="submitting"
      >
        <option :value="null" disabled>{{ $t('locations.selectGroupPlaceholder') }}</option>
        <option v-for="option in groupOptions" :key="option.id" :value="option.id">
          {{ option.label }}
        </option>
      </FormSelect>

      <FormInput
        v-model="name"
        :label="$t('locations.locationNameField')"
        validation-key="name"
        :error="errors.name"
        name="location_name"
        maxlength="128"
        autocomplete="off"
        :hint="$t('locations.nameUniqueHint')"
        autofocus
        required
        :disabled="submitting"
      />

      <FormTextarea
        v-model="notes"
        :label="$t('locations.remark')"
        validation-key="notes"
        :error="errors.notes"
        name="location_notes"
        maxlength="1024"
        :rows="3"
        :disabled="submitting"
      />

      <FormInput
        v-model="sortOrder"
        :label="$t('locations.sortField')"
        validation-key="sort_order"
        :error="errors.sort_order"
        :hint="$t('locations.sortAscendingHint')"
        name="location_sort_order"
        type="number"
        step="1"
        :disabled="submitting"
      />
    </form>

    <template #actions>
      <button class="secondary-button" type="button" :disabled="submitting" @click="emit('close')">
        {{ $t('locations.cancel') }}
      </button>
      <button class="primary-button" type="submit" :form="formId" :disabled="submitting">
        {{ submitting ? $t('locations.saving') : location ? $t('locations.saveLocation') : $t('locations.createLocationSubmit') }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { ref, useId, watch } from "vue";
import { useI18n } from "vue-i18n";
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
const { t } = useI18n();
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
  if (!groupId.value) nextErrors.group_id = t("locations.selectGroupError");
  if (!normalizedName) nextErrors.name = t("locations.enterLocationName");
  if (!Number.isInteger(sortOrder.value ?? 0)) nextErrors.sort_order = t("locations.sortMustBeInteger");
  errors.value = nextErrors;
  if (Object.keys(nextErrors).length > 0) {
    notice.warning(t("locations.checkLocationInfo"), { detail: Object.values(nextErrors)[0] });
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
