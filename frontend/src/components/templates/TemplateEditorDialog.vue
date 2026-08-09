<!--
  本组件拥有物品属性模板的查看与编辑工作区、字段草稿和本地校验。
  它不调用模板 API，也不决定模板的删除或复制流程。
-->
<template>
  <ModalDialog
    :open="open"
    :title="dialogTitle"
    :description="dialogDescription"
    :busy="submitting"
    workspace
    @close="requestClose"
  >
    <form v-if="!readOnly" :id="formId" class="template-editor" novalidate @submit.prevent="submit">
      <section class="template-editor__basics" :aria-label="$t('templates.basicsAria')">
        <FormInput
          v-model="draft.name"
          :label="$t('templates.templateNameLabel')"
          validation-key="name"
          :error="errors.name"
          maxlength="128"
          autocomplete="off"
          autofocus
          required
          :disabled="submitting"
        />
        <FormTextarea
          v-model="draft.description"
          :label="$t('templates.templateDescriptionLabel')"
          validation-key="description"
          :error="errors.description"
          maxlength="1024"
          :rows="3"
          :disabled="submitting"
        />
      </section>

      <section class="template-editor__fields" aria-labelledby="template-fields-heading">
        <header class="template-editor__fields-toolbar">
          <div>
            <h3 id="template-fields-heading">{{ $t('templates.fieldStructure') }}</h3>
            <span>{{ draft.fields.length }} / 64</span>
            <span>{{ $t('templates.catalogVisibleCount', { n: catalogVisibleCount }) }}</span>
          </div>
          <button
            class="secondary-button"
            type="button"
            :disabled="submitting || draft.fields.length >= 64"
            @click="addField"
          >
            {{ $t('templates.addField') }}
          </button>
        </header>

        <article
          v-for="(field, index) in draft.fields"
          :key="field.key"
          class="template-field-card"
          :class="{ 'template-field-card--error': fieldHasError(index) }"
        >
          <header class="template-field-card__header">
            <button
              class="template-field-card__toggle"
              type="button"
              :aria-expanded="field.expanded"
              :aria-controls="`${field.key}-body`"
              @click="field.expanded = !field.expanded"
            >
              <span class="template-field-card__order">{{ index + 1 }}</span>
              <span class="template-field-card__identity">
                <strong>{{ field.fieldName.trim() || $t('templates.unnamedField') }}</strong>
                <span
                  >{{ fieldTypeLabel(field.fieldType)
                  }}<template v-if="field.required"> · {{ $t('common.required') }}</template></span
                >
              </span>
              <svg viewBox="0 0 16 16" aria-hidden="true">
                <path :d="field.expanded ? 'm4 10 4-4 4 4' : 'm4 6 4 4 4-4'" />
              </svg>
            </button>
            <span class="template-field-card__actions">
              <button
                class="icon-button"
                type="button"
                :title="$t('templates.moveFieldUp')"
                :aria-label="
                  $t('templates.moveFieldUpAria', { name: field.fieldName || index + 1 })
                "
                :disabled="submitting || index === 0"
                @click="moveField(index, -1)"
              >
                ↑
              </button>
              <button
                class="icon-button"
                type="button"
                :title="$t('templates.moveFieldDown')"
                :aria-label="
                  $t('templates.moveFieldDownAria', { name: field.fieldName || index + 1 })
                "
                :disabled="submitting || index === draft.fields.length - 1"
                @click="moveField(index, 1)"
              >
                ↓
              </button>
              <button
                class="icon-button template-field-card__delete"
                type="button"
                :title="$t('templates.deleteField')"
                :aria-label="
                  $t('templates.deleteFieldAria', { name: field.fieldName || index + 1 })
                "
                :disabled="submitting || draft.fields.length === 1"
                @click="requestDeleteField(index)"
              >
                ×
              </button>
            </span>
          </header>

          <div v-show="field.expanded" :id="`${field.key}-body`" class="template-field-card__body">
            <div class="template-field-card__grid">
              <FormInput
                v-model="field.fieldName"
                :label="$t('templates.fieldNameLabel')"
                :validation-key="`fields.${index}.field_name`"
                :error="errors[`fields.${index}.field_name`]"
                maxlength="64"
                autocomplete="off"
                required
                :disabled="submitting"
              />
              <FormSelect
                :model-value="field.fieldType"
                :label="$t('templates.fieldTypeName')"
                :validation-key="`fields.${index}.field_type`"
                :error="errors[`fields.${index}.field_type`]"
                :disabled="submitting"
                @update:model-value="requestFieldTypeChange(index, $event)"
              >
                <option
                  v-for="option in fieldTypeOptions"
                  :key="option.value"
                  :value="option.value"
                >
                  {{ option.label }}
                </option>
              </FormSelect>
            </div>

            <div class="template-field-card__switches">
              <label
                ><input v-model="field.required" type="checkbox" :disabled="submitting" />
                <span>{{ $t('templates.requiredInput') }}</span></label
              >
              <label
                ><input v-model="field.searchable" type="checkbox" :disabled="submitting" />
                <span>{{ $t('templates.searchableToggle') }}</span></label
              >
              <label>
                <input
                  v-model="field.catalogVisible"
                  type="checkbox"
                  :disabled="submitting || (!field.catalogVisible && catalogVisibleCount >= 3)"
                />
                <span>{{ $t('templates.catalogVisibleToggle') }}</span>
              </label>
            </div>

            <div v-if="field.fieldType === 'file'" class="template-field-card__hint">
              {{ $t('templates.fileNoDefaultHint') }}
            </div>
            <FormSelect
              v-else-if="field.fieldType === 'boolean'"
              v-model="field.defaultValue"
              :label="$t('templates.defaultValueLabel')"
              :validation-key="`fields.${index}.default_value`"
              :error="errors[`fields.${index}.default_value`]"
              :disabled="submitting"
            >
              <option value="">{{ $t('templates.noDefault') }}</option>
              <option value="true">{{ $t('common.yes') }}</option>
              <option value="false">{{ $t('common.no') }}</option>
            </FormSelect>
            <FormSelect
              v-else-if="field.fieldType === 'select'"
              v-model="field.defaultValue"
              :label="$t('templates.defaultValueLabel')"
              :validation-key="`fields.${index}.default_value`"
              :error="errors[`fields.${index}.default_value`]"
              :disabled="submitting"
            >
              <option value="">{{ $t('templates.noDefault') }}</option>
              <option
                v-for="option in normalizedOptions(field.options)"
                :key="option"
                :value="option"
              >
                {{ option }}
              </option>
            </FormSelect>
            <FormInput
              v-else
              v-model="field.defaultValue"
              :label="$t('templates.defaultValueLabel')"
              :validation-key="`fields.${index}.default_value`"
              :error="errors[`fields.${index}.default_value`]"
              maxlength="256"
              :placeholder="
                field.fieldType === 'date'
                  ? 'YYYY-MM-DD'
                  : field.fieldType === 'url'
                    ? 'https://example.com'
                    : undefined
              "
              autocomplete="off"
              :inputmode="field.fieldType === 'number' ? 'decimal' : undefined"
              :disabled="submitting"
            />

            <OptionEditor
              v-if="field.fieldType === 'select'"
              v-model="field.options"
              :label="$t('templates.optionChoicesLabel')"
              :error-prefix="`fields.${index}.options`"
              :errors="errors"
              :max-items="128"
              :max-length="128"
              :disabled="submitting"
            />

            <section class="template-field-card__unit" :aria-label="$t('templates.unitRuleLabel')">
              <FormSelect
                :model-value="field.unitMode"
                :label="$t('templates.unitRuleLabel')"
                :validation-key="`fields.${index}.unit_mode`"
                :disabled="submitting"
                @update:model-value="requestUnitModeChange(index, $event)"
              >
                <option value="none">{{ $t('templates.unitModeNone') }}</option>
                <option value="fixed">{{ $t('templates.unitModeFixed') }}</option>
                <option value="select">{{ $t('templates.unitModeSelect') }}</option>
              </FormSelect>
              <FormInput
                v-if="field.unitMode === 'fixed'"
                v-model="field.unitValue"
                :label="$t('templates.unitModeFixed')"
                :validation-key="`fields.${index}.unit_value`"
                :error="errors[`fields.${index}.unit_value`]"
                maxlength="32"
                autocomplete="off"
                :disabled="submitting"
              />
              <OptionEditor
                v-if="field.unitMode === 'select'"
                v-model="field.unitOptions"
                :label="$t('templates.unitOptionsLabel')"
                :error-prefix="`fields.${index}.unit_options`"
                :errors="errors"
                :max-items="32"
                :max-length="32"
                :disabled="submitting"
              />
            </section>
          </div>
        </article>
      </section>
    </form>

    <div v-else class="template-detail">
      <dl class="template-detail__basics">
        <div>
          <dt>{{ $t('templates.templateNameLabel') }}</dt>
          <dd>{{ template?.name }}</dd>
        </div>
        <div>
          <dt>{{ $t('templates.templateDescriptionLabel') }}</dt>
          <dd>{{ template?.description || $t('templates.noDescription') }}</dd>
        </div>
        <div v-if="template && 'item_usage_count' in template">
          <dt>{{ $t('templates.activeItemUsage') }}</dt>
          <dd>{{ $t('templates.itemCount', { n: template.item_usage_count }) }}</dd>
        </div>
      </dl>
      <section class="template-detail__fields" :aria-label="$t('templates.fieldStructure')">
        <h3>
          {{ $t('templates.fieldStructure') }}
          <span>{{ $t('templates.fieldCount', { n: draft.fields.length }) }}</span>
        </h3>
        <article v-for="(field, index) in draft.fields" :key="field.key">
          <header>
            <span>{{ index + 1 }}</span
            ><strong>{{ field.fieldName }}</strong
            ><em>{{ fieldTypeLabel(field.fieldType) }}</em>
          </header>
          <dl>
            <div>
              <dt>{{ $t('templates.entryRuleLabel') }}</dt>
              <dd>
                {{ field.required ? $t('common.required') : $t('templates.optionalEntry') }} ·
                {{ field.searchable ? $t('templates.searchableCountLabel') : $t('templates.notSearchable') }}
              </dd>
            </div>
            <div v-if="field.defaultValue">
              <dt>{{ $t('templates.defaultValueLabel') }}</dt>
              <dd>{{ field.defaultValue }}</dd>
            </div>
            <div v-if="field.options.length">
              <dt>{{ $t('templates.optionsDetailLabel') }}</dt>
              <dd>{{ field.options.join("、") }}</dd>
            </div>
            <div>
              <dt>{{ $t('common.unit') }}</dt>
              <dd>{{ unitLabel(field) }}</dd>
            </div>
            <div>
              <dt>{{ $t('templates.catalogVisibleDetail') }}</dt>
              <dd>{{ field.catalogVisible ? $t('common.yes') : $t('common.no') }}</dd>
            </div>
          </dl>
        </article>
      </section>
    </div>

    <template #actions>
      <button class="secondary-button" type="button" :disabled="submitting" @click="requestClose">
        {{ readOnly ? $t('common.close') : $t('common.cancel') }}
      </button>
      <button v-if="readOnly && canEdit" class="primary-button" type="button" @click="emit('edit')">
        {{ $t('templates.editTemplate') }}
      </button>
      <button
        v-else-if="!readOnly"
        class="primary-button"
        type="submit"
        :form="formId"
        :disabled="submitting"
      >
        {{
          submitting
            ? $t('templates.savingNow')
            : template
              ? $t('templates.saveTemplateButton')
              : $t('templates.createTemplateButton')
        }}
      </button>
    </template>
  </ModalDialog>

  <ModalDialog
    :open="Boolean(confirmState)"
    :title="confirmState?.title ?? ''"
    :description="confirmState?.description"
    compact
    nested
    @close="confirmState = null"
  >
    <template #actions>
      <button class="secondary-button" type="button" @click="confirmState = null">
        {{ $t('common.cancel') }}
      </button>
      <button
        :class="confirmState?.danger ? 'danger-button' : 'primary-button'"
        type="button"
        @click="runConfirmedAction"
      >
        {{ confirmState?.confirmLabel ?? $t('templates.continueAction') }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { computed, nextTick, reactive, ref, useId, watch } from "vue";
import { useI18n } from "vue-i18n";
import type {
  ItemAttributeTemplateResponse,
  ItemAttributeUnitMode,
} from "../../api/itemAttributeTemplates";
import type { TemplateFieldType } from "../../api/templateFields";
import { useFormValidation } from "../../composables/useFormValidation";
import { notice } from "../../notices/notice";
import {
  clearIncompatibleFieldData,
  createEmptyField,
  createTemplateDraft,
  fieldTypeLabel,
  serializeTemplateDraft,
  validateTemplateDraft,
  type TemplateDraft,
  type TemplateFieldDraft,
} from "../../pages/templates/model";
import ModalDialog from "../ModalDialog.vue";
import FormInput from "../forms/FormInput.vue";
import FormSelect from "../forms/FormSelect.vue";
import FormTextarea from "../forms/FormTextarea.vue";
import OptionEditor from "./TemplateOptionEditor.vue";

interface ConfirmState {
  title: string;
  description: string;
  confirmLabel?: string;
  danger?: boolean;
  action: () => void;
}

const props = defineProps<{
  open: boolean;
  template: ItemAttributeTemplateResponse | null;
  readOnly: boolean;
  canEdit: boolean;
  submitting: boolean;
  errorMessage: string;
  fieldErrors: Record<string, string>;
}>();

const emit = defineEmits<{
  close: [];
  edit: [];
  submit: [draft: TemplateDraft];
}>();

const { t } = useI18n();
const formId = `template-editor-form-${useId()}`;
const draft = reactive<TemplateDraft>(createTemplateDraft(null));
const errors = ref<Record<string, string>>({});
const initialSnapshot = ref("");
const confirmState = ref<ConfirmState | null>(null);
useFormValidation(errors);

const fieldTypeOptions: { value: TemplateFieldType; label: string }[] = (
  ["text", "number", "select", "date", "file", "url", "boolean"] as TemplateFieldType[]
).map((value) => ({ value, label: fieldTypeLabel(value) }));

const dialogTitle = computed(() =>
  props.readOnly
    ? t("templates.templateDetailTitle")
    : props.template
      ? t("templates.editTemplateTitle")
      : t("templates.newTemplateTitle"),
);
const dialogDescription = computed(() => t("templates.editorDescription"));
const catalogVisibleCount = computed(
  () => draft.fields.filter((field) => field.catalogVisible).length,
);

watch(
  () => props.open,
  (open) => {
    if (!open) return;
    resetDraft();
  },
);

watch(
  () => props.readOnly,
  () => {
    if (props.open) resetDraft();
  },
);

watch(
  () => props.fieldErrors,
  (value) => {
    if (props.open) errors.value = { ...value };
  },
  { deep: true },
);

function resetDraft(): void {
  Object.assign(draft, createTemplateDraft(props.template));
  errors.value = { ...props.fieldErrors };
  confirmState.value = null;
  initialSnapshot.value = serializeTemplateDraft(draft);
}

function submit(): void {
  const result = validateTemplateDraft(draft);
  errors.value = result.errors;
  if (result.firstFieldIndex !== null) draft.fields[result.firstFieldIndex].expanded = true;
  if (Object.keys(result.errors).length) {
    notice.warning(t("templates.checkTemplateInfo"), { detail: Object.values(result.errors)[0] });
    return;
  }
  emit("submit", JSON.parse(JSON.stringify(draft)) as TemplateDraft);
}

function addField(): void {
  if (draft.fields.length >= 64) return;
  draft.fields.push(createEmptyField(draft.fields.length < 3));
  void nextTick(() =>
    document.querySelector<HTMLElement>(`#${draft.fields.at(-1)?.key}-body input`)?.focus(),
  );
}

function moveField(index: number, offset: -1 | 1): void {
  const target = index + offset;
  if (target < 0 || target >= draft.fields.length) return;
  const [field] = draft.fields.splice(index, 1);
  draft.fields.splice(target, 0, field);
}

function requestDeleteField(index: number): void {
  if (draft.fields.length === 1) return;
  const field = draft.fields[index];
  const remove = () => draft.fields.splice(index, 1);
  if (
    !field.fieldName.trim() &&
    !field.defaultValue.trim() &&
    !field.options.some((item) => item.trim())
  ) {
    remove();
    return;
  }
  confirmState.value = {
    title: t("templates.deleteFieldConfirmTitle"),
    description: t("templates.deleteFieldConfirmDescription", {
      name: field.fieldName || index + 1,
    }),
    confirmLabel: t("templates.deleteField"),
    danger: true,
    action: remove,
  };
}

function requestFieldTypeChange(
  index: number,
  value: string | number | boolean | null | undefined,
): void {
  const nextType = value as TemplateFieldType;
  const field = draft.fields[index];
  if (nextType === field.fieldType) return;
  const needsConfirmation = Boolean(
    field.defaultValue.trim() || field.options.some((option) => option.trim()),
  );
  const apply = () => clearIncompatibleFieldData(field, nextType);
  if (!needsConfirmation) apply();
  else
    confirmState.value = {
      title: t("templates.changeTypeConfirmTitle"),
      description: t("templates.changeTypeConfirmDescription"),
      confirmLabel: t("templates.changeTypeConfirmLabel"),
      action: apply,
    };
}

function requestUnitModeChange(
  index: number,
  value: string | number | boolean | null | undefined,
): void {
  const nextMode = value as ItemAttributeUnitMode;
  const field = draft.fields[index];
  if (nextMode === field.unitMode) return;
  const hasCurrentData =
    field.unitValue.trim() || field.unitOptions.some((option) => option.trim());
  const apply = () => {
    field.unitMode = nextMode;
    field.unitValue = "";
    field.unitOptions = [];
  };
  if (!hasCurrentData) apply();
  else
    confirmState.value = {
      title: t("templates.changeUnitRuleConfirmTitle"),
      description: t("templates.changeUnitRuleConfirmDescription"),
      confirmLabel: t("templates.changeUnitRuleConfirmLabel"),
      action: apply,
    };
}

function requestClose(): void {
  if (!props.readOnly && serializeTemplateDraft(draft) !== initialSnapshot.value) {
    confirmState.value = {
      title: t("templates.discardChangesTitle"),
      description: t("templates.discardTemplateChangesDescription"),
      confirmLabel: t("templates.discardChanges"),
      danger: true,
      action: () => emit("close"),
    };
    return;
  }
  emit("close");
}

function runConfirmedAction(): void {
  const action = confirmState.value?.action;
  confirmState.value = null;
  action?.();
}

function fieldHasError(index: number): boolean {
  return Object.keys(errors.value).some((key) => key.startsWith(`fields.${index}.`));
}

function normalizedOptions(options: readonly string[]): string[] {
  return options.map((option) => option.trim()).filter(Boolean);
}

function unitLabel(field: TemplateFieldDraft): string {
  if (field.unitMode === "fixed") return t("templates.unitFixed", { value: field.unitValue });
  if (field.unitMode === "select")
    return t("templates.unitSelectable", { value: field.unitOptions.join("、") });
  return t("templates.unitModeNone");
}
</script>
