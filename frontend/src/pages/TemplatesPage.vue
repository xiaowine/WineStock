<!--
  本文件拥有分类与模板页面的三业务域状态、权限入口和 CRUD 请求编排。
  它不编辑具体物品或入库记录，也不虚构服务端未提供的引用数量。
-->
<template>
  <section class="route-page templates-page">
    <header class="content-header templates-page__header">
      <div>
        <h1>分类与模板</h1>
        <p>维护物品归类、长期属性结构与单次入库字段。</p>
      </div>
    </header>

    <section class="templates-workspace">
      <div class="templates-tabs" role="tablist" aria-label="分类与模板业务域" @keydown="handleTabKeydown">
        <button
          v-for="domain in domains"
          :id="`templates-tab-${domain.value}`"
          :key="domain.value"
          type="button"
          role="tab"
          :aria-selected="activeDomain === domain.value"
          :aria-controls="`templates-panel-${domain.value}`"
          :tabindex="activeDomain === domain.value ? 0 : -1"
          @click="selectDomain(domain.value)"
        >
          {{ domain.label }}
        </button>
      </div>

      <div class="templates-panels">
        <Transition :name="domainTransitionName">
          <div
            :id="`templates-panel-${activeDomain}`"
            :key="activeDomain"
            class="templates-panel"
            role="tabpanel"
            :aria-labelledby="`templates-tab-${activeDomain}`"
          >
        <div class="templates-toolbar">
          <SearchField
            :model-value="searchInputs[activeDomain]"
            class="templates-toolbar__search"
            :label="searchLabel"
            :name="`template_search_${activeDomain}`"
            :placeholder="searchPlaceholder"
            hide-label
            :disabled="currentState.loading && !currentState.loaded"
            @update:model-value="searchInputs[activeDomain] = $event"
            @search="applySearch"
          />
          <div class="templates-toolbar__commands">
            <div class="templates-toolbar__summary">
              <span>{{ filteredCount }} {{ domainCountLabel }}</span>
              <span v-if="showRefreshing" role="status">正在刷新</span>
            </div>
            <div class="templates-toolbar__actions">
              <button
                class="icon-button"
                :class="{ 'templates-toolbar__refresh--pending': showRefreshing }"
                type="button"
                :title="`刷新${activeDomainLabel}`"
                :aria-label="`刷新${activeDomainLabel}`"
                :aria-busy="currentState.loading"
                :disabled="currentState.loading"
                @click="refreshCurrent"
              >
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M20 7v5h-5"/><path d="M18.2 16a7 7 0 1 1 .8-7l1 3"/></svg>
              </button>
              <button
                v-if="canManage"
                class="icon-button icon-button--primary"
                type="button"
                :title="`新建${activeDomainLabel}`"
                :aria-label="`新建${activeDomainLabel}`"
                @click="openCreate"
              >
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14"/></svg>
              </button>
            </div>
          </div>
        </div>

        <div class="templates-list-stage" :aria-busy="currentState.loading">
          <div v-if="currentState.error && !currentState.loaded" class="templates-state templates-state--error" role="alert">
            <strong>{{ currentState.error }}</strong>
            <button class="secondary-button" type="button" @click="loadDomain(activeDomain)">重试</button>
          </div>
          <div v-else-if="showInitialLoading && !currentState.loaded" class="templates-state" role="status">正在加载{{ activeDomainLabel }}…</div>
          <div v-else-if="filteredCount === 0" class="templates-state">
            <strong>{{ currentSearch ? '没有符合条件的结果' : `还没有${activeDomainLabel}` }}</strong>
            <span>{{ currentSearch ? '可以清除搜索查看全部内容。' : canManage ? `可从工具栏新建${activeDomainLabel}。` : '当前没有可查看的数据。' }}</span>
            <button v-if="currentSearch" class="text-button" type="button" @click="clearSearch">清除搜索</button>
          </div>
          <template v-else>
            <p v-if="currentState.error" class="templates-list-stage__inline-error" role="alert">{{ currentState.error }}</p>

            <div v-if="activeDomain === 'category'" class="templates-table templates-table--category" role="table" aria-label="物品分类列表">
              <div class="templates-table__head" role="row"><span>分类</span><span>说明</span><span>排序与操作</span></div>
              <article v-for="category in filteredCategories" :key="category.id" class="templates-table__row" role="row">
                <div class="templates-table__identity" role="cell"><strong>{{ category.name }}</strong><span>分类 #{{ category.id }}</span></div>
                <div class="templates-table__description" role="cell" :title="category.description ?? undefined">{{ category.description || '暂无说明' }}</div>
                <div class="templates-table__decision" role="cell">
                  <span class="templates-table__meta"><span>排序 <strong>{{ category.sort_order }}</strong></span><span>更新 <time :datetime="category.updated_at" :title="formatFullDateTime(category.updated_at)">{{ formatTime(category.updated_at) }}</time></span></span>
                  <span v-if="canManage" class="templates-table__actions">
                    <button class="icon-button" type="button" title="编辑分类" :aria-label="`编辑分类 ${category.name}`" @click="openCategory(category)"><EditIcon /></button>
                    <button class="icon-button templates-table__delete" type="button" title="删除分类" :aria-label="`删除分类 ${category.name}`" @click="openDelete('category', category)"><DeleteIcon /></button>
                  </span>
                </div>
              </article>
            </div>

            <div v-else-if="activeDomain === 'item'" class="templates-table templates-table--template" role="table" aria-label="物品属性模板列表">
              <div class="templates-table__head" role="row"><span>物品模板</span><span>字段与默认项</span><span>更新与操作</span></div>
              <article v-for="template in filteredItemTemplates" :key="template.id" class="templates-table__row" role="row">
                <button class="templates-table__identity templates-table__identity--button" type="button" role="cell" @click="openTemplate('item', template, true)"><strong>{{ template.name }}</strong><span>物品模板 #{{ template.id }}</span><small>{{ template.description || '暂无说明' }}</small></button>
                <div class="templates-table__information" role="cell">
                  <span class="templates-table__metrics"><span>字段 <strong>{{ template.fields.length }}</strong></span><span>必填 <strong>{{ countRequired(template.fields) }}</strong></span><span>可筛选 <strong>{{ countSearchable(template.fields) }}</strong></span><span>目录 <strong>{{ countCatalogVisible(template) }}/3</strong></span></span>
                  <span class="templates-table__default" :class="{ 'templates-table__default--warning': isUnresolvedDefault(template) }">默认入库模板 <strong>{{ defaultInboundLabel(template.default_inbound_template_id) }}</strong></span>
                </div>
                <div class="templates-table__decision" role="cell">
                  <time :datetime="template.updated_at" :title="formatFullDateTime(template.updated_at)">{{ formatTime(template.updated_at) }}</time>
                  <span class="templates-table__actions">
                    <button class="icon-button" type="button" title="查看模板" :aria-label="`查看模板 ${template.name}`" @click="openTemplate('item', template, true)"><ViewIcon /></button>
                    <template v-if="canManage">
                      <button class="icon-button" type="button" title="编辑模板" :aria-label="`编辑模板 ${template.name}`" @click="openTemplate('item', template, false)"><EditIcon /></button>
                      <button class="icon-button" type="button" title="复制模板" :aria-label="`复制模板 ${template.name}`" @click="openCopy('item', template)"><CopyIcon /></button>
                      <button class="icon-button templates-table__delete" type="button" title="删除模板" :aria-label="`删除模板 ${template.name}`" @click="openDelete('item', template)"><DeleteIcon /></button>
                    </template>
                  </span>
                </div>
              </article>
            </div>

            <div v-else class="templates-table templates-table--template" role="table" aria-label="入库模板列表">
              <div class="templates-table__head" role="row"><span>入库模板</span><span>字段结构</span><span>更新与操作</span></div>
              <article v-for="template in filteredInboundTemplates" :key="template.id" class="templates-table__row" role="row">
                <button class="templates-table__identity templates-table__identity--button" type="button" role="cell" @click="openTemplate('inbound', template, true)"><strong>{{ template.name }}</strong><span>入库模板 #{{ template.id }}</span><small>{{ template.description || '暂无说明' }}</small></button>
                <div class="templates-table__information" role="cell">
                  <span class="templates-table__metrics"><span>字段 <strong>{{ template.fields.length }}</strong></span><span>必填 <strong>{{ countRequired(template.fields) }}</strong></span><span>可筛选 <strong>{{ countSearchable(template.fields) }}</strong></span></span>
                  <span class="templates-table__types">{{ countFieldTypes(template.fields) }}</span>
                </div>
                <div class="templates-table__decision" role="cell">
                  <time :datetime="template.updated_at" :title="formatFullDateTime(template.updated_at)">{{ formatTime(template.updated_at) }}</time>
                  <span class="templates-table__actions">
                    <button class="icon-button" type="button" title="查看模板" :aria-label="`查看模板 ${template.name}`" @click="openTemplate('inbound', template, true)"><ViewIcon /></button>
                    <template v-if="canManage">
                      <button class="icon-button" type="button" title="编辑模板" :aria-label="`编辑模板 ${template.name}`" @click="openTemplate('inbound', template, false)"><EditIcon /></button>
                      <button class="icon-button" type="button" title="复制模板" :aria-label="`复制模板 ${template.name}`" @click="openCopy('inbound', template)"><CopyIcon /></button>
                      <button class="icon-button templates-table__delete" type="button" title="删除模板" :aria-label="`删除模板 ${template.name}`" @click="openDelete('inbound', template)"><DeleteIcon /></button>
                    </template>
                  </span>
                </div>
              </article>
            </div>
          </template>
          </div>
          </div>
        </Transition>
      </div>
    </section>

    <CategoryDialog
      :open="categoryDialogOpen"
      :category="editingCategory"
      :default-sort-order="defaultCategorySortOrder"
      :submitting="actionSubmitting"
      :error-message="actionError"
      :field-errors="actionFieldErrors"
      @close="closeActions"
      @submit="saveCategory"
    />
    <TemplateEditorDialog
      :open="Boolean(editorState)"
      :kind="editorState?.kind ?? 'item'"
      :template="editorState?.template ?? null"
      :inbound-templates="inboundTemplates"
      :read-only="editorState?.readOnly ?? false"
      :can-edit="canManage"
      :submitting="actionSubmitting"
      :error-message="actionError"
      :field-errors="actionFieldErrors"
      @close="closeActions"
      @edit="enableEditor"
      @submit="saveTemplate"
    />
    <TemplateCopyDialog :target="copyTarget" :submitting="actionSubmitting" :error-message="actionError" :field-error="actionFieldErrors.name ?? ''" @close="closeActions" @submit="copyTemplate" />
    <TemplateDeleteDialog :target="deleteTarget" :submitting="actionSubmitting" :error-message="actionError" @close="closeActions" @submit="deleteTargetRecord" />
  </section>
</template>

<script setup lang="ts">
import { computed, defineComponent, h, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue'
import { ApiConfigurationError, ApiError, ApiNetworkError, ApiResponseError } from '../api/errors'
import {
  createItemCategory,
  deleteItemCategory,
  listItemCategories,
  updateItemCategory,
  type ItemCategoryResponse,
  type ItemCategoryWriteRequest,
} from '../api/itemCategories'
import {
  copyItemAttributeTemplate,
  createItemAttributeTemplate,
  deleteItemAttributeTemplate,
  listItemAttributeTemplates,
  updateItemAttributeTemplate,
  type ItemAttributeTemplateResponse,
} from '../api/itemAttributeTemplates'
import {
  copyInboundTemplate,
  createInboundTemplate,
  deleteInboundTemplate,
  listInboundTemplates,
  updateInboundTemplate,
  type InboundTemplateResponse,
} from '../api/inboundTemplates'
import type { TemplateFieldResponse } from '../api/templateFields'
import { hasPermission, stockPermissions } from '../auth/permissions'
import { authSession } from '../auth/session'
import CategoryDialog from '../components/templates/CategoryDialog.vue'
import TemplateCopyDialog, { type TemplateCopyTarget } from '../components/templates/TemplateCopyDialog.vue'
import TemplateDeleteDialog, { type TemplateDeleteTarget } from '../components/templates/TemplateDeleteDialog.vue'
import TemplateEditorDialog from '../components/templates/TemplateEditorDialog.vue'
import SearchField from '../components/SearchField.vue'
import { useStablePendingIndicator } from '../composables/useStablePendingIndicator'
import { notice } from '../notices/notice'
import {
  buildInboundTemplateRequest,
  buildItemTemplateRequest,
  countFieldTypes,
  type AttributeTemplateKind,
  type TemplateDomain,
  type TemplateDraft,
} from './templates/model'
import './TemplatesPage.scss'

interface DomainState {
  loaded: boolean
  loading: boolean
  error: string
}

interface EditorState {
  kind: AttributeTemplateKind
  template: ItemAttributeTemplateResponse | InboundTemplateResponse | null
  readOnly: boolean
}

const domains: { value: TemplateDomain; label: string }[] = [
  { value: 'category', label: '物品分类' },
  { value: 'item', label: '物品属性模板' },
  { value: 'inbound', label: '入库模板' },
]

const activeDomain = ref<TemplateDomain>('category')
const domainTransitionDirection = ref<'next' | 'previous'>('next')
const categories = ref<ItemCategoryResponse[]>([])
const itemTemplates = ref<ItemAttributeTemplateResponse[]>([])
const inboundTemplates = ref<InboundTemplateResponse[]>([])
const states = reactive<Record<TemplateDomain, DomainState>>({
  category: { loaded: false, loading: false, error: '' },
  item: { loaded: false, loading: false, error: '' },
  inbound: { loaded: false, loading: false, error: '' },
})
const searchInputs = reactive<Record<TemplateDomain, string>>({ category: '', item: '', inbound: '' })
const searches = reactive<Record<TemplateDomain, string>>({ category: '', item: '', inbound: '' })
const categoryDialogOpen = ref(false)
const editingCategory = ref<ItemCategoryResponse | null>(null)
const editorState = ref<EditorState | null>(null)
const copyTarget = ref<TemplateCopyTarget | null>(null)
const deleteTarget = ref<TemplateDeleteTarget | null>(null)
const actionSubmitting = ref(false)
const actionError = ref('')
const actionFieldErrors = ref<Record<string, string>>({})
const controllers = new Map<TemplateDomain, AbortController>()

const currentPermissions = computed(() => authSession.value?.user.permissions)
const canManage = computed(() => hasPermission(currentPermissions.value, stockPermissions.templateManage))
const currentState = computed(() => states[activeDomain.value])
const currentSearch = computed(() => searches[activeDomain.value])
const activeDomainLabel = computed(() => domains.find((domain) => domain.value === activeDomain.value)?.label ?? '')
const domainTransitionName = computed(() => `templates-domain-${domainTransitionDirection.value}`)
const domainCountLabel = computed(() => ({ category: '个分类', item: '个物品模板', inbound: '个入库模板' })[activeDomain.value])
const searchLabel = computed(() => `搜索${activeDomainLabel.value}`)
const searchPlaceholder = computed(() => `搜索${activeDomain.value === 'category' ? '分类' : '模板'}名称或说明`)
const refreshPending = computed(() => currentState.value.loaded && currentState.value.loading)
const initialLoading = computed(() => !currentState.value.loaded && currentState.value.loading)
const showRefreshing = useStablePendingIndicator(refreshPending, { showDelayMs: 200, minimumVisibleMs: 350 })
const showInitialLoading = useStablePendingIndicator(initialLoading, { showDelayMs: 200, minimumVisibleMs: 350 })
const filteredCategories = computed(() => filterRecords(categories.value, searches.category)
  .sort((left, right) => left.sort_order - right.sort_order || left.name.localeCompare(right.name, 'zh-CN') || left.id - right.id))
const filteredItemTemplates = computed(() => filterRecords(itemTemplates.value, searches.item))
const filteredInboundTemplates = computed(() => filterRecords(inboundTemplates.value, searches.inbound))
const filteredCount = computed(() => activeDomain.value === 'category'
  ? filteredCategories.value.length
  : activeDomain.value === 'item' ? filteredItemTemplates.value.length : filteredInboundTemplates.value.length)
const defaultCategorySortOrder = computed(() => Math.max(-1, ...categories.value.map((item) => item.sort_order)) + 1)

onMounted(() => { void loadDomain('category') })
onBeforeUnmount(() => controllers.forEach((controller) => controller.abort()))

watch(canManage, (allowed) => {
  if (allowed) return
  categoryDialogOpen.value = false
  copyTarget.value = null
  deleteTarget.value = null
  if (editorState.value) editorState.value = { ...editorState.value, readOnly: true }
})

async function loadDomain(domain: TemplateDomain, announce = false): Promise<boolean> {
  controllers.get(domain)?.abort()
  const controller = new AbortController()
  controllers.set(domain, controller)
  states[domain].loading = true
  states[domain].error = ''
  try {
    if (domain === 'category') categories.value = await listItemCategories(controller.signal)
    else if (domain === 'item') {
      itemTemplates.value = await listItemAttributeTemplates(controller.signal)
      if (!states.inbound.loaded && !states.inbound.loading) void loadDomain('inbound')
    } else inboundTemplates.value = await listInboundTemplates(controller.signal)
    states[domain].loaded = true
    if (announce) notice.success(`${domains.find((item) => item.value === domain)?.label}已刷新`)
    return true
  } catch (error) {
    if (isAbortError(error)) return false
    states[domain].error = errorMessage(error, `加载${domains.find((item) => item.value === domain)?.label}失败`)
    notice.error(states[domain].error)
    return false
  } finally {
    if (controllers.get(domain) === controller) {
      controllers.delete(domain)
      states[domain].loading = false
    }
  }
}

function selectDomain(domain: TemplateDomain): void {
  if (activeDomain.value === domain) return
  const currentIndex = domains.findIndex((item) => item.value === activeDomain.value)
  const nextIndex = domains.findIndex((item) => item.value === domain)
  domainTransitionDirection.value = nextIndex > currentIndex ? 'next' : 'previous'
  activeDomain.value = domain
  if (!states[domain].loaded && !states[domain].loading) void loadDomain(domain)
}

function handleTabKeydown(event: KeyboardEvent): void {
  if (!['ArrowLeft', 'ArrowRight'].includes(event.key)) return
  event.preventDefault()
  const index = domains.findIndex((domain) => domain.value === activeDomain.value)
  const next = (index + (event.key === 'ArrowRight' ? 1 : -1) + domains.length) % domains.length
  selectDomain(domains[next].value)
  requestAnimationFrame(() => document.getElementById(`templates-tab-${domains[next].value}`)?.focus())
}

function refreshCurrent(): void { void loadDomain(activeDomain.value, true) }
function applySearch(value: string): void { searches[activeDomain.value] = value.trim() }
function clearSearch(): void { searchInputs[activeDomain.value] = ''; searches[activeDomain.value] = '' }

function openCreate(): void {
  resetActionState()
  if (activeDomain.value === 'category') {
    editingCategory.value = null
    categoryDialogOpen.value = true
  } else editorState.value = { kind: activeDomain.value, template: null, readOnly: false }
}

function openCategory(category: ItemCategoryResponse): void {
  resetActionState()
  editingCategory.value = category
  categoryDialogOpen.value = true
}

function openTemplate(kind: AttributeTemplateKind, template: ItemAttributeTemplateResponse | InboundTemplateResponse, readOnly: boolean): void {
  resetActionState()
  editorState.value = { kind, template, readOnly }
  if (!states.inbound.loaded && !states.inbound.loading) void loadDomain('inbound')
}

function enableEditor(): void {
  if (editorState.value && canManage.value) editorState.value = { ...editorState.value, readOnly: false }
}

function openCopy(kind: AttributeTemplateKind, template: ItemAttributeTemplateResponse | InboundTemplateResponse): void {
  resetActionState()
  copyTarget.value = { id: template.id, name: template.name, kind }
}

function openDelete(kind: TemplateDomain, record: { id: number; name: string }): void {
  resetActionState()
  deleteTarget.value = { id: record.id, name: record.name, kind }
}

function closeActions(): void {
  if (actionSubmitting.value) return
  categoryDialogOpen.value = false
  editingCategory.value = null
  editorState.value = null
  copyTarget.value = null
  deleteTarget.value = null
  resetActionState()
}

async function saveCategory(request: ItemCategoryWriteRequest): Promise<void> {
  actionSubmitting.value = true
  resetActionErrors()
  try {
    const updated = editingCategory.value
      ? await updateItemCategory(editingCategory.value.id, request)
      : await createItemCategory(request)
    categories.value = upsert(categories.value, updated)
    notice.success(editingCategory.value ? '物品分类已更新' : '物品分类已创建')
    closeActionsAfterSuccess()
  } catch (error) { handleActionError(error, '保存物品分类失败') }
  finally { actionSubmitting.value = false }
}

async function saveTemplate(draft: TemplateDraft): Promise<void> {
  if (!editorState.value) return
  actionSubmitting.value = true
  resetActionErrors()
  const { kind, template } = editorState.value
  try {
    if (kind === 'item') {
      const request = buildItemTemplateRequest(draft)
      const updated = template
        ? await updateItemAttributeTemplate(template.id, request)
        : await createItemAttributeTemplate(request)
      itemTemplates.value = upsert(itemTemplates.value, updated)
    } else {
      const request = buildInboundTemplateRequest(draft)
      const updated = template
        ? await updateInboundTemplate(template.id, request)
        : await createInboundTemplate(request)
      inboundTemplates.value = upsert(inboundTemplates.value, updated)
    }
    notice.success(template ? '模板已更新' : '模板已创建')
    closeActionsAfterSuccess()
  } catch (error) { handleActionError(error, '保存模板失败') }
  finally { actionSubmitting.value = false }
}

async function copyTemplate(name: string): Promise<void> {
  const target = copyTarget.value
  if (!target) return
  actionSubmitting.value = true
  resetActionErrors()
  try {
    if (target.kind === 'item') {
      const copied = await copyItemAttributeTemplate(target.id, { name })
      itemTemplates.value = upsert(itemTemplates.value, copied)
      editorState.value = { kind: 'item', template: copied, readOnly: false }
    } else {
      const copied = await copyInboundTemplate(target.id, { name })
      inboundTemplates.value = upsert(inboundTemplates.value, copied)
      editorState.value = { kind: 'inbound', template: copied, readOnly: false }
    }
    copyTarget.value = null
    notice.success('模板已复制，请检查后保存')
  } catch (error) { handleActionError(error, '复制模板失败') }
  finally { actionSubmitting.value = false }
}

async function deleteTargetRecord(): Promise<void> {
  const target = deleteTarget.value
  if (!target) return
  actionSubmitting.value = true
  resetActionErrors()
  try {
    if (target.kind === 'category') {
      await deleteItemCategory(target.id)
      categories.value = categories.value.filter((item) => item.id !== target.id)
    } else if (target.kind === 'item') {
      await deleteItemAttributeTemplate(target.id)
      itemTemplates.value = itemTemplates.value.filter((item) => item.id !== target.id)
    } else {
      await deleteInboundTemplate(target.id)
      inboundTemplates.value = inboundTemplates.value.filter((item) => item.id !== target.id)
    }
    notice.success(target.kind === 'category' ? '物品分类已删除' : '模板已删除')
    closeActionsAfterSuccess()
  } catch (error) { handleActionError(error, '删除失败') }
  finally { actionSubmitting.value = false }
}

function defaultInboundLabel(id: number | null): string {
  if (id === null) return '未设置'
  return inboundTemplates.value.find((template) => template.id === id)?.name ?? `已删除入库模板 #${id}`
}

function isUnresolvedDefault(template: ItemAttributeTemplateResponse): boolean {
  return template.default_inbound_template_id !== null && !inboundTemplates.value.some((item) => item.id === template.default_inbound_template_id)
}

function countRequired(fields: readonly TemplateFieldResponse[]): number { return fields.filter((field) => field.required).length }
function countSearchable(fields: readonly TemplateFieldResponse[]): number { return fields.filter((field) => field.searchable).length }
function countCatalogVisible(template: ItemAttributeTemplateResponse): number { return template.fields.filter((field) => field.catalog_visible).length }

function filterRecords<T extends { name: string; description: string | null }>(records: readonly T[], search: string): T[] {
  const normalized = search.trim().toLocaleLowerCase()
  if (!normalized) return [...records]
  return records.filter((item) => `${item.name}\n${item.description ?? ''}`.toLocaleLowerCase().includes(normalized))
}

function upsert<T extends { id: number }>(records: readonly T[], record: T): T[] {
  const existing = records.findIndex((item) => item.id === record.id)
  if (existing < 0) return [record, ...records]
  const next = [...records]
  next.splice(existing, 1, record)
  return next
}

function formatTime(value: string): string {
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? '--:--' : new Intl.DateTimeFormat('zh-CN', { hour: '2-digit', minute: '2-digit', hour12: false }).format(date)
}

function formatFullDateTime(value: string): string {
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : new Intl.DateTimeFormat('zh-CN', { dateStyle: 'medium', timeStyle: 'short' }).format(date)
}

function resetActionState(): void { actionError.value = ''; actionFieldErrors.value = {} }
function resetActionErrors(): void { actionError.value = ''; actionFieldErrors.value = {} }
function closeActionsAfterSuccess(): void {
  categoryDialogOpen.value = false
  editingCategory.value = null
  editorState.value = null
  copyTarget.value = null
  deleteTarget.value = null
  resetActionState()
}

function handleActionError(error: unknown, fallback: string): void {
  actionError.value = errorMessage(error, fallback)
  actionFieldErrors.value = apiFieldErrors(error)
  if (error instanceof ApiError && ['category_name_taken', 'template_name_taken'].includes(error.code)) {
    actionFieldErrors.value.name = error.code === 'category_name_taken' ? '分类名称已存在' : '模板名称已存在'
  }
  notice.error(actionError.value)
  if (error instanceof ApiError && error.status === 404) void loadDomain(activeDomain.value)
}

function apiFieldErrors(error: unknown): Record<string, string> {
  if (!(error instanceof ApiError)) return {}
  return Object.fromEntries(Object.entries(error.fieldErrors).map(([path, messages]) => [normalizeFieldPath(path), messages[0] ?? '字段无效']))
}

function normalizeFieldPath(path: string): string {
  return path
    .replace(/\[(\d+)\]/g, '.$1')
    .replace(/^body\./, '')
    .replace(/\.unit\.value$/, '.unit_value')
    .replace(/\.unit\.options(?=\.|$)/, '.unit_options')
}

function errorMessage(error: unknown, fallback: string): string {
  if (error instanceof ApiError) {
    if (error.code === 'category_name_taken') return '分类名称已存在'
    if (error.code === 'template_name_taken') return '模板名称已存在'
    if (error.status === 403) return '当前账号已没有执行此操作的权限'
    if (error.status === 404) return '记录已被删除，请刷新后重试'
    return error.message || fallback
  }
  if (error instanceof ApiNetworkError) return '无法连接到 WineStock 服务'
  if (error instanceof ApiConfigurationError || error instanceof ApiResponseError) return error.message
  return fallback
}

function isAbortError(error: unknown): boolean { return error instanceof DOMException && error.name === 'AbortError' }

const icon = (paths: string[]) => defineComponent({
  setup: () => () => h('svg', { viewBox: '0 0 24 24', 'aria-hidden': 'true' }, paths.map((path) => h('path', { d: path }))),
})
const ViewIcon = icon(['M2.5 12s3.5-6 9.5-6 9.5 6 9.5 6-3.5 6-9.5 6S2.5 12 2.5 12Z', 'M12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6Z'])
const EditIcon = icon(['m5 17-1 3 3-1L19 7l-2-2L5 17Z', 'm15 7 2 2'])
const CopyIcon = icon(['M8 8h11v11H8z', 'M5 16H4V5h11v1'])
const DeleteIcon = icon(['M4 7h16M9 7V4h6v3M7 7l1 13h8l1-13M10 11v5M14 11v5'])
</script>
