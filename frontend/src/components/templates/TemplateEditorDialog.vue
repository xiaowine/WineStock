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
      <section class="template-editor__basics" aria-label="模板基础信息">
        <FormInput
          v-model="draft.name"
          label="模板名称"
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
          label="模板说明"
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
            <h3 id="template-fields-heading">字段结构</h3>
            <span>{{ draft.fields.length }} / 64</span>
            <span>目录字段：已选择 {{ catalogVisibleCount }} / 3</span>
          </div>
          <button
            class="secondary-button"
            type="button"
            :disabled="submitting || draft.fields.length >= 64"
            @click="addField"
          >
            添加字段
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
                <strong>{{ field.fieldName.trim() || "未命名字段" }}</strong>
                <span
                  >{{ fieldTypeLabel(field.fieldType) }}{{ field.required ? " · 必填" : "" }}</span
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
                title="上移字段"
                :aria-label="`上移字段 ${field.fieldName || index + 1}`"
                :disabled="submitting || index === 0"
                @click="moveField(index, -1)"
              >
                ↑
              </button>
              <button
                class="icon-button"
                type="button"
                title="下移字段"
                :aria-label="`下移字段 ${field.fieldName || index + 1}`"
                :disabled="submitting || index === draft.fields.length - 1"
                @click="moveField(index, 1)"
              >
                ↓
              </button>
              <button
                class="icon-button template-field-card__delete"
                type="button"
                title="删除字段"
                :aria-label="`删除字段 ${field.fieldName || index + 1}`"
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
                label="字段名称"
                :validation-key="`fields.${index}.field_name`"
                :error="errors[`fields.${index}.field_name`]"
                maxlength="64"
                autocomplete="off"
                required
                :disabled="submitting"
              />
              <FormSelect
                :model-value="field.fieldType"
                label="字段类型"
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
                <span>实际录入必填</span></label
              >
              <label
                ><input v-model="field.searchable" type="checkbox" :disabled="submitting" />
                <span>允许参与筛选</span></label
              >
              <label>
                <input
                  v-model="field.catalogVisible"
                  type="checkbox"
                  :disabled="submitting || (!field.catalogVisible && catalogVisibleCount >= 3)"
                />
                <span>在物品目录展示</span>
              </label>
            </div>

            <div v-if="field.fieldType === 'file'" class="template-field-card__hint">
              图片字段不支持默认文件。
            </div>
            <FormSelect
              v-else-if="field.fieldType === 'boolean'"
              v-model="field.defaultValue"
              label="默认值"
              :validation-key="`fields.${index}.default_value`"
              :error="errors[`fields.${index}.default_value`]"
              :disabled="submitting"
            >
              <option value="">不设置</option>
              <option value="true">是</option>
              <option value="false">否</option>
            </FormSelect>
            <FormSelect
              v-else-if="field.fieldType === 'select'"
              v-model="field.defaultValue"
              label="默认值"
              :validation-key="`fields.${index}.default_value`"
              :error="errors[`fields.${index}.default_value`]"
              :disabled="submitting"
            >
              <option value="">不设置</option>
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
              label="默认值"
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
              label="选择候选项"
              :error-prefix="`fields.${index}.options`"
              :errors="errors"
              :max-items="128"
              :max-length="128"
              :disabled="submitting"
            />

            <section class="template-field-card__unit" aria-label="单位规则">
              <FormSelect
                :model-value="field.unitMode"
                label="单位规则"
                :validation-key="`fields.${index}.unit_mode`"
                :disabled="submitting"
                @update:model-value="requestUnitModeChange(index, $event)"
              >
                <option value="none">无单位</option>
                <option value="fixed">固定单位</option>
                <option value="select">选择单位</option>
              </FormSelect>
              <FormInput
                v-if="field.unitMode === 'fixed'"
                v-model="field.unitValue"
                label="固定单位"
                :validation-key="`fields.${index}.unit_value`"
                :error="errors[`fields.${index}.unit_value`]"
                maxlength="32"
                autocomplete="off"
                :disabled="submitting"
              />
              <OptionEditor
                v-if="field.unitMode === 'select'"
                v-model="field.unitOptions"
                label="单位候选项"
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
          <dt>模板名称</dt>
          <dd>{{ template?.name }}</dd>
        </div>
        <div>
          <dt>模板说明</dt>
          <dd>{{ template?.description || "暂无说明" }}</dd>
        </div>
        <div v-if="template && 'item_usage_count' in template">
          <dt>当前有效物品使用</dt>
          <dd>{{ template.item_usage_count }} 个</dd>
        </div>
      </dl>
      <section class="template-detail__fields" aria-label="字段结构">
        <h3>
          字段结构 <span>{{ draft.fields.length }} 个字段</span>
        </h3>
        <article v-for="(field, index) in draft.fields" :key="field.key">
          <header>
            <span>{{ index + 1 }}</span
            ><strong>{{ field.fieldName }}</strong
            ><em>{{ fieldTypeLabel(field.fieldType) }}</em>
          </header>
          <dl>
            <div>
              <dt>录入规则</dt>
              <dd>
                {{ field.required ? "必填" : "选填" }} ·
                {{ field.searchable ? "可筛选" : "不参与筛选" }}
              </dd>
            </div>
            <div v-if="field.defaultValue">
              <dt>默认值</dt>
              <dd>{{ field.defaultValue }}</dd>
            </div>
            <div v-if="field.options.length">
              <dt>候选项</dt>
              <dd>{{ field.options.join("、") }}</dd>
            </div>
            <div>
              <dt>单位</dt>
              <dd>{{ unitLabel(field) }}</dd>
            </div>
            <div>
              <dt>目录展示</dt>
              <dd>{{ field.catalogVisible ? "是" : "否" }}</dd>
            </div>
          </dl>
        </article>
      </section>
    </div>

    <template #actions>
      <button class="secondary-button" type="button" :disabled="submitting" @click="requestClose">
        {{ readOnly ? "关闭" : "取消" }}
      </button>
      <button v-if="readOnly && canEdit" class="primary-button" type="button" @click="emit('edit')">
        编辑模板
      </button>
      <button
        v-else-if="!readOnly"
        class="primary-button"
        type="submit"
        :form="formId"
        :disabled="submitting"
      >
        {{ submitting ? "正在保存…" : template ? "保存模板" : "创建模板" }}
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
      <button class="secondary-button" type="button" @click="confirmState = null">取消</button>
      <button
        :class="confirmState?.danger ? 'danger-button' : 'primary-button'"
        type="button"
        @click="runConfirmedAction"
      >
        {{ confirmState?.confirmLabel ?? "继续" }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { computed, nextTick, reactive, ref, useId, watch } from "vue";
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

const formId = `template-editor-form-${useId()}`;
const draft = reactive<TemplateDraft>(createTemplateDraft(null));
const errors = ref<Record<string, string>>({});
const initialSnapshot = ref("");
const confirmState = ref<ConfirmState | null>(null);
useFormValidation(errors);

const fieldTypeOptions: { value: TemplateFieldType; label: string }[] = [
  { value: "text", label: "文本" },
  { value: "number", label: "数字" },
  { value: "select", label: "选择" },
  { value: "date", label: "日期" },
  { value: "file", label: "图片" },
  { value: "url", label: "链接" },
  { value: "boolean", label: "是/否" },
];

const dialogTitle = computed(() =>
  props.readOnly ? "物品属性模板详情" : props.template ? "编辑物品属性模板" : "新建物品属性模板",
);
const dialogDescription = "定义物品长期属性、单位和最多三个目录展示字段。";
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
    notice.warning("请检查模板信息", { detail: Object.values(result.errors)[0] });
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
    title: "删除字段？",
    description: `字段“${field.fieldName || index + 1}”及其配置会从当前模板草稿中移除。`,
    confirmLabel: "删除字段",
    danger: true,
    action: remove,
  };
}

function requestFieldTypeChange(index: number, value: unknown): void {
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
      title: "更改字段类型？",
      description: "与新类型不兼容的默认值或候选项会被清除。",
      confirmLabel: "更改类型",
      action: apply,
    };
}

function requestUnitModeChange(index: number, value: unknown): void {
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
      title: "更改单位规则？",
      description: "当前固定单位或单位候选项会被清除。",
      confirmLabel: "更改规则",
      action: apply,
    };
}

function requestClose(): void {
  if (!props.readOnly && serializeTemplateDraft(draft) !== initialSnapshot.value) {
    confirmState.value = {
      title: "放弃未保存修改？",
      description: "关闭后，本次模板字段和基础信息修改不会保留。",
      confirmLabel: "放弃修改",
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
  if (field.unitMode === "fixed") return `固定：${field.unitValue}`;
  if (field.unitMode === "select") return `可选：${field.unitOptions.join("、")}`;
  return "无单位";
}
</script>
