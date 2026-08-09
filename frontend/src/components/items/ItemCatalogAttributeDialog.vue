<!-- 本组件拥有物品模板列表展示字段的选择会话；它不编辑模板字段结构或物品属性值。 -->
<template>
  <ModalDialog
    :open="open"
    :busy="saving"
    :title="$t('items.displaySettingsTitle')"
    :description="$t('items.displaySettingsDescription')"
    @close="emit('close')"
  >
    <template v-if="activeTemplate" #context>
      <div class="catalog-attribute-dialog__context">
        <div>
          <span>{{ $t('items.currentTemplate') }}</span>
          <strong>{{ activeTemplate.name }}</strong>
        </div>
        <div class="catalog-attribute-dialog__count">
          <span>{{ $t('items.selectedDisplayFields') }}</span>
          <strong>{{ selectedCount }} / 3</strong>
        </div>
      </div>
    </template>

    <form
      id="item-catalog-attribute-form"
      class="catalog-attribute-dialog"
      novalidate
      @submit.prevent="save"
    >
      <label class="catalog-attribute-dialog__template">
        <span>{{ $t('items.itemAttributeTemplate') }}</span>
        <SelectControl
          v-model="selectedTemplateId"
          name="catalog_attribute_template"
          :disabled="saving"
        >
          <option v-for="template in templates" :key="template.id" :value="template.id">
            {{ template.name }}
          </option>
        </SelectControl>
      </label>

      <Transition name="catalog-attribute-panel" mode="out-in">
        <div v-if="activeTemplate" :key="activeTemplate.id" class="catalog-attribute-dialog__panel">
          <div class="catalog-attribute-dialog__panel-header">
            <strong>{{ $t('items.availableDisplayFields') }}</strong>
            <span>{{ $t('items.itemCount', { n: activeTemplate.fields.length }) }}</span>
          </div>
          <div v-overlay-scrollbar class="catalog-attribute-dialog__options">
            <label
              v-for="field in activeTemplate.fields"
              :key="field.id"
              class="catalog-attribute-dialog__field"
              :class="{
                'catalog-attribute-dialog__field--selected': selectedFieldIds.includes(field.id),
                'catalog-attribute-dialog__field--disabled': fieldDisabled(field.id),
              }"
            >
              <input
                v-model="selectedFieldIds"
                name="catalog_visible_fields"
                type="checkbox"
                :value="field.id"
                :disabled="saving || fieldDisabled(field.id)"
              />
              <span>
                <strong>{{ field.field_name }}</strong>
                <small>{{ fieldTypeLabel(field.field_type) }}</small>
              </span>
            </label>
          </div>
        </div>

        <div v-else key="empty" class="catalog-attribute-dialog__empty">
          {{ $t('items.noConfigurableTemplates') }}
        </div>
      </Transition>
    </form>

    <template #actions>
      <button class="secondary-button" type="button" :disabled="saving" @click="emit('close')">
        {{ $t('items.cancel') }}
      </button>
      <button
        class="primary-button"
        type="submit"
        form="item-catalog-attribute-form"
        :disabled="saving || !activeTemplate || !changed"
      >
        {{ saving ? $t('items.saving') : $t('items.saveSettings') }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { TemplateFieldType } from "../../api/templateFields";
import {
  updateItemAttributeTemplate,
  type ItemAttributeTemplateFieldResponse,
  type ItemAttributeTemplateResponse,
} from "../../api/itemAttributeTemplates";
import { ApiError } from "../../api/errors";
import { notice } from "../../notices/notice";
import ModalDialog from "../ModalDialog.vue";
import SelectControl from "../forms/SelectControl.vue";

const props = defineProps<{
  open: boolean;
  templates: ItemAttributeTemplateResponse[];
}>();

const emit = defineEmits<{
  close: [];
  saved: [template: ItemAttributeTemplateResponse];
}>();
const { t } = useI18n();

const selectedTemplateId = ref<number | null>(null);
const selectedFieldIds = ref<number[]>([]);
const baselineFieldIds = ref<number[]>([]);
const saving = ref(false);

const activeTemplate = computed(
  () => props.templates.find((template) => template.id === selectedTemplateId.value) ?? null,
);
const selectedCount = computed(() => selectedFieldIds.value.length);
const changed = computed(
  () => normalizedIds(selectedFieldIds.value) !== normalizedIds(baselineFieldIds.value),
);

watch(
  () => props.open,
  (open) => {
    if (!open) return;
    const firstTemplate = props.templates[0] ?? null;
    selectedTemplateId.value = firstTemplate?.id ?? null;
    restoreSelection(firstTemplate);
  },
);

watch(selectedTemplateId, (id) => {
  restoreSelection(props.templates.find((template) => template.id === id) ?? null);
});

function restoreSelection(template: ItemAttributeTemplateResponse | null): void {
  const ids =
    template?.fields.filter((field) => field.catalog_visible).map((field) => field.id) ?? [];
  selectedFieldIds.value = [...ids];
  baselineFieldIds.value = [...ids];
}

function fieldDisabled(fieldId: number): boolean {
  return selectedCount.value >= 3 && !selectedFieldIds.value.includes(fieldId);
}

async function save(): Promise<void> {
  const template = activeTemplate.value;
  if (!template || saving.value || !changed.value) return;
  saving.value = true;
  try {
    const selected = new Set(selectedFieldIds.value);
    const updated = await updateItemAttributeTemplate(template.id, {
      fields: template.fields.map((field) => fieldRequest(field, selected.has(field.id))),
    });
    notice.success(t("items.displaySettingsUpdated"));
    emit("saved", updated);
    emit("close");
  } catch (error) {
    notice.error(t("items.displaySettingsSaveFailed"), {
      detail: error instanceof ApiError ? error.message : t("error.network_unavailable"),
    });
  } finally {
    saving.value = false;
  }
}

function fieldRequest(field: ItemAttributeTemplateFieldResponse, catalogVisible: boolean) {
  return {
    definition_id: field.id,
    field_name: field.field_name,
    field_type: field.field_type,
    default_value: field.default_value,
    options: field.options,
    required: field.required,
    searchable: field.searchable,
    catalog_visible: catalogVisible,
    unit: field.unit,
  };
}

function normalizedIds(ids: number[]): string {
  return [...ids].sort((left, right) => left - right).join(",");
}

function fieldTypeLabel(type: TemplateFieldType): string {
  return {
    text: t("items.fieldTypeText"),
    number: t("items.catalogFieldTypeNumber"),
    select: t("items.fieldTypeSelect"),
    date: t("items.fieldTypeDate"),
    file: t("items.fieldTypeFile"),
    url: t("items.catalogFieldTypeUrl"),
    boolean: t("items.catalogFieldTypeBoolean"),
  }[type];
}
</script>

<style scoped lang="scss">
.catalog-attribute-dialog {
  display: grid;
  min-height: 0;
  gap: 16px;
}

.catalog-attribute-dialog__context {
  display: flex;
  min-width: 0;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
  padding: 11px 13px;
  border: 1px solid var(--color-border);
  border-left: 4px solid var(--color-accent);
  background: var(--color-surface-raised);
}

.catalog-attribute-dialog__context > div {
  display: grid;
  min-width: 0;
  gap: 3px;
}

.catalog-attribute-dialog__context span {
  color: var(--color-subtle);
  font-size: 12px;
}

.catalog-attribute-dialog__context strong {
  overflow: hidden;
  font-size: 15px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.catalog-attribute-dialog__count {
  flex: 0 0 auto;
  justify-items: end;
}

.catalog-attribute-dialog__template {
  display: grid;
  gap: 7px;
  color: var(--color-text);
  font-size: 13px;
  font-weight: 650;
}

.catalog-attribute-dialog__panel {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  min-height: 220px;
  max-height: min(42vh, 380px);
  overflow: hidden;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-surface);
}

.catalog-attribute-dialog__panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  min-height: 42px;
  padding: 0 14px;
  border-bottom: 1px solid var(--color-border-strong);
  background: var(--color-surface-raised);
}

.catalog-attribute-dialog__panel-header strong {
  font-size: 13px;
}

.catalog-attribute-dialog__panel-header span {
  color: var(--color-muted);
  font-size: 12px;
  font-variant-numeric: tabular-nums;
}

.catalog-attribute-dialog__options {
  min-height: 0;
  overflow-y: auto;
  padding: 0 10px;
}

.catalog-attribute-dialog__field {
  position: relative;
  display: grid;
  grid-template-columns: 18px minmax(0, 1fr);
  align-items: center;
  gap: 10px;
  min-height: 52px;
  padding: 8px 6px;
  border-bottom: 1px solid var(--color-border);
  cursor: pointer;
}

.catalog-attribute-dialog__field:last-child {
  border-bottom: 0;
}

.catalog-attribute-dialog__field--selected {
  background: var(--color-accent-soft);
  box-shadow: inset 3px 0 var(--color-accent);
}

.catalog-attribute-dialog__field--disabled {
  background: var(--color-surface-raised);
  color: var(--color-muted);
  cursor: not-allowed;
}

.catalog-attribute-dialog__field input {
  width: 16px;
  height: 16px;
  margin: 0;
  accent-color: var(--color-accent);
}

.catalog-attribute-dialog__field input:focus-visible {
  outline: 2px solid var(--color-accent);
  outline-offset: 2px;
}

.catalog-attribute-dialog__field > span {
  display: grid;
  gap: 2px;
  min-width: 0;
}

.catalog-attribute-dialog__field strong {
  overflow-wrap: anywhere;
  font-size: 13px;
}

.catalog-attribute-dialog__field small {
  color: var(--color-muted);
  font-size: 12px;
  font-weight: 400;
}

.catalog-attribute-dialog__empty {
  display: grid;
  min-height: 220px;
  place-items: center;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-surface-raised);
  color: var(--color-muted);
  font-size: 13px;
}

.catalog-attribute-panel-enter-active,
.catalog-attribute-panel-leave-active {
  transition:
    opacity var(--motion-duration-standard) var(--motion-ease-standard),
    transform var(--motion-duration-standard) var(--motion-ease-standard);
}

.catalog-attribute-panel-enter-from,
.catalog-attribute-panel-leave-to {
  opacity: 0;
  transform: translateY(var(--motion-distance-small));
}

@media (hover: hover) and (pointer: fine) {
  .catalog-attribute-dialog__field:not(.catalog-attribute-dialog__field--disabled):hover {
    background: var(--color-surface-raised);
  }

  .catalog-attribute-dialog__field--selected:not(.catalog-attribute-dialog__field--disabled):hover {
    background: var(--color-accent-soft);
  }
}

@media (max-width: 767px) {
  .catalog-attribute-dialog {
    gap: 14px;
  }

  .catalog-attribute-dialog__context {
    gap: 12px;
  }

  .catalog-attribute-dialog__panel {
    min-height: 190px;
    max-height: min(38vh, 330px);
  }

  .catalog-attribute-dialog__options {
    padding: 0 8px;
  }

  .catalog-attribute-dialog__field {
    min-height: 50px;
  }
}
</style>
