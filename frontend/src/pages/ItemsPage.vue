<!-- 本文件拥有物品管理页面编排，负责加载列表、分类、属性模板并保存物品；字段布局由 items 组件承担。 -->
<template>
  <section class="route-page items-page">
    <header class="content-header"><div><h1>物品</h1><p>分类负责归类，属性模板仅用于快速生成常用字段。</p></div><button class="secondary-button" type="button" @click="startNew">新建物品</button></header>
    <div class="items-page__workspace">
      <aside class="items-page__list">
        <label><span>搜索物品</span><input v-model="search" type="search" placeholder="名称、SKU 或属性" @input="reloadSoon" /></label>
        <div v-if="loading" class="items-page__state">正在加载物品…</div>
        <div v-else-if="loadError" class="items-page__state items-page__state--error">{{ loadError }}<button class="text-button" type="button" @click="loadAll">重试</button></div>
        <button v-for="item in items" :key="item.id" class="items-page__item" :class="{ 'items-page__item--selected': draft.id === item.id }" type="button" @click="editItem(item)"><strong>{{ item.name }}</strong><span>{{ item.sku }} · {{ item.unit }}</span><small>{{ item.attributes.length }} 个属性</small></button>
        <p v-if="!loading && !loadError && !items.length" class="items-page__state">暂无物品。</p>
      </aside>
      <ItemEditor :draft="draft" :categories="categories" :templates="templates" :saving="saving" @save="save" />
    </div>
  </section>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { createItem, listItems, updateItem, type ItemCreateRequest, type ItemResponse, type ItemUpdateRequest } from '../api/items'
import { listItemCategories, type ItemCategoryResponse } from '../api/itemCategories'
import { listItemAttributeTemplates, type ItemAttributeTemplateResponse } from '../api/itemAttributeTemplates'
import ItemEditor from '../components/items/ItemEditor.vue'
import { ApiError } from '../api/errors'
import { notice } from '../notices/notice'
import { draftFromItem, emptyItemDraft, itemAttributeRequests, type ItemDraft } from './items/model'
import { discardTemporaryItemFiles } from './items/fileCleanup'
import './ItemsPage.scss'

const items = ref<ItemResponse[]>([])
const categories = ref<ItemCategoryResponse[]>([])
const templates = ref<ItemAttributeTemplateResponse[]>([])
const draft = ref<ItemDraft>(emptyItemDraft())
const search = ref('')
const loading = ref(false)
const saving = ref(false)
const loadError = ref('')
let searchTimer: number | undefined

onMounted(() => void loadAll())
onBeforeUnmount(() => {
  window.clearTimeout(searchTimer)
  void discardCurrentTemporaryFiles()
})

async function loadAll(): Promise<void> {
  loading.value = true; loadError.value = ''
  try {
    const [itemPage, nextCategories, nextTemplates] = await Promise.all([listItems(search.value, 1, 200), listItemCategories(), listItemAttributeTemplates()])
    items.value = itemPage.items; categories.value = nextCategories; templates.value = nextTemplates
  } catch (error) { loadError.value = errorMessage(error) } finally { loading.value = false }
}

function reloadSoon(): void { window.clearTimeout(searchTimer); searchTimer = window.setTimeout(() => void loadAll(), 250) }
async function startNew(): Promise<void> {
  await discardCurrentTemporaryFiles()
  draft.value = emptyItemDraft()
}

async function editItem(item: ItemResponse): Promise<void> {
  await discardCurrentTemporaryFiles()
  draft.value = draftFromItem(item)
}

async function save(): Promise<void> {
  if (!draft.value.name.trim() || !draft.value.sku.trim() || !draft.value.unit.trim()) { notice.warning('请填写名称、SKU 和计量单位'); return }
  if (draft.value.attributes.some((attribute) => attribute.fieldType === 'file' && (typeof attribute.value !== 'object' || !attribute.value?.fileId || attribute.value.status !== 'uploaded'))) { notice.warning('请等待所有物品图片上传完成'); return }
  saving.value = true
  const baseRequest = {
    name: draft.value.name.trim(), sku: draft.value.sku.trim(), unit: draft.value.unit.trim(),
    attributes: itemAttributeRequests(draft.value),
  }
  try {
    const saved = draft.value.id
      ? await updateItem(draft.value.id, updateRequest(baseRequest))
      : await createItem(createRequest(baseRequest))
    draft.value.attributes.forEach((attribute) => { attribute.fileTemporary = false })
    notice.success(draft.value.id ? '物品已更新' : '物品已创建')
    await loadAll(); draft.value = draftFromItem(saved)
  } catch (error) { notice.error('保存物品失败', { detail: errorMessage(error) }) } finally { saving.value = false }
}

function errorMessage(error: unknown): string { return error instanceof ApiError ? error.message : '无法连接到 WineStock 服务' }

async function discardCurrentTemporaryFiles(): Promise<void> {
  try { await discardTemporaryItemFiles(draft.value) }
  catch { notice.warning('部分临时图片未能立即删除', { detail: '服务会在超过保留期限后自动清理。' }) }
}

function createRequest(base: Pick<ItemCreateRequest, 'name' | 'sku' | 'unit' | 'attributes'>): ItemCreateRequest {
  return {
    ...base,
    category_id: draft.value.categoryId ?? undefined,
    attribute_template_id: draft.value.attributeTemplateId ?? undefined,
    description: draft.value.description.trim() || undefined,
    default_price: draft.value.defaultPrice ?? undefined,
    reorder_point: draft.value.reorderPoint ?? undefined,
  }
}

function updateRequest(base: Pick<ItemUpdateRequest, 'name' | 'sku' | 'unit' | 'attributes'>): ItemUpdateRequest {
  return {
    ...base,
    category_id: draft.value.categoryId,
    attribute_template_id: draft.value.attributeTemplateId,
    description: draft.value.description.trim() || null,
    default_price: draft.value.defaultPrice,
    reorder_point: draft.value.reorderPoint,
  }
}
</script>
