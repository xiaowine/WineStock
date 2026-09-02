<!--
  本组件拥有一键批量创建物品的前置选项 Dialog：整批统一的属性模板、分类与计量单位。
  它不执行创建，也不拥有预览列表；确认后回传批次选项由调用方驱动批量会话。
-->
<template>
  <ModalDialog
    :open="open"
    :title="$t('items.batchCreateOptionsTitle')"
    :description="$t('items.batchCreateOptionsDescription', { n: count })"
    compact
    nested
    @close="emit('close')"
  >
    <div class="batch-create-options">
      <p v-if="metadataError" class="form-error" role="alert">{{ metadataError }}</p>
      <template v-else>
        <FormSelect v-model="templateId" :label="$t('items.attributeTemplate')" name="batch_create_template">
          <option :value="null">{{ $t('items.noTemplate') }}</option>
          <option v-for="template in templates" :key="template.id" :value="template.id">
            {{ template.name }}{{ template.is_default ? $t('items.defaultSuffix') : "" }}
          </option>
        </FormSelect>
        <p class="batch-create-options__hint">{{ $t('items.batchCreateOptionsHint') }}</p>
        <FormSelect v-model="categoryId" :label="$t('items.category')" name="batch_create_category">
          <option :value="null">{{ $t('items.unspecified') }}</option>
          <option v-for="category in categories" :key="category.id" :value="category.id">
            {{ category.name }}
          </option>
        </FormSelect>
        <FormInput v-model="unit" :label="$t('items.unit')" name="batch_create_unit" required />
      </template>
    </div>
    <template #actions>
      <button class="secondary-button" type="button" @click="emit('close')">
        {{ $t('common.cancel') }}
      </button>
      <button
        class="primary-button"
        type="button"
        :disabled="metadataError !== '' || !unit.trim()"
        @click="confirm"
      >
        {{ $t('items.startCreating') }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import type { ItemAttributeTemplateResponse } from "../../api/itemAttributeTemplates";
import type { ItemCategoryResponse } from "../../api/itemCategories";
import FormInput from "../forms/FormInput.vue";
import FormSelect from "../forms/FormSelect.vue";
import ModalDialog from "../ModalDialog.vue";
import type { BatchLcscCreationOptions } from "./useBatchLcscItemCreation";

const props = defineProps<{
  open: boolean;
  count: number;
  templates: ItemAttributeTemplateResponse[];
  categories: ItemCategoryResponse[];
  metadataError: string;
  /** 打开时的初始选项；同一会话内由调用方记住上次选择。 */
  initialOptions: BatchLcscCreationOptions;
}>();

const emit = defineEmits<{
  close: [];
  confirm: [options: BatchLcscCreationOptions];
}>();

const templateId = ref<number | null>(null);
const categoryId = ref<number | null>(null);
const unit = ref("\u4e2a"); // 个

watch(
  () => props.open,
  (open) => {
    if (!open) return;
    templateId.value = props.initialOptions.templateId;
    categoryId.value = props.initialOptions.categoryId;
    unit.value = props.initialOptions.unit;
  },
  { immediate: true },
);

function confirm(): void {
  emit("confirm", {
    templateId: templateId.value,
    categoryId: categoryId.value,
    unit: unit.value.trim() || "\u4e2a", // 个
  });
}
</script>

<style scoped lang="scss">
.batch-create-options {
  display: grid;
  gap: 12px;
}

.batch-create-options__hint {
  margin: -6px 0 0;
  color: var(--color-muted);
  font-size: 12px;
  line-height: 1.6;
}
</style>
