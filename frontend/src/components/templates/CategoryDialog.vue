<!-- 本组件拥有分类创建与编辑草稿和本地校验，不调用分类 API。 -->
<template>
  <ModalDialog
    :open="open"
    :title="category ? $t('templates.editCategoryTitle') : $t('templates.createCategoryTitle')"
    :description="
      category
        ? $t('templates.editCategoryDescription')
        : $t('templates.createCategoryDescription')
    "
    :busy="submitting"
    @close="requestClose"
  >
    <template v-if="category" #context>
      <div class="dialog-account-context">
        <span>{{ $t('templates.activeItemUsage') }}</span>
        <strong>{{ $t('templates.itemCount', { n: category.item_usage_count }) }}</strong>
      </div>
    </template>
    <form :id="formId" class="dialog-form" novalidate @submit.prevent="submit">
      <FormInput
        v-model="name"
        :label="$t('templates.categoryNameLabel')"
        validation-key="name"
        :error="errors.name"
        maxlength="128"
        autocomplete="off"
        autofocus
        required
        :disabled="submitting"
      />
      <FormTextarea
        v-model="description"
        :label="$t('templates.categoryDescriptionLabel')"
        validation-key="description"
        :error="errors.description"
        maxlength="1024"
        :rows="4"
        :disabled="submitting"
      />
      <FormInput
        v-model="sortOrder"
        :label="$t('templates.sortOrder')"
        validation-key="sort_order"
        :error="errors.sort_order"
        :hint="$t('templates.sortOrderHint')"
        type="number"
        min="0"
        step="1"
        :disabled="submitting"
      />
    </form>

    <template #actions>
      <button class="secondary-button" type="button" :disabled="submitting" @click="requestClose">
        {{ $t('common.cancel') }}
      </button>
      <button class="primary-button" type="submit" :form="formId" :disabled="submitting">
        {{
          submitting
            ? $t('templates.savingNow')
            : category
              ? $t('templates.saveCategory')
              : $t('templates.createCategory')
        }}
      </button>
    </template>
  </ModalDialog>

  <ModalDialog
    :open="discardOpen"
    :title="$t('templates.discardChangesTitle')"
    :description="$t('templates.discardCategoryChangesDescription')"
    compact
    nested
    @close="discardOpen = false"
  >
    <template #actions>
      <button class="secondary-button" type="button" @click="discardOpen = false">
        {{ $t('templates.keepEditing') }}
      </button>
      <button class="danger-button" type="button" @click="confirmClose">
        {{ $t('templates.discardChanges') }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { computed, ref, useId, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { ItemCategoryResponse, ItemCategoryWriteRequest } from "../../api/itemCategories";
import { useFormValidation } from "../../composables/useFormValidation";
import { notice } from "../../notices/notice";
import ModalDialog from "../ModalDialog.vue";
import FormInput from "../forms/FormInput.vue";
import FormTextarea from "../forms/FormTextarea.vue";

const props = defineProps<{
  open: boolean;
  category: ItemCategoryResponse | null;
  defaultSortOrder: number;
  submitting: boolean;
  errorMessage: string;
  fieldErrors: Record<string, string>;
}>();

const emit = defineEmits<{
  close: [];
  submit: [request: ItemCategoryWriteRequest];
}>();

const { t } = useI18n();
const formId = `category-form-${useId()}`;
const name = ref("");
const description = ref("");
const sortOrder = ref<number | null>(0);
const errors = ref<Record<string, string>>({});
const initialSnapshot = ref("");
const discardOpen = ref(false);
useFormValidation(errors);

const snapshot = computed(() => JSON.stringify([name.value, description.value, sortOrder.value]));

watch(
  () => props.open,
  (open) => {
    if (!open) return;
    name.value = props.category?.name ?? "";
    description.value = props.category?.description ?? "";
    sortOrder.value = props.category?.sort_order ?? props.defaultSortOrder;
    errors.value = { ...props.fieldErrors };
    discardOpen.value = false;
    initialSnapshot.value = JSON.stringify([name.value, description.value, sortOrder.value]);
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
  const normalizedDescription = description.value.trim();
  if (!normalizedName) nextErrors.name = t("templates.categoryNameRequired");
  else if (normalizedName.length > 128) nextErrors.name = t("templates.categoryNameTooLong");
  if (normalizedDescription.length > 1024)
    nextErrors.description = t("templates.categoryDescriptionTooLong");
  if (!Number.isInteger(sortOrder.value) || (sortOrder.value ?? -1) < 0)
    nextErrors.sort_order = t("templates.invalidSortOrder");
  errors.value = nextErrors;
  if (Object.keys(nextErrors).length) {
    notice.warning(t("templates.checkCategoryInfo"), { detail: Object.values(nextErrors)[0] });
    return;
  }
  emit("submit", {
    name: normalizedName,
    description: normalizedDescription || null,
    sort_order: sortOrder.value as number,
  });
}

function requestClose(): void {
  if (snapshot.value !== initialSnapshot.value) discardOpen.value = true;
  else emit("close");
}

function confirmClose(): void {
  discardOpen.value = false;
  emit("close");
}
</script>
