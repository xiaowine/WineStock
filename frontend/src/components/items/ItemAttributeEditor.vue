<!-- 本组件拥有单个物品属性草稿的类型化字段布局；它不保存物品或选择模板。 -->
<template>
  <div
    class="item-attribute-editor"
    :class="{
      'item-attribute-editor--template': templateField,
    }"
  >
    <label v-if="!templateField" class="form-field item-attribute-editor__name">
      <span>属性名称</span>
      <input
        v-model="attribute.fieldName"
        :name="`attribute_name_${attribute.key}`"
        :readonly="attribute.templateFieldId !== null"
        maxlength="64"
        placeholder="例如：产地"
      />
    </label>
    <label v-if="!templateField" class="form-field item-attribute-editor__type">
      <span>类型</span>
      <select v-model="attribute.fieldType" :name="`attribute_type_${attribute.key}`" :disabled="attribute.templateFieldId !== null" @change="resetValue">
        <option value="text">文本</option>
        <option value="number">数字</option>
        <option value="select">选项</option>
        <option value="date">日期</option>
        <option value="url">网址</option>
        <option value="boolean">布尔</option>
        <option value="file">图片</option>
      </select>
    </label>
    <div class="form-field item-attribute-editor__value">
      <span>{{ templateField?.field_name ?? '属性值' }}{{ templateField?.required ? ' *' : '' }}</span>
      <div class="item-attribute-editor__value-control">
        <AttributeImageField
          v-if="attribute.fieldType === 'file'"
          :model-value="fileValue"
          :delete-on-remove="attribute.fileTemporary"
          :label="templateField?.field_name ?? (attribute.fieldName || '属性图片')"
          @update:model-value="updateFile"
        />
        <select
          v-else-if="attribute.fieldType === 'boolean'"
          v-model="attribute.value"
          :name="`attribute_value_${attribute.key}`"
          :required="templateField?.required"
        >
          <option :value="undefined">请选择</option>
          <option :value="true">是</option>
          <option :value="false">否</option>
        </select>
        <select
          v-else-if="attribute.fieldType === 'select' && templateField?.options"
          v-model="attribute.value"
          :name="`attribute_value_${attribute.key}`"
          :required="templateField.required"
        >
          <option value="">请选择</option>
          <option v-for="option in templateField.options" :key="option" :value="option">{{ option }}</option>
        </select>
        <input
          v-else
          v-model="attribute.value"
          :name="`attribute_value_${attribute.key}`"
          :type="inputType"
          :required="templateField?.required"
          :pattern="attribute.fieldType === 'url' ? 'https?://.+' : undefined"
          placeholder="输入属性值"
        />
        <span v-if="unitMode === 'fixed'" class="item-attribute-editor__fixed-unit">{{ templateField?.unit.value }}</span>
      </div>
    </div>
    <label v-if="!templateField || unitMode === 'custom'" class="form-field item-attribute-editor__unit">
      <span>单位</span>
      <input v-model="attribute.unit" :name="`attribute_unit_${attribute.key}`" maxlength="32" placeholder="可选" />
    </label>
    <label v-else-if="unitMode === 'select'" class="form-field item-attribute-editor__unit">
      <span>单位 *</span>
      <select v-model="attribute.unit" :name="`attribute_unit_${attribute.key}`" required>
        <option value="">请选择</option>
        <option v-for="option in templateField?.unit.options ?? []" :key="option" :value="option">{{ option }}</option>
      </select>
    </label>
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
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import AttributeImageField from '../attributes/AttributeImageField.vue'
import type { ItemAttributeTemplateFieldResponse } from '../../api/itemAttributeTemplates'
import type { FileDraftValue } from '../../pages/inbound-draft/model'
import type { ItemAttributeDraft } from '../../pages/items/model'
import { discardTemporaryAttributeFile } from '../../pages/items/fileCleanup'
import { notice } from '../../notices/notice'

const props = defineProps<{ attribute: ItemAttributeDraft; templateField?: ItemAttributeTemplateFieldResponse }>()
const emit = defineEmits<{ remove: [] }>()
const fileValue = computed(() => typeof props.attribute.value === 'object' && props.attribute.value?.kind === 'file' ? props.attribute.value : undefined)
const inputType = computed(() => props.attribute.fieldType === 'number' ? 'number' : props.attribute.fieldType === 'date' ? 'date' : props.attribute.fieldType === 'url' ? 'url' : 'text')
const unitMode = computed(() => props.templateField?.unit.mode ?? 'none')

function updateFile(value: FileDraftValue | undefined): void {
  props.attribute.value = value
  props.attribute.fileTemporary = true
}

async function removeAttribute(): Promise<void> {
  await discardTemporaryFile()
  emit('remove')
}

async function resetValue(): Promise<void> {
  await discardTemporaryFile()
  props.attribute.value = props.attribute.fieldType === 'boolean' ? undefined : ''
  props.attribute.unit = ''
}

async function discardTemporaryFile(): Promise<void> {
  try {
    await discardTemporaryAttributeFile(props.attribute)
  } catch {
    notice.warning('临时图片未能立即删除', { detail: '服务会在超过保留期限后自动清理。' })
  }
}
</script>
