<!-- 本组件拥有单个物品属性草稿的类型化编辑控件；它不保存物品或选择模板。 -->
<template>
  <div class="item-attribute-editor">
    <input v-model="attribute.fieldName" :readonly="attribute.templateFieldId !== null" maxlength="64" placeholder="属性名称" />
    <select v-model="attribute.fieldType" :disabled="attribute.templateFieldId !== null" @change="resetValue">
      <option value="text">文本</option><option value="number">数字</option><option value="select">选项</option>
      <option value="date">日期</option><option value="url">网址</option><option value="boolean">布尔</option><option value="file">图片</option>
    </select>
    <AttributeImageField v-if="attribute.fieldType === 'file'" :model-value="fileValue" :delete-on-remove="attribute.fileTemporary" @update:model-value="updateFile" />
    <select v-else-if="attribute.fieldType === 'boolean'" v-model="attribute.value"><option :value="undefined">请选择</option><option :value="true">是</option><option :value="false">否</option></select>
    <input v-else v-model="attribute.value" :type="inputType" placeholder="属性值" />
    <input v-model="attribute.unit" maxlength="32" placeholder="单位（可选）" />
    <button class="text-button" type="button" @click="removeAttribute">删除</button>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import AttributeImageField from '../attributes/AttributeImageField.vue'
import type { FileDraftValue } from '../../pages/inbound-draft/model'
import type { ItemAttributeDraft } from '../../pages/items/model'
import { discardTemporaryAttributeFile } from '../../pages/items/fileCleanup'
import { notice } from '../../notices/notice'

const props = defineProps<{ attribute: ItemAttributeDraft }>()
const emit = defineEmits<{ remove: [] }>()
const fileValue = computed(() => typeof props.attribute.value === 'object' && props.attribute.value?.kind === 'file' ? props.attribute.value : undefined)
const inputType = computed(() => props.attribute.fieldType === 'number' ? 'number' : props.attribute.fieldType === 'date' ? 'date' : props.attribute.fieldType === 'url' ? 'url' : 'text')
function updateFile(value: FileDraftValue | undefined): void { props.attribute.value = value; props.attribute.fileTemporary = true }
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
  try { await discardTemporaryAttributeFile(props.attribute) }
  catch { notice.warning('临时图片未能立即删除', { detail: '服务会在超过保留期限后自动清理。' }) }
}
</script>
