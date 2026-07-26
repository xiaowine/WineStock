<!-- 本组件拥有单个物品属性草稿的类型化字段布局；它不保存物品或选择模板。 -->
<template>
  <div
    class="item-attribute-editor"
    :class="{
      'item-attribute-editor--template': templateField,
    }"
  >
    <FormInput
      v-if="!templateField"
      v-model="attribute.fieldName"
      class="item-attribute-editor__name"
      label="属性名称"
      :name="`attribute_name_${attribute.key}`"
      :readonly="!attribute.custom"
      :required="attribute.custom"
      :validation-key="validationKey('name')"
      :error="validationErrors.name"
      maxlength="64"
      placeholder="例如：产地"
    />
    <FormField
      v-if="attribute.custom && attribute.fieldType === 'select'"
      class="item-attribute-editor__options"
      label="候选项"
      :validation-key="validationKey('options')"
      :error="validationErrors.options"
      required
      v-slot="{ describedBy, invalid }"
    >
      <div
        v-for="(_, index) in attribute.options"
        :key="index"
        class="item-attribute-editor__option-row"
      >
        <input
          v-model="attribute.options[index]"
          :name="`attribute_option_${attribute.key}_${index}`"
          required
          placeholder="输入候选项"
          :aria-invalid="invalid || undefined"
          :aria-describedby="describedBy"
        />
        <button
          type="button"
          class="icon-button"
          aria-label="删除候选项"
          @click="removeOption(attribute.options, index)"
        >
          ×
        </button>
      </div>
      <button type="button" class="secondary-button" @click="attribute.options.push('')">
        添加候选项
      </button>
    </FormField>
    <FormSelect
      v-if="!templateField"
      v-model="attribute.fieldType"
      class="item-attribute-editor__type"
      label="类型"
      :name="`attribute_type_${attribute.key}`"
      :disabled="!attribute.custom"
      @focus="rememberType"
      @change="resetValue"
    >
      <option value="text">文本</option>
      <option value="number">数字</option>
      <option value="select">选项</option>
      <option value="date">日期</option>
      <option value="url">网址</option>
      <option value="boolean">布尔</option>
      <option value="file">图片</option>
    </FormSelect>
    <FormField
      class="item-attribute-editor__value"
      :label="templateField?.field_name ?? '属性值'"
      :required="templateField?.required ?? attribute.custom"
      :validation-key="validationKey('value')"
      :error="validationErrors.value"
      v-slot="{ describedBy, invalid }"
    >
      <div class="item-attribute-editor__value-control">
        <AttributeImageField
          v-if="attribute.fieldType === 'file'"
          :model-value="fileValue"
          :delete-on-remove="attribute.fileTemporary"
          :invalid="invalid"
          :label="templateField?.field_name ?? (attribute.fieldName || '属性图片')"
          @update:model-value="updateFile"
        />
        <SelectControl
          v-else-if="attribute.fieldType === 'boolean'"
          v-model="attribute.value"
          :name="`attribute_value_${attribute.key}`"
          :required="templateField?.required ?? attribute.custom"
          :aria-invalid="invalid || undefined"
          :aria-describedby="describedBy"
        >
          <option :value="undefined">请选择</option>
          <option :value="true">是</option>
          <option :value="false">否</option>
        </SelectControl>
        <SelectControl
          v-else-if="attribute.fieldType === 'select'"
          v-model="attribute.value"
          :name="`attribute_value_${attribute.key}`"
          :required="templateField?.required ?? true"
          :aria-invalid="invalid || undefined"
          :aria-describedby="describedBy"
        >
          <option value="">请选择</option>
          <option v-for="option in selectOptions" :key="option" :value="option">
            {{ option }}
          </option>
        </SelectControl>
        <input
          v-else
          v-model="attribute.value"
          :name="`attribute_value_${attribute.key}`"
          :type="inputType"
          :required="templateField?.required ?? attribute.custom"
          :pattern="attribute.fieldType === 'url' ? 'https?://.+' : undefined"
          :aria-invalid="invalid || undefined"
          :aria-describedby="describedBy"
          placeholder="输入属性值"
        />
        <span v-if="unitMode === 'fixed'" class="item-attribute-editor__fixed-unit">{{
          fixedUnitLabel
        }}</span>
        <FormField
          v-else-if="unitMode === 'select'"
          class="item-attribute-editor__unit-value"
          :validation-key="validationKey('unitValue')"
          :error="validationErrors.unitValue"
          v-slot="{ describedBy: unitDescribedBy, invalid: unitInvalid }"
        >
          <SelectControl
            v-model="attribute.unit"
            :name="`attribute_unit_${attribute.key}`"
            aria-label="单位"
            required
            :aria-invalid="unitInvalid || undefined"
            :aria-describedby="unitDescribedBy"
          >
            <option value="">选择单位</option>
            <option v-for="option in unitOptions" :key="option" :value="option">
              {{ option }}
            </option>
          </SelectControl>
        </FormField>
      </div>
    </FormField>
    <FormField
      v-if="attribute.fieldType === 'number' && attribute.custom"
      class="item-attribute-editor__unit"
      label="单位"
      :validation-key="validationKey('unitSettings')"
      :error="validationErrors.unitSettings"
    >
      <button
        type="button"
        class="item-attribute-editor__unit-settings"
        :class="{ 'is-invalid': validationErrors.unitSettings }"
        :aria-invalid="Boolean(validationErrors.unitSettings)"
        @click="unitDialogOpen = true"
      >
        <span>{{ unitSummary }}</span>
        <strong>设置单位</strong>
      </button>
    </FormField>
    <button
      v-if="!templateField"
      class="icon-button item-attribute-editor__remove"
      type="button"
      title="删除属性"
      aria-label="删除属性"
      @click="removeAttribute"
    >
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M4 7h16M9 7V4h6v3M7 7l1 13h8l1-13M10 11v5M14 11v5" />
      </svg>
    </button>
    <ItemUnitSettingsDialog
      :open="unitDialogOpen"
      :attribute-name="attribute.fieldName"
      :unit-mode="attribute.unitMode"
      :fixed-unit="attribute.fixedUnit"
      :unit-options="attribute.unitOptions"
      @close="unitDialogOpen = false"
      @save="applyUnitSettings"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import AttributeImageField from "../attributes/AttributeImageField.vue";
import type { ItemAttributeTemplateFieldResponse } from "../../api/itemAttributeTemplates";
import type { ImageDraftValue } from "../attributes/imageDraft";
import type { ItemAttributeDraft } from "../../pages/items/model";
import { discardTemporaryAttributeFile } from "../../pages/items/fileCleanup";
import { notice } from "../../notices/notice";
import ItemUnitSettingsDialog from "./ItemUnitSettingsDialog.vue";
import type { ItemAttributeUnitMode } from "../../api/itemAttributeTemplates";
import FormField from "../forms/FormField.vue";
import FormInput from "../forms/FormInput.vue";
import FormSelect from "../forms/FormSelect.vue";
import SelectControl from "../forms/SelectControl.vue";

const props = withDefaults(
  defineProps<{
    attribute: ItemAttributeDraft;
    templateField?: ItemAttributeTemplateFieldResponse;
    validationErrors?: Record<string, string>;
  }>(),
  {
    templateField: undefined,
    validationErrors: () => ({}),
  },
);
const emit = defineEmits<{ remove: [] }>();
const fileValue = computed(() =>
  typeof props.attribute.value === "object" && props.attribute.value?.kind === "file"
    ? props.attribute.value
    : undefined,
);
const inputType = computed(() =>
  props.attribute.fieldType === "number"
    ? "number"
    : props.attribute.fieldType === "date"
      ? "date"
      : props.attribute.fieldType === "url"
        ? "url"
        : "text",
);
const unitMode = computed(() => props.templateField?.unit.mode ?? props.attribute.unitMode);
const fixedUnitLabel = computed(() => props.templateField?.unit.value ?? props.attribute.fixedUnit);
const selectOptions = computed(() => props.templateField?.options ?? props.attribute.options);
const unitOptions = computed(
  () => props.templateField?.unit.options ?? props.attribute.unitOptions,
);
const unitDialogOpen = ref(false);
const unitSummary = computed(() => {
  if (props.attribute.unitMode === "fixed")
    return `指定单位 · ${props.attribute.fixedUnit || "未设置"}`;
  if (props.attribute.unitMode === "select")
    return `选择单位 · ${props.attribute.unitOptions.length} 个候选`;
  return "无单位";
});
let previousType = props.attribute.fieldType;

function validationKey(field: string): string {
  return `attribute.${props.attribute.key}.${field}`;
}

function rememberType(): void {
  previousType = props.attribute.fieldType;
}

function removeOption(options: string[], index: number): void {
  const [removed] = options.splice(index, 1);
  if (props.attribute.value === removed || props.attribute.unit === removed) {
    props.attribute.value = props.attribute.fieldType === "select" ? "" : props.attribute.value;
    props.attribute.unit = props.attribute.unit === removed ? "" : props.attribute.unit;
  }
}

function applyUnitSettings(settings: {
  mode: ItemAttributeUnitMode;
  fixedUnit: string;
  options: string[];
}): void {
  const previousUnit = props.attribute.unit;
  props.attribute.unitMode = settings.mode;
  props.attribute.fixedUnit = settings.fixedUnit;
  props.attribute.unitOptions = settings.options;
  if (
    settings.mode === "none" ||
    settings.mode === "fixed" ||
    !settings.options.includes(previousUnit)
  ) {
    props.attribute.unit = "";
  }
  unitDialogOpen.value = false;
}

function updateFile(value: ImageDraftValue | undefined): void {
  props.attribute.value = value;
  props.attribute.fileTemporary = true;
}

async function removeAttribute(): Promise<void> {
  await discardTemporaryFile();
  emit("remove");
}

async function resetValue(): Promise<void> {
  if (
    props.attribute.value !== "" &&
    props.attribute.value !== undefined &&
    !window.confirm("修改类型会清空当前属性值，是否继续？")
  ) {
    props.attribute.fieldType = previousType;
    return;
  }
  await discardTemporaryFile();
  props.attribute.value = props.attribute.fieldType === "boolean" ? undefined : "";
  props.attribute.unit = "";
  props.attribute.options = [];
  props.attribute.unitMode = "none";
  props.attribute.fixedUnit = "";
  props.attribute.unitOptions = [];
  previousType = props.attribute.fieldType;
}

async function discardTemporaryFile(): Promise<void> {
  try {
    await discardTemporaryAttributeFile(props.attribute);
  } catch {
    notice.warning("临时图片未能立即删除", { detail: "服务会在超过保留期限后自动清理。" });
  }
}
</script>
