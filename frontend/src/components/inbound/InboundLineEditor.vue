<!-- 本组件拥有正式入库明细的批次、有效期和模板字段编辑区；它不选择明细或提交整张入库单。 -->
<template>
  <section ref="editor" class="inbound-line-editor inbound-line-editor--drawer" role="dialog" aria-modal="true" aria-labelledby="inbound-line-editor-title" tabindex="-1">
    <header>
      <AuthenticatedImage :file-id="line.item.image_file_id" :alt="`${line.item.name} 主图`" :size="34" previewable />
      <div>
        <strong id="inbound-line-editor-title">批次与属性</strong>
        <span>{{ line.item.name }} · {{ line.item.sku }} · {{ line.item.unit }}</span>
      </div>
      <button class="inbound-line-editor__close" type="button" aria-label="关闭当前明细详情" title="关闭" @click="$emit('close')">
        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path d="M6 6l12 12M18 6 6 18" /></svg>
      </button>
    </header>
    <div class="inbound-line-editor__base-fields">
      <label><span>批次号</span><input v-model="line.batchNo" :name="`batch_no_${line.lineId}`" type="text" maxlength="128" placeholder="留空后由服务端生成" /></label>
      <label><span>有效期</span><input v-model="line.expiresAt" :name="`expires_at_${line.lineId}`" type="date" /></label>
    </div>
    <section class="inbound-template-config" aria-labelledby="inbound-template-config-title">
      <header>
        <strong id="inbound-template-config-title">模板扩展属性</strong>
      </header>
      <label class="inbound-template-picker">
        <span>模板方案</span>
        <SelectControl data-template-picker :name="`template_${line.lineId}`" :model-value="line.templateId ?? ''" @change="emitTemplateSelection">
          <option value="">不使用入库模板</option>
          <option v-for="template in templates" :key="template.id" :value="template.id">{{ template.name }}</option>
        </SelectControl>
      </label>
    </section>
    <section v-if="line.templateLoading" class="inbound-template-state">正在加载入库模板…</section>
    <section v-else-if="line.templateError" class="inbound-template-state inbound-template-state--error" role="alert">
      <span>{{ line.templateError }}</span>
      <button class="text-button" type="button" data-template-retry @click="$emit('retry-template', line)">重试</button>
    </section>
    <section v-else-if="line.template" class="inbound-template-fields">
      <header><strong>{{ line.template.name }}</strong><span>* 必填</span></header>
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
            <option value="">请选择</option><option v-for="option in field.options ?? []" :key="option" :value="option">{{ option }}</option>
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
            <option :value="undefined">请选择</option><option :value="true">是</option><option :value="false">否</option>
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
          <small v-if="fieldInvalid(field)" :id="fieldErrorId(field)" class="visually-hidden" role="alert">{{ fieldTitle(field) }}</small>
        </div>
      </div>
    </section>
    <p v-else class="inbound-line-editor__empty">该物品没有需要填写的模板扩展属性。</p>
    <footer class="inbound-line-editor__footer">
      <button class="primary-button" type="button" @click="$emit('close')">完成</button>
    </footer>
  </section>
</template>

<script setup lang="ts">
import { nextTick, onMounted, ref } from 'vue'
import type { TemplateFieldResponse, TemplateFieldType } from '../../api/inbound'
import type { InboundTemplateResponse } from '../../api/inboundTemplates'
import { fileValue, templateFieldError, type InboundDraftLine } from '../../pages/inbound-draft/model'
import AttributeImageField from '../attributes/AttributeImageField.vue'
import AuthenticatedImage from '../attributes/AuthenticatedImage.vue'
import SelectControl from '../forms/SelectControl.vue'

const props = defineProps<{ line: InboundDraftLine; templates: InboundTemplateResponse[]; validationAttempted: boolean }>()
const emit = defineEmits<{
  close: []
  'retry-template': [line: InboundDraftLine]
  'select-template': [templateId: number | null]
}>()
const editor = ref<HTMLElement | null>(null)

onMounted(() => { void nextTick(() => editor.value?.focus()) })

function emitTemplateSelection(value: unknown): void {
  emit('select-template', value === '' || value === null || value === undefined ? null : Number(value))
}

function fieldInvalid(field: TemplateFieldResponse): boolean {
  return props.validationAttempted && templateFieldError(props.line, field) !== null
}

function fieldTitle(field: TemplateFieldResponse): string | undefined {
  return props.validationAttempted ? templateFieldError(props.line, field) ?? undefined : undefined
}

function fieldControlName(fieldName: string): string {
  return `attribute_${props.line.lineId}_${fieldName.replace(/[^a-zA-Z0-9_-]/g, '_')}`
}

function fieldErrorId(field: TemplateFieldResponse): string {
  return `inbound-field-${props.line.lineId}-${field.id}-error`
}

function inputType(type: TemplateFieldType): string {
  return type === 'number' ? 'number' : type === 'date' ? 'date' : type === 'url' ? 'url' : 'text'
}
</script>
