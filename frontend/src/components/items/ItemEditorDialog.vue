<!--
  本组件拥有物品编辑器在通用 Dialog 中的组合方式，属于 frontend 共享组件层。
  它不加载元数据、不保存物品，也不决定未保存草稿的处理策略。
-->
<template>
  <ModalDialog
    :open="open"
    :title="title"
    :description="description"
    :busy="saving"
    wide
    @close="emit('close')"
  >
    <ItemEditor
      :draft="draft"
      :categories="categories"
      :templates="templates"
      :saving="saving"
      :metadata-error="metadataError"
      :form-id="formId"
      embedded
      @save="emit('save')"
    />

    <template #actions>
      <button class="secondary-button" type="button" :disabled="saving" @click="emit('close')">取消</button>
      <button class="primary-button" type="submit" :form="formId" :disabled="saving">
        {{ saving ? '保存中…' : '保存物品' }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { computed, useId } from 'vue'
import type { ItemCategoryResponse } from '../../api/itemCategories'
import type { ItemAttributeTemplateResponse } from '../../api/itemAttributeTemplates'
import type { ItemDraft } from '../../pages/items/model'
import ModalDialog from '../ModalDialog.vue'
import ItemEditor from './ItemEditor.vue'

const props = defineProps<{
  open: boolean
  draft: ItemDraft
  categories: ItemCategoryResponse[]
  templates: ItemAttributeTemplateResponse[]
  saving: boolean
  metadataError: string
}>()

const emit = defineEmits<{ save: []; close: [] }>()
const formId = `item-editor-${useId()}`
const title = computed(() => props.draft.id ? '编辑物品' : '新建物品')
const description = computed(() => props.draft.id
  ? [props.draft.name, props.draft.sku, props.draft.unit].filter(Boolean).join(' · ')
  : undefined)
</script>
