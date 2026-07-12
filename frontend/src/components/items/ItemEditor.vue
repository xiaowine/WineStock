<!-- 本组件拥有物品基础资料、主图和任意属性的表单布局；它不发起 HTTP 请求。 -->
<template>
  <form class="item-editor" @submit.prevent="emit('save')">
    <header class="item-editor__header">
      <button
        class="icon-button item-editor__back"
        type="button"
        title="返回物品目录"
        aria-label="返回物品目录"
        @click="emit('close')"
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="m15 5-7 7 7 7" />
        </svg>
      </button>
      <div class="item-editor__heading">
        <h2>{{ draft.id ? draft.name || '编辑物品' : '新建物品' }}</h2>
        <p v-if="draft.id">{{ draft.sku }} · {{ draft.unit }}</p>
      </div>
      <button class="primary-button item-editor__desktop-save" type="submit" :disabled="saving">
        {{ saving ? '保存中…' : '保存物品' }}
      </button>
    </header>

    <div class="item-editor__content">
      <div v-if="metadataError" class="item-editor__metadata-error" role="alert">
        分类和属性模板暂不可用，仍可编辑其它字段。
      </div>

      <section class="item-editor__section" aria-labelledby="item-base-heading">
        <header class="item-editor__section-header">
          <h3 id="item-base-heading">基础资料</h3>
        </header>

        <div class="item-editor__base-layout">
          <div class="item-editor__image form-field">
            <span>物品主图 *</span>
            <AttributeImageField
              :model-value="draft.image ?? undefined"
              :delete-on-remove="draft.imageTemporary"
              label="物品主图"
              @update:model-value="updateMainImage"
            />
          </div>

          <div class="item-editor__fields">
            <label class="form-field">
              <span>名称 *</span>
              <input v-model="draft.name" name="name" maxlength="128" required autocomplete="off" />
            </label>
            <label class="form-field">
              <span>SKU *</span>
              <input v-model="draft.sku" name="sku" maxlength="64" required autocomplete="off" />
            </label>
            <label class="form-field">
              <span>分类</span>
              <select v-model="draft.categoryId" name="category">
                <option :value="null">未分类</option>
                <option v-for="category in categories" :key="category.id" :value="category.id">{{ category.name }}</option>
              </select>
            </label>
            <label class="form-field">
              <span>计量单位 *</span>
              <input v-model="draft.unit" name="unit" maxlength="32" required autocomplete="off" />
            </label>
            <label class="form-field">
              <span>参考单价</span>
              <input v-model.number="draft.defaultPrice" name="default_price" type="number" min="0" step="0.01" inputmode="decimal" />
            </label>
            <label class="form-field">
              <span>再订货点</span>
              <input v-model.number="draft.reorderPoint" name="reorder_point" type="number" min="0" step="0.01" inputmode="decimal" />
            </label>
            <label class="form-field item-editor__description">
              <span>描述</span>
              <textarea v-model="draft.description" name="description" maxlength="1024" rows="3" />
            </label>
          </div>
        </div>
      </section>

      <section class="item-editor__section item-editor__attributes" aria-labelledby="item-attributes-heading">
        <header class="item-editor__section-header">
          <h3 id="item-attributes-heading">物品属性</h3>
          <button class="secondary-button item-editor__add-attribute" type="button" @click="addAttribute">
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M12 5v14M5 12h14" />
            </svg>
            添加属性
          </button>
        </header>

        <label class="form-field item-editor__template">
          <span>属性模板</span>
          <select :value="draft.attributeTemplateId ?? ''" name="attribute_template" @change="selectTemplate">
            <option value="">不使用模板</option>
            <option v-for="template in templates" :key="template.id" :value="template.id">{{ template.name }}</option>
          </select>
        </label>

        <div v-if="!draft.attributes.length" class="item-editor__empty-attributes">
          当前没有属性
        </div>
        <div v-else class="item-editor__attribute-list">
          <ItemAttributeEditor
            v-for="(attribute, index) in draft.attributes"
            :key="attribute.key"
            :attribute="attribute"
            @remove="draft.attributes.splice(index, 1)"
          />
        </div>
      </section>
    </div>

    <footer class="item-editor__mobile-actions">
      <button class="primary-button" type="submit" :disabled="saving">{{ saving ? '保存中…' : '保存物品' }}</button>
    </footer>
  </form>
</template>

<script setup lang="ts">
import type { ItemCategoryResponse } from '../../api/itemCategories'
import type { ItemAttributeTemplateResponse } from '../../api/itemAttributeTemplates'
import { applyAttributeTemplate, newCustomAttribute, type ItemDraft } from '../../pages/items/model'
import { discardTemporaryAttributeFile } from '../../pages/items/fileCleanup'
import ItemAttributeEditor from './ItemAttributeEditor.vue'
import AttributeImageField from '../attributes/AttributeImageField.vue'
import type { FileDraftValue } from '../../pages/inbound-draft/model'
import { notice } from '../../notices/notice'

const props = defineProps<{
  draft: ItemDraft
  categories: ItemCategoryResponse[]
  templates: ItemAttributeTemplateResponse[]
  saving: boolean
  metadataError: string
}>()

const emit = defineEmits<{ save: []; close: [] }>()

function addAttribute(): void {
  props.draft.attributes.push(newCustomAttribute())
}

function updateMainImage(value: FileDraftValue | undefined): void {
  if (!props.draft.imageTemporary && props.draft.image?.fileId) {
    props.draft.obsoleteImageFileId = props.draft.image.fileId
  }
  props.draft.image = value ?? null
  props.draft.imageTemporary = true
}

async function selectTemplate(event: Event): Promise<void> {
  const id = Number((event.target as HTMLSelectElement).value)
  const template = props.templates.find((candidate) => candidate.id === id) ?? null
  const nextFields = new Map(template?.fields.map((field) => [field.field_name.toLowerCase(), field.field_type]) ?? [])
  const changingFiles = props.draft.attributes.filter((attribute) =>
    attribute.fieldType === 'file' && nextFields.has(attribute.fieldName.toLowerCase()) &&
    nextFields.get(attribute.fieldName.toLowerCase()) !== 'file')
  try {
    await Promise.all(changingFiles.map(discardTemporaryAttributeFile))
  } catch {
    notice.warning('部分临时图片未能立即删除', { detail: '服务会在超过保留期限后自动清理。' })
  }
  applyAttributeTemplate(props.draft, template)
}
</script>
