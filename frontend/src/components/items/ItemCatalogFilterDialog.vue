<!-- 本组件拥有物品目录高级筛选草稿；它不请求目录、不管理分页，也不修改已应用筛选。 -->
<template>
  <ModalDialog
    :open="open"
    title="高级筛选"
    description="组合分类、属性模板和物品属性筛选目录。"
    compact
    @close="emit('close')"
  >
    <form id="item-catalog-filter-form" class="item-catalog-filter" @submit.prevent="apply">
      <div class="item-catalog-filter__fixed-fields">
        <label>
          <span>分类</span>
          <SelectControl v-model="draft.categoryId" name="item_filter_category">
            <option :value="null">全部分类</option>
            <option v-for="category in categories" :key="category.id" :value="category.id">
              {{ category.name }}
            </option>
          </SelectControl>
        </label>
        <label>
          <span>属性模板</span>
          <SelectControl v-model="draft.attributeTemplateId" name="item_filter_template">
            <option :value="null">全部模板</option>
            <option v-for="template in templates" :key="template.id" :value="template.id">
              {{ template.name }}
            </option>
          </SelectControl>
        </label>
      </div>

      <div v-if="loading" class="item-catalog-filter__state" role="status">正在加载筛选值…</div>
      <div
        v-else-if="error"
        class="item-catalog-filter__state item-catalog-filter__state--error"
        role="alert"
      >
        <span>{{ error }}</span>
        <button class="text-button" type="button" @click="emit('retry')">重试</button>
      </div>
      <div v-else-if="visibleFields.length" class="item-catalog-filter__groups">
        <section v-for="field in visibleFields" :key="field.key" class="item-catalog-filter__group">
          <header>
            <strong>{{ field.label }}</strong>
            <span>{{
              selectedValues(field.key).length
                ? `已选 ${selectedValues(field.key).length}`
                : `${field.values.length} 项`
            }}</span>
          </header>
          <div class="item-catalog-filter__values">
            <label
              v-for="value in displayedValues(field)"
              :key="value.value"
              class="item-catalog-filter__value"
              :class="{
                'is-selected': selectedValues(field.key).includes(value.value),
                'is-unavailable': value.unavailable,
              }"
            >
              <input
                type="checkbox"
                :checked="selectedValues(field.key).includes(value.value)"
                @change="toggleValue(field.key, value.value)"
              />
              <span :title="displayValue(field, value.value)">{{
                displayValue(field, value.value)
              }}</span>
              <small>{{ value.unavailable ? "不可用" : value.count }}</small>
            </label>
          </div>
          <button
            v-if="mergedValues(field).length > COLLAPSED_VALUE_COUNT"
            class="text-button item-catalog-filter__expand"
            type="button"
            @click="toggleExpanded(field.key)"
          >
            {{
              expandedFields.includes(field.key)
                ? "收起"
                : `显示其余 ${mergedValues(field).length - COLLAPSED_VALUE_COUNT} 项`
            }}
          </button>
        </section>
      </div>
      <div v-else class="item-catalog-filter__state">当前条件下没有可用的高级筛选字段</div>
    </form>

    <template #actions>
      <button
        class="text-button item-catalog-filter__clear"
        type="button"
        :disabled="!hasDraftFilters"
        @click="clearDraft"
      >
        清除全部
      </button>
      <button class="secondary-button" type="button" @click="emit('close')">取消</button>
      <button class="primary-button" type="submit" form="item-catalog-filter-form">应用筛选</button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import type { ItemCategoryResponse } from "../../api/itemCategories";
import type { ItemAttributeTemplateResponse } from "../../api/itemAttributeTemplates";
import {
  cloneItemCatalogFilters,
  emptyItemCatalogFilters,
  type ItemCatalogFilters,
  type ItemFilterFieldResponse,
  type ItemFilterValueResponse,
} from "../../api/items";
import ModalDialog from "../ModalDialog.vue";
import SelectControl from "../forms/SelectControl.vue";

interface DisplayFilterValue extends ItemFilterValueResponse {
  unavailable?: boolean;
}

const COLLAPSED_VALUE_COUNT = 8;
const props = defineProps<{
  open: boolean;
  applied: ItemCatalogFilters;
  fields: ItemFilterFieldResponse[];
  categories: ItemCategoryResponse[];
  templates: ItemAttributeTemplateResponse[];
  loading: boolean;
  error: string;
}>();
const emit = defineEmits<{
  close: [];
  retry: [];
  apply: [filters: ItemCatalogFilters];
}>();

const draft = reactive<ItemCatalogFilters>(emptyItemCatalogFilters());
const expandedFields = ref<string[]>([]);
const visibleFields = computed(() => props.fields.filter((field) => field.value_type !== "file"));
const hasDraftFilters = computed(
  () =>
    draft.categoryId !== null ||
    draft.attributeTemplateId !== null ||
    Object.values(draft.fields).some((values) => values.length),
);

watch(
  () => props.open,
  (open) => {
    if (!open) return;
    assignDraft(cloneItemCatalogFilters(props.applied));
    expandedFields.value = [];
  },
);

function assignDraft(filters: ItemCatalogFilters): void {
  draft.categoryId = filters.categoryId;
  draft.attributeTemplateId = filters.attributeTemplateId;
  draft.fields = filters.fields;
}

function selectedValues(key: string): string[] {
  return draft.fields[key] ?? [];
}

function toggleValue(key: string, value: string): void {
  const selected = selectedValues(key);
  draft.fields[key] = selected.includes(value)
    ? selected.filter((candidate) => candidate !== value)
    : [...selected, value];
  if (!draft.fields[key]?.length) delete draft.fields[key];
}

function mergedValues(field: ItemFilterFieldResponse): DisplayFilterValue[] {
  const available = new Map(field.values.map((value) => [value.value, value]));
  return [
    ...selectedValues(field.key)
      .filter((value) => !available.has(value))
      .map((value) => ({ value, count: 0, unavailable: true })),
    ...field.values,
  ];
}

function displayedValues(field: ItemFilterFieldResponse): DisplayFilterValue[] {
  const values = mergedValues(field);
  return expandedFields.value.includes(field.key) ? values : values.slice(0, COLLAPSED_VALUE_COUNT);
}

function toggleExpanded(key: string): void {
  expandedFields.value = expandedFields.value.includes(key)
    ? expandedFields.value.filter((candidate) => candidate !== key)
    : [...expandedFields.value, key];
}

function displayValue(field: ItemFilterFieldResponse, value: string): string {
  if (field.value_type === "boolean")
    return value === "true" ? "是" : value === "false" ? "否" : value;
  return value;
}

function clearDraft(): void {
  assignDraft(emptyItemCatalogFilters());
}

function apply(): void {
  emit("apply", cloneItemCatalogFilters(draft));
}
</script>

<style scoped lang="scss">
.item-catalog-filter {
  display: grid;
  gap: 18px;
}
.item-catalog-filter__fixed-fields {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}
.item-catalog-filter__fixed-fields label {
  display: grid;
  min-width: 0;
  gap: 6px;
  color: var(--color-muted);
  font-size: 12px;
  font-weight: 650;
}
.item-catalog-filter__groups {
  display: grid;
  gap: 16px;
}
.item-catalog-filter__group {
  display: grid;
  gap: 8px;
  padding-top: 14px;
  border-top: 1px solid var(--color-border);
}
.item-catalog-filter__group > header {
  display: flex;
  min-width: 0;
  align-items: baseline;
  justify-content: space-between;
  gap: 12px;
}
.item-catalog-filter__group > header strong {
  overflow: hidden;
  font-size: 13px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.item-catalog-filter__group > header span {
  color: var(--color-subtle);
  font-size: 11px;
  white-space: nowrap;
}
.item-catalog-filter__values {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 6px;
}
.item-catalog-filter__value {
  display: grid;
  min-width: 0;
  min-height: 36px;
  grid-template-columns: 16px minmax(0, 1fr) auto;
  align-items: center;
  gap: 7px;
  padding: 7px 8px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text);
  cursor: pointer;
}
.item-catalog-filter__value.is-selected {
  border-color: color-mix(in srgb, var(--color-accent) 45%, var(--color-border));
  background: var(--color-accent-soft);
  color: var(--color-accent);
}
.item-catalog-filter__value.is-unavailable {
  border-style: dashed;
  color: var(--color-muted);
}
.item-catalog-filter__value input {
  width: 15px;
  height: 15px;
  margin: 0;
  accent-color: var(--color-accent);
}
.item-catalog-filter__value > span {
  overflow: hidden;
  font-size: 12px;
  font-weight: 620;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.item-catalog-filter__value small {
  color: var(--color-subtle);
  font-size: 11px;
  font-weight: 550;
}
.item-catalog-filter__expand {
  justify-self: start;
}
.item-catalog-filter__state {
  display: flex;
  min-height: 84px;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: var(--color-muted);
  font-size: 12px;
  text-align: center;
}
.item-catalog-filter__state--error {
  color: var(--color-danger);
}
.item-catalog-filter__clear {
  margin-right: auto;
}

@media (max-width: 767px) {
  .item-catalog-filter__fixed-fields,
  .item-catalog-filter__values {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
