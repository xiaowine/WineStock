<!-- 本组件拥有当前入库明细的批次、有效期和模板字段编辑区；它不选择明细或提交整张入库单。 -->
<template>
  <section class="inbound-line-editor" aria-label="当前明细详情">
    <header>
      <div><span>当前明细</span><strong>{{ line.item.name }}</strong></div>
      <small>{{ line.item.sku }}</small>
    </header>
    <div class="inbound-line-editor__base-fields">
      <label><span>批次号</span><input v-model="line.batchNo" type="text" maxlength="128" placeholder="留空后由服务端生成" /></label>
      <label><span>有效期</span><input v-model="line.expiresAt" type="date" /></label>
    </div>
    <section v-if="line.item.attributes.length" class="inbound-item-attributes">
      <header><strong>物品属性</strong><span>只读，修改请前往物品页面</span></header>
      <dl>
        <div v-for="attribute in line.item.attributes" :key="attribute.id">
          <dt>{{ attribute.field_name }}</dt>
          <dd>{{ formatItemAttribute(attribute.value) }}<small v-if="attribute.unit"> {{ attribute.unit }}</small></dd>
        </div>
      </dl>
    </section>
    <label class="inbound-template-picker">
      <span>入库模板</span>
      <select data-template-picker :value="line.templateId ?? ''" @change="emitTemplateSelection">
        <option value="">不使用入库模板</option>
        <option v-for="template in templates" :key="template.id" :value="template.id">{{ template.name }}</option>
      </select>
    </label>
    <section v-if="line.templateLoading" class="inbound-template-state">正在加载物品模板…</section>
    <section v-else-if="line.templateError" class="inbound-template-state inbound-template-state--error" role="alert">
      <span>{{ line.templateError }}</span>
      <button class="text-button" type="button" data-template-retry @click="$emit('retry-template', line)">重试</button>
    </section>
    <section v-else-if="line.template" class="inbound-template-fields">
      <header><strong>{{ line.template.name }}</strong><span>提交与审批时校验</span></header>
      <div>
        <label v-for="field in line.template.fields" :key="field.id">
          <span>{{ field.field_name }}<template v-if="field.required"> *</template></span>
          <AttributeImageField
            v-if="field.field_type === 'file'"
            :model-value="fileValue(line, field.field_name)"
            :invalid="fieldInvalid(field)"
            :title="fieldTitle(field)"
            :data-template-field="field.field_name"
            @update:model-value="line.extAttributes[field.field_name] = $event"
          />
          <select
            v-else-if="field.field_type === 'select'"
            v-model="line.extAttributes[field.field_name]"
            :data-template-field="field.field_name"
            :class="{ 'inbound-control--error': fieldInvalid(field) }"
            :title="fieldTitle(field)"
          >
            <option value="">请选择</option><option v-for="option in field.options ?? []" :key="option" :value="option">{{ option }}</option>
          </select>
          <select
            v-else-if="field.field_type === 'boolean'"
            v-model="line.extAttributes[field.field_name]"
            :data-template-field="field.field_name"
            :class="{ 'inbound-control--error': fieldInvalid(field) }"
            :title="fieldTitle(field)"
          >
            <option :value="undefined">请选择</option><option :value="true">是</option><option :value="false">否</option>
          </select>
          <input
            v-else
            v-model="line.extAttributes[field.field_name]"
            :data-template-field="field.field_name"
            :class="{ 'inbound-control--error': fieldInvalid(field) }"
            :type="inputType(field.field_type)"
            :placeholder="field.default_value ?? undefined"
            :title="fieldTitle(field)"
          />
        </label>
      </div>
    </section>
    <p v-else class="inbound-line-editor__empty">该物品没有需要填写的模板扩展属性。</p>
  </section>
</template>

<script setup lang="ts">
import type { TemplateFieldResponse, TemplateFieldType } from '../../api/inbound'
import type { InboundTemplateResponse } from '../../api/inboundTemplates'
import { fileValue, templateFieldError, type InboundDraftLine } from '../../pages/inbound-draft/model'
import AttributeImageField from '../attributes/AttributeImageField.vue'

const props = defineProps<{ line: InboundDraftLine; templates: InboundTemplateResponse[]; validationAttempted: boolean }>()
const emit = defineEmits<{ 'retry-template': [line: InboundDraftLine]; 'select-template': [templateId: number | null] }>()

function emitTemplateSelection(event: Event): void {
  const value = (event.target as HTMLSelectElement).value
  emit('select-template', value ? Number(value) : null)
}

function formatItemAttribute(value: unknown): string {
  if (typeof value === 'object' && value !== null && 'file_id' in value) return `图片 #${String((value as { file_id: number }).file_id)}`
  if (typeof value === 'boolean') return value ? '是' : '否'
  return String(value)
}

function fieldInvalid(field: TemplateFieldResponse): boolean {
  return props.validationAttempted && templateFieldError(props.line, field) !== null
}

function fieldTitle(field: TemplateFieldResponse): string | undefined {
  return props.validationAttempted ? templateFieldError(props.line, field) ?? undefined : undefined
}

function inputType(type: TemplateFieldType): string {
  return type === 'number' ? 'number' : type === 'date' ? 'date' : type === 'url' ? 'url' : 'text'
}
</script>
