<!-- 本组件只编辑数字属性的单位定义草稿；取消关闭不会修改外部属性。 -->
<template>
  <ModalDialog
    :open="open"
    title="设置单位"
    :description="attributeName ? `配置“${attributeName}”的单位规则` : '配置当前数字属性的单位规则'"
    compact
    nested
    @close="close"
  >
    <div class="item-unit-settings">
      <fieldset class="item-unit-settings__mode">
        <legend>单位规则</legend>
        <div class="item-unit-settings__segments" role="radiogroup" aria-label="单位规则">
          <button
            v-for="option in modeOptions"
            :key="option.value"
            type="button"
            role="radio"
            :aria-checked="mode === option.value"
            :class="{ 'is-active': mode === option.value }"
            :autofocus="option.value === 'none'"
            @click="selectMode(option.value)"
          >
            {{ option.label }}
          </button>
        </div>
      </fieldset>

      <FormInput
        v-if="mode === 'fixed'"
        v-model="fixedUnit"
        label="单位名称"
        validation-key="fixedUnit"
        :error="fieldErrors.fixedUnit"
        name="unit_setting_fixed"
        maxlength="32"
        placeholder="例如：kg"
        required
      />

      <FormField
        v-if="mode === 'select'"
        class="item-unit-settings__options-field"
        validation-key="options"
        :error="fieldErrors.options"
      >
        <section class="item-unit-settings__options" aria-labelledby="unit-options-heading">
          <header class="item-unit-settings__options-header">
            <div>
              <h3 id="unit-options-heading">
                单位候选 <span>{{ options.length }}/32</span>
              </h3>
              <p>名称不能为空，忽略大小写不能重复。</p>
            </div>
            <button
              type="button"
              class="icon-button item-unit-settings__add"
              title="添加单位"
              aria-label="添加单位"
              :disabled="options.length >= 32"
              @click="addOption"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M12 5v14M5 12h14" />
              </svg>
            </button>
          </header>
          <div v-for="(_, index) in options" :key="index" class="item-unit-settings__option">
            <FormField
              :validation-key="`option.${index}`"
              :error="fieldErrors[`option.${index}`]"
              v-slot="{ describedBy, invalid }"
            >
              <input
                v-model="options[index]"
                :name="`unit_setting_option_${index}`"
                maxlength="32"
                :placeholder="`候选单位 ${index + 1}`"
                :aria-label="`候选单位 ${index + 1}`"
                :aria-invalid="invalid || undefined"
                :aria-describedby="describedBy"
              />
            </FormField>
            <button
              type="button"
              class="icon-button"
              :aria-label="`删除候选单位 ${index + 1}`"
              @click="removeOption(index)"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M4 7h16M9 7V4h6v3M7 7l1 13h8l1-13M10 11v5M14 11v5" />
              </svg>
            </button>
          </div>
          <p v-if="options.length === 0" class="item-unit-settings__empty">尚未添加候选单位</p>
        </section>
      </FormField>
    </div>

    <template #actions>
      <button type="button" class="secondary-button" @click="close">取消</button>
      <button type="button" class="primary-button" @click="save">应用设置</button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import type { ItemAttributeUnitMode } from "../../api/itemAttributeTemplates";
import { useFormValidation } from "../../composables/useFormValidation";
import { notice } from "../../notices/notice";
import ModalDialog from "../ModalDialog.vue";
import FormField from "../forms/FormField.vue";
import FormInput from "../forms/FormInput.vue";

const props = defineProps<{
  open: boolean;
  attributeName: string;
  unitMode: ItemAttributeUnitMode;
  fixedUnit: string;
  unitOptions: string[];
}>();
const emit = defineEmits<{
  close: [];
  save: [settings: { mode: ItemAttributeUnitMode; fixedUnit: string; options: string[] }];
}>();

const mode = ref<ItemAttributeUnitMode>("none");
const fixedUnit = ref("");
const options = ref<string[]>([]);
const fieldErrors = ref<Record<string, string>>({});
const { clearErrors } = useFormValidation(fieldErrors);
const modeOptions: Array<{ value: ItemAttributeUnitMode; label: string }> = [
  { value: "none", label: "无单位" },
  { value: "fixed", label: "指定单位" },
  { value: "select", label: "选择单位" },
];

watch(
  () => props.open,
  (open) => {
    if (!open) return;
    mode.value = props.unitMode;
    fixedUnit.value = props.fixedUnit;
    options.value = [...props.unitOptions];
    clearErrors();
  },
);

function close(): void {
  emit("close");
}

function selectMode(nextMode: ItemAttributeUnitMode): void {
  mode.value = nextMode;
  clearErrors();
}

function addOption(): void {
  options.value.push("");
  clearErrors();
}

function removeOption(index: number): void {
  options.value.splice(index, 1);
  clearErrors();
}

function save(): void {
  const normalizedFixed = fixedUnit.value.trim();
  const normalizedOptions = options.value.map((option) => option.trim());
  const errors: Record<string, string> = {};
  if (mode.value === "fixed" && !normalizedFixed) {
    errors.fixedUnit = "请输入单位名称。";
  }
  if (mode.value === "select") {
    const names = new Set<string>();
    if (normalizedOptions.length === 0) errors.options = "至少添加一个候选单位。";
    normalizedOptions.forEach((option, index) => {
      if (!option) {
        errors[`option.${index}`] = "请输入候选单位。";
        return;
      }
      if (!names.add(option.toLowerCase())) errors[`option.${index}`] = "候选单位不能重复。";
    });
  }
  if (Object.keys(errors).length > 0) {
    fieldErrors.value = errors;
    notice.warning("请检查单位设置", { detail: Object.values(errors)[0] });
    return;
  }
  emit("save", {
    mode: mode.value,
    fixedUnit: mode.value === "fixed" ? normalizedFixed : "",
    options: mode.value === "select" ? normalizedOptions : [],
  });
}
</script>

<style scoped lang="scss">
.item-unit-settings {
  display: grid;
  gap: 18px;
}

.item-unit-settings__mode {
  min-width: 0;
  margin: 0;
  padding: 0;
  border: 0;
}

.item-unit-settings__mode legend {
  margin-bottom: 7px;
  color: var(--color-text);
  font-size: 13px;
  font-weight: 650;
}

.item-unit-settings__segments {
  display: grid;
  overflow: hidden;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-sm);
  background: var(--color-surface-raised);
}

.item-unit-settings__segments button {
  min-width: 0;
  min-height: 40px;
  padding: 0 8px;
  border: 0;
  border-right: 1px solid var(--color-border-strong);
  background: transparent;
  color: var(--color-muted);
  font-size: 13px;
  font-weight: 650;
}

.item-unit-settings__segments button:last-child {
  border-right: 0;
}

.item-unit-settings__segments button.is-active {
  background: var(--color-accent-soft);
  color: var(--color-accent);
  box-shadow: inset 0 0 0 1px var(--color-focus-ring-soft);
}

.item-unit-settings__segments button:focus-visible {
  position: relative;
  z-index: 1;
  outline: 2px solid var(--color-accent);
  outline-offset: -2px;
}

.item-unit-settings__options {
  display: grid;
  gap: 10px;
}

.item-unit-settings__options-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}

.item-unit-settings__options h3,
.item-unit-settings__options p {
  margin: 0;
}

.item-unit-settings__options h3 {
  font-size: 14px;
}

.item-unit-settings__options h3 span {
  color: var(--color-subtle);
  font-size: 12px;
  font-weight: 500;
}

.item-unit-settings__options p,
.item-unit-settings__empty {
  color: var(--color-muted);
  font-size: 12px;
}

.item-unit-settings__option {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 40px;
  align-items: center;
  gap: 8px;
}

.item-unit-settings__option input {
  width: 100%;
  min-width: 0;
  min-height: 40px;
  padding: 0 11px;
  border: 1px solid var(--color-border-strong);
  border-radius: var(--radius-sm);
  outline: 0;
  background: var(--color-surface);
  color: var(--color-text);
  transition:
    border-color var(--motion-duration-fast) var(--motion-ease-standard),
    box-shadow var(--motion-duration-fast) var(--motion-ease-standard);
}

.item-unit-settings__option input:focus {
  border-color: var(--color-accent);
  box-shadow: 0 0 0 3px var(--color-focus-ring-soft);
}

.item-unit-settings__option input[aria-invalid="true"]:focus {
  border-color: var(--color-danger);
  box-shadow: 0 0 0 3px var(--color-danger-ring);
}

.item-unit-settings__option .icon-button {
  color: var(--color-danger);
}

.item-unit-settings__option svg,
.item-unit-settings__add svg {
  width: 18px;
  height: 18px;
  fill: none;
  stroke: currentcolor;
  stroke-width: 1.8;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.item-unit-settings__empty {
  display: grid;
  min-height: 52px;
  place-items: center;
  border-top: 1px solid var(--color-border);
  border-bottom: 1px solid var(--color-border);
}

@media (max-width: 767px) {
  .item-unit-settings__options-header {
    align-items: center;
    gap: 10px;
  }

  .item-unit-settings__add {
    width: 38px;
    height: 38px;
    min-height: 38px;
    flex: 0 0 auto;
  }
}
</style>
