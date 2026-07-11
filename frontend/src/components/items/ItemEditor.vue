<!-- 本组件拥有物品基础资料、可选属性模板和任意属性编辑布局；它不发起 HTTP 请求。 -->
<template>
  <form class="item-editor" @submit.prevent="$emit('save')">
    <header><div><h2>{{ draft.id ? '编辑物品' : '新建物品' }}</h2><p>属性模板是可选预设，自定义字段不会被模板限制。</p></div><button class="primary-button" type="submit" :disabled="saving">{{ saving ? '保存中…' : '保存物品' }}</button></header>
    <div class="item-editor__base">
      <label><span>名称 *</span><input v-model="draft.name" maxlength="128" required /></label>
      <label><span>SKU *</span><input v-model="draft.sku" maxlength="64" required /></label>
      <label><span>分类</span><select v-model="draft.categoryId"><option :value="null">未分类</option><option v-for="category in categories" :key="category.id" :value="category.id">{{ category.name }}</option></select></label>
      <label><span>属性模板</span><select :value="draft.attributeTemplateId ?? ''" @change="selectTemplate"><option value="">不使用模板</option><option v-for="template in templates" :key="template.id" :value="template.id">{{ template.name }}</option></select></label>
      <label><span>计量单位 *</span><input v-model="draft.unit" maxlength="32" required /></label>
      <label><span>参考单价</span><input v-model.number="draft.defaultPrice" type="number" min="0" step="0.01" /></label>
      <label><span>再订货点</span><input v-model.number="draft.reorderPoint" type="number" min="0" step="0.01" /></label>
      <label class="item-editor__description"><span>描述</span><input v-model="draft.description" maxlength="1024" /></label>
    </div>
    <section class="item-editor__attributes"><header><div><strong>物品属性</strong><span>型号、参数、图片等长期资料</span></div><button class="secondary-button" type="button" @click="draft.attributes.push(newCustomAttribute())">添加自定义属性</button></header>
      <p v-if="!draft.attributes.length">当前没有属性，可选择模板或添加自定义字段。</p>
      <ItemAttributeEditor v-for="(attribute, index) in draft.attributes" :key="attribute.key" :attribute="attribute" @remove="draft.attributes.splice(index, 1)" />
    </section>
  </form>
</template>

<script setup lang="ts">
import type { ItemCategoryResponse } from '../../api/itemCategories'
import type { ItemAttributeTemplateResponse } from '../../api/itemAttributeTemplates'
import { applyAttributeTemplate, newCustomAttribute, type ItemDraft } from '../../pages/items/model'
import { discardTemporaryAttributeFile } from '../../pages/items/fileCleanup'
import ItemAttributeEditor from './ItemAttributeEditor.vue'
import { notice } from '../../notices/notice'

const props = defineProps<{ draft: ItemDraft; categories: ItemCategoryResponse[]; templates: ItemAttributeTemplateResponse[]; saving: boolean }>()
defineEmits<{ save: [] }>()
async function selectTemplate(event: Event): Promise<void> {
  const id = Number((event.target as HTMLSelectElement).value)
  const template = props.templates.find((candidate) => candidate.id === id) ?? null
  const nextFields = new Map(template?.fields.map((field) => [field.field_name.toLowerCase(), field.field_type]) ?? [])
  const changingFiles = props.draft.attributes.filter((attribute) =>
    attribute.fieldType === 'file' && nextFields.has(attribute.fieldName.toLowerCase()) &&
    nextFields.get(attribute.fieldName.toLowerCase()) !== 'file')
  try { await Promise.all(changingFiles.map(discardTemporaryAttributeFile)) }
  catch { notice.warning('部分临时图片未能立即删除', { detail: '服务会在超过保留期限后自动清理。' }) }
  applyAttributeTemplate(props.draft, template)
}
</script>
