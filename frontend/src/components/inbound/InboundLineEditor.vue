<!-- 本组件拥有单条入库明细的完整编辑区；它不选择物品、不提交整张入库单，也不管理路由。 -->
<template>
  <section
    ref="editor"
    class="inbound-line-editor inbound-line-editor--drawer"
    role="dialog"
    aria-modal="true"
    aria-labelledby="inbound-line-editor-title"
    tabindex="-1"
  >
    <header>
      <AuthenticatedImage
        :file-id="line.item.image_file_id"
        :alt="line.item.name + ' 主图'"
        :size="34"
        previewable
      />
      <div>
        <strong id="inbound-line-editor-title">配置入库明细</strong>
        <span>{{ line.item.name }} · {{ line.item.sku }} · {{ line.item.unit }}</span>
      </div>
      <button
        class="icon-button inbound-line-editor__close"
        type="button"
        aria-label="关闭当前入库明细"
        title="关闭"
        @click="$emit('close')"
      >
        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <path d="M6 6l12 12M18 6 6 18" />
        </svg>
      </button>
    </header>

    <section class="inbound-line-editor__operation-fields" aria-label="本次入库参数">
      <label>
        <span>数量 *</span>
        <input
          v-model.number="line.quantity"
          :name="'quantity_' + line.lineId"
          :data-line-id="line.lineId"
          data-field="quantity"
          :class="{ 'inbound-control--error': validationAttempted && !validQuantity(line.quantity) }"
          type="number"
          min="0.01"
          step="0.01"
          inputmode="decimal"
          :aria-label="line.item.name + ' 入库数量'"
        />
      </label>
      <label>
        <span>单价 *</span>
        <input
          v-model.number="line.unitPrice"
          :name="'unit_price_' + line.lineId"
          :data-line-id="line.lineId"
          data-field="unitPrice"
          :class="{ 'inbound-control--error': validationAttempted && !validUnitPrice(line.unitPrice) }"
          type="number"
          min="0"
          step="0.01"
          inputmode="decimal"
          :aria-label="line.item.name + ' 入库单价'"
        />
      </label>
      <label>
        <span>入库库位 *</span>
        <SelectControl
          v-model="line.locationId"
          :name="'location_' + line.lineId"
          :data-line-id="line.lineId"
          data-field="locationId"
          :aria-invalid="validationAttempted && line.locationId === null ? true : undefined"
          :aria-label="line.item.name + ' 入库库位'"
          compact
        >
          <option :value="null">请选择</option>
          <optgroup v-for="group in locationGroups" :key="group.name" :label="group.name">
            <option v-for="location in group.locations" :key="location.id" :value="location.id">
              {{ location.name }}
            </option>
          </optgroup>
        </SelectControl>
      </label>
    </section>

    <div v-if="locationError" class="inbound-location-error" role="alert">
      {{ locationError }}
      <button class="text-button" type="button" @click="$emit('retry-locations')">重试</button>
    </div>

    <div class="inbound-line-editor__base-fields">
      <label>
        <span>批次号</span>
        <input
          v-model="line.batchNo"
          :name="'batch_no_' + line.lineId"
          type="text"
          maxlength="128"
          placeholder="留空后由服务端生成"
        />
      </label>
      <label>
        <span>有效期</span>
        <input
          v-model="line.expiresAt"
          :name="'expires_at_' + line.lineId"
          type="date"
        />
      </label>
    </div>

    <section class="inbound-template-config" aria-labelledby="inbound-template-config-title">
      <header>
        <strong id="inbound-template-config-title">入库模板</strong>
        <span>记录本次收货状态，不会修改物品长期属性。</span>
      </header>
      <div v-if="templatesError" class="inbound-template-options-error" role="alert">
        <span>{{ templatesError }}</span>
        <button
          class="text-button"
          type="button"
          :disabled="templatesLoading"
          @click="$emit('retry-templates')"
        >
          {{ templatesLoading ? "正在重试…" : "重新加载模板" }}
        </button>
      </div>
      <label class="inbound-template-picker">
        <span>模板方案</span>
        <SelectControl
          data-template-picker
          :name="'template_' + line.lineId"
          :model-value="line.templateId ?? ''"
          :disabled="templatesLoading && !templates.length"
          @change="emitTemplateSelection"
        >
          <option value="">不使用入库模板</option>
          <option v-if="unresolvedSelectedTemplate" :value="line.templateId" disabled>
            已删除入库模板 #{{ line.templateId }}
          </option>
          <option v-for="template in templates" :key="template.id" :value="template.id">
            {{ template.name }}
          </option>
        </SelectControl>
      </label>
    </section>

    <section v-if="line.templateState === 'resolving'" class="inbound-template-state">
      正在加载入库模板…
    </section>
    <section
      v-else-if="line.templateError"
      class="inbound-template-state inbound-template-state--error"
      role="alert"
    >
      <span>{{ line.templateError }}</span>
      <button
        v-if="line.templateState === 'error'"
        class="text-button"
        type="button"
        data-template-retry
        @click="$emit('retry-template', line)"
      >
        重试
      </button>
    </section>
    <section v-else-if="line.template" class="inbound-template-fields">
      <header>
        <strong>{{ line.template.name }}</strong>
        <span>* 必填</span>
      </header>
      <div>
        <div
          v-for="field in line.template.fields"
          :key="field.id"
          class="inbound-template-field"
          :class="{ 'inbound-template-field--wide': field.field_type === 'file' }"
        >
          <span>{{ field.field_name }}<template v-if="field.required"> *</template></span>
          <AttributeImageField
            v-if="field.field_type === 'file'"
            :model-value="fileValue(line, field.field_name)"
            :label="field.field_name"
            :invalid="fieldInvalid(field)"
            :title="fieldTitle(field)"
            :aria-describedby="fieldInvalid(field) ? fieldErrorId(field) : undefined"
            :data-template-field="field.field_name"
            @update:model-value="line.extAttributes[field.field_name] = $event"
          />
          <SelectControl
            v-else-if="field.field_type === 'select'"
            v-model="line.extAttributes[field.field_name]"
            :name="fieldControlName(field.field_name)"
            :aria-label="field.field_name"
            :data-template-field="field.field_name"
            :aria-invalid="fieldInvalid(field) || undefined"
            :aria-describedby="fieldInvalid(field) ? fieldErrorId(field) : undefined"
            :title="fieldTitle(field)"
          >
            <option value="">请选择</option>
            <option v-for="option in field.options ?? []" :key="option" :value="option">
              {{ option }}
            </option>
          </SelectControl>
          <SelectControl
            v-else-if="field.field_type === 'boolean'"
            v-model="line.extAttributes[field.field_name]"
            :name="fieldControlName(field.field_name)"
            :aria-label="field.field_name"
            :data-template-field="field.field_name"
            :aria-invalid="fieldInvalid(field) || undefined"
            :aria-describedby="fieldInvalid(field) ? fieldErrorId(field) : undefined"
            :title="fieldTitle(field)"
          >
            <option :value="undefined">请选择</option>
            <option :value="true">是</option>
            <option :value="false">否</option>
          </SelectControl>
          <input
            v-else
            v-model="line.extAttributes[field.field_name]"
            :name="fieldControlName(field.field_name)"
            :aria-label="field.field_name"
            :data-template-field="field.field_name"
            :class="{ 'inbound-control--error': fieldInvalid(field) }"
            :aria-invalid="fieldInvalid(field) || undefined"
            :aria-describedby="fieldInvalid(field) ? fieldErrorId(field) : undefined"
            :type="inputType(field.field_type)"
            :placeholder="field.default_value ?? undefined"
            :title="fieldTitle(field)"
          />
          <small
            v-if="fieldInvalid(field)"
            :id="fieldErrorId(field)"
            class="visually-hidden"
            role="alert"
            >{{ fieldTitle(field) }}</small
          >
        </div>
      </div>
    </section>
    <p v-else class="inbound-line-editor__empty">
      当前未使用入库模板，没有需要填写的本次收货属性。
    </p>

    <footer class="inbound-line-editor__footer">
      <button class="secondary-button" type="button" @click="$emit('close')">暂存并关闭</button>
      <button class="primary-button" type="button" @click="$emit('complete-and-continue')">
        完成并继续添加
      </button>
    </footer>
  </section>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from "vue";
import type { TemplateFieldResponse, TemplateFieldType } from "../../api/inbound";
import type { InboundTemplateResponse } from "../../api/inboundTemplates";
import type { LocationResponse } from "../../api/locations";
import {
  fileValue,
  templateFieldError,
  validQuantity,
  validUnitPrice,
  type InboundDraftLine,
} from "../../pages/inbound-draft/model";
import AttributeImageField from "../attributes/AttributeImageField.vue";
import AuthenticatedImage from "../attributes/AuthenticatedImage.vue";
import SelectControl from "../forms/SelectControl.vue";

const props = defineProps<{
  line: InboundDraftLine;
  locations: LocationResponse[];
  locationError: string;
  templates: InboundTemplateResponse[];
  templatesLoading: boolean;
  templatesError: string;
  validationAttempted: boolean;
}>();

const emit = defineEmits<{
  close: [];
  "complete-and-continue": [];
  "retry-template": [line: InboundDraftLine];
  "retry-templates": [];
  "retry-locations": [];
  "select-template": [templateId: number | null];
}>();

const editor = ref<HTMLElement | null>(null);
const unresolvedSelectedTemplate = computed(
  () => props.line.templateState === "unresolved" && props.line.templateId !== null,
);
const locationGroups = computed(() => {
  const groups = new Map<string, LocationResponse[]>();
  for (const location of props.locations) {
    const list = groups.get(location.group_name) ?? [];
    list.push(location);
    groups.set(location.group_name, list);
  }
  return Array.from(groups, ([name, locations]) => ({ name, locations }));
});

onMounted(() => {
  void nextTick(() => editor.value?.focus());
});

function emitTemplateSelection(value: unknown): void {
  emit(
    "select-template",
    value === "" || value === null || value === undefined ? null : Number(value),
  );
}

function fieldInvalid(field: TemplateFieldResponse): boolean {
  return props.validationAttempted && templateFieldError(props.line, field) !== null;
}

function fieldTitle(field: TemplateFieldResponse): string | undefined {
  return props.validationAttempted
    ? (templateFieldError(props.line, field) ?? undefined)
    : undefined;
}

function fieldControlName(fieldName: string): string {
  return "attribute_" + props.line.lineId + "_" + fieldName.replace(/[^a-zA-Z0-9_-]/g, "_");
}

function fieldErrorId(field: TemplateFieldResponse): string {
  return "inbound-field-" + props.line.lineId + "-" + field.id + "-error";
}

function inputType(type: TemplateFieldType): string {
  return type === "number" ? "number" : type === "date" ? "date" : type === "url" ? "url" : "text";
}
</script>
