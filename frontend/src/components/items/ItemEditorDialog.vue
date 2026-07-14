<!--
  本组件拥有物品创建/已有物品工作区 Dialog，复用同一编辑器并为已有物品提供库存页。
  create 会话不挂载库存状态；existing 会话按需请求库存与批次，不拥有目录刷新。
-->
<template>
  <ModalDialog
    :open="open"
    :title="title"
    :description="readOnly && activePage === 'data' ? '你拥有查看权限，物品资料不可修改。' : undefined"
    :busy="saving"
    :wide="mode === 'create'"
    :workspace="mode === 'existing'"
    @close="emit('close')"
  >
    <template v-if="mode === 'existing'" #context>
      <div class="item-workspace__context">
        <div>
          <span>当前物品</span>
          <strong :title="draft.name || itemName">{{ draft.name || itemName || '未命名物品' }}</strong>
        </div>
        <div class="item-workspace__identity">
          <span>编号</span>
          <strong :title="draft.sku || itemSku">{{ draft.sku || itemSku || '未设置' }}</strong>
        </div>
      </div>
    </template>

    <div v-if="mode === 'existing'" class="item-workspace">
      <nav
        ref="workspaceNav"
        class="item-workspace__nav"
        :class="{
          'item-workspace__nav--two': itemPageCount === 2,
          'item-workspace__nav--scrollable': itemPageCount > 4,
        }"
        aria-label="物品页面"
      >
        <button
          type="button"
          :class="{ 'is-active': activePage === 'data' }"
          :aria-pressed="activePage === 'data'"
          @click="selectPage('data')"
        >
          <span>物品资料</span>
        </button>
        <button
          type="button"
          :class="{ 'is-active': activePage === 'inventory' }"
          :aria-pressed="activePage === 'inventory'"
          @click="selectPage('inventory')"
        >
          <span>库存详情</span>
        </button>
        <button
          v-if="canViewSubstitutes"
          type="button"
          :class="{ 'is-active': activePage === 'substitutes' }"
          :aria-pressed="activePage === 'substitutes'"
          @click="selectPage('substitutes')"
        >
          <span>替代关系</span>
        </button>
      </nav>

      <Transition name="item-workspace-panel" mode="out-in">
        <section :key="activePage" class="item-workspace__panel">
          <header class="item-workspace__panel-header">
            <div v-if="activePage === 'data'">
              <strong>物品资料</strong>
              <span>基础资料与物品属性</span>
            </div>
            <template v-else-if="activePage === 'inventory'">
              <div>
                <strong>库存详情</strong>
                <span v-if="inventory">{{ inventory.locations.length }} 个库位 · {{ inventory.batch_count }} 个批次</span>
              </div>
              <button
                class="icon-button"
                :class="{ 'is-pending': inventoryPending }"
                type="button"
                title="刷新库存详情"
                aria-label="刷新库存详情"
                :disabled="inventoryPending"
                @click="loadInventory(true)"
              >
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M20 7v5h-5"/><path d="M18.2 16a7 7 0 1 1 .8-7l1 3"/></svg>
              </button>
            </template>
            <div v-else>
              <strong>替代关系</strong>
              <span>按优先级使用可替代物品</span>
            </div>
          </header>

          <div class="item-workspace__content">
            <div v-if="activePage === 'data' && !dataReady" class="dialog-state" :class="{ 'dialog-state--error': dataError }" :role="dataError ? 'alert' : 'status'">
              <span>{{ dataLoading ? '正在加载物品资料…' : dataError || '物品资料尚未加载' }}</span>
              <button v-if="!dataLoading" class="secondary-button" type="button" @click="emit('request-data')">重试</button>
            </div>
            <ItemEditor
              v-else-if="activePage === 'data'"
              :draft="draft"
              :categories="categories"
              :templates="templates"
              :saving="saving"
              :metadata-error="metadataError"
              :validation-errors="validationErrors"
              :form-id="formId"
              :read-only="readOnly"
              embedded
              @save="emit('save')"
            />
            <KeepAlive v-else-if="activePage === 'substitutes'">
              <ItemSubstitutesPanel
                :item-id="itemId ?? 0"
                :can-manage="canManageSubstitutes"
                @dirty-change="handleSubstitutesDirty"
              />
            </KeepAlive>
            <div v-else class="item-inventory" :aria-busy="inventoryPending">

              <div v-if="inventoryError && !inventory" class="dialog-state dialog-state--error" role="alert">
                <span>{{ inventoryError }}</span>
                <button class="secondary-button" type="button" @click="loadInventory(true)">重试</button>
              </div>
              <template v-else-if="inventory">
                <p v-if="inventoryError" class="item-inventory__inline-error" role="alert">{{ inventoryError }}</p>
                <div class="item-inventory__summary">
                  <div><span>当前库存</span><strong>{{ formatQuantity(inventory.current_quantity) }} {{ inventory.unit }}</strong></div>
                  <div><span>库存价值</span><strong>{{ formatMoney(inventory.inventory_value) }}</strong></div>
                  <div><span>补货点</span><strong>{{ inventory.reorder_point === null ? '未设置' : `${formatQuantity(inventory.reorder_point)} ${inventory.unit}` }}</strong></div>
                  <div><span>库存状态</span><strong :class="`stock-state stock-state--${inventory.stock_state}`">{{ stockStateLabel(inventory.stock_state) }}</strong></div>
                </div>

                <section class="item-inventory__section">
                  <h3>库位分布</h3>
                  <div v-if="inventory.locations.length" class="item-inventory__locations">
                    <div v-for="location in inventory.locations" :key="location.location_id">
                      <span><strong>{{ location.location_code }}</strong>{{ location.location_name }}</span>
                      <span>{{ formatQuantity(location.quantity) }} {{ inventory.unit }}</span>
                      <small>{{ location.batch_count }} 个批次 · {{ formatMoney(location.value) }}</small>
                    </div>
                  </div>
                  <p v-else class="item-inventory__empty">暂无在库库位</p>
                </section>

                <section class="item-inventory__section">
                  <h3>当前批次</h3>
                  <div v-if="batches.length" class="item-inventory__batches">
                    <div v-for="batch in batches" :key="batch.id">
                      <span><strong>{{ batch.batch_no }}</strong>{{ batch.location_code }} · {{ batch.location_name }}</span>
                      <span>{{ formatQuantity(batch.remaining_quantity) }} {{ inventory.unit }}</span>
                      <small>{{ formatMoney(batch.unit_cost) }}/{{ inventory.unit }} · {{ batch.expires_at ? `有效期 ${formatDate(batch.expires_at)}` : '无有效期' }}</small>
                    </div>
                  </div>
                  <p v-else-if="!batchesPending" class="item-inventory__empty">暂无有效批次</p>
                  <div class="item-inventory__more">
                    <span v-if="batchesError" role="alert">{{ batchesError }}</span>
                    <button v-if="batchesError" class="text-button" type="button" @click="loadBatchPage(batchPage || 1)">重试本页</button>
                    <button v-else-if="batchPage < batchTotalPages" class="text-button" type="button" :disabled="batchesPending" @click="loadBatchPage(batchPage + 1)">
                      {{ batchesPending ? '正在加载…' : '加载更多批次' }}
                    </button>
                  </div>
                </section>
              </template>
              <div v-else class="dialog-state" role="status">正在加载库存详情…</div>
            </div>
          </div>
        </section>
      </Transition>
    </div>

    <ItemEditor
      v-else
      :draft="draft"
      :categories="categories"
      :templates="templates"
      :saving="saving"
      :metadata-error="metadataError"
      :validation-errors="validationErrors"
      :form-id="formId"
      :read-only="readOnly"
      embedded
      @save="emit('save')"
    />

    <template #actions>
      <button class="secondary-button" type="button" :disabled="saving" @click="emit('close')">
        {{ activePage !== 'data' || readOnly ? '关闭' : '取消' }}
      </button>
      <button v-if="activePage === 'data' && !readOnly" class="primary-button" type="submit" :form="formId" :disabled="saving || dataLoading || substitutesDirty" :title="substitutesDirty ? '请先保存替代关系' : undefined">
        {{ saving ? '保存中…' : '保存物品' }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, useId, watch } from 'vue'
import type { ItemCategoryResponse } from '../../api/itemCategories'
import type { ItemAttributeTemplateResponse } from '../../api/itemAttributeTemplates'
import {
  getItemInventory, listItemBatches, type ItemBatchStockResponse, type ItemInventoryResponse,
  type ItemStockState,
} from '../../api/items'
import type { ItemDraft } from '../../pages/items/model'
import { ApiError } from '../../api/errors'
import ModalDialog from '../ModalDialog.vue'
import ItemEditor from './ItemEditor.vue'
import ItemSubstitutesPanel from './ItemSubstitutesPanel.vue'

type ItemDialogPage = 'data' | 'inventory' | 'substitutes'

const props = withDefaults(defineProps<{
  open: boolean
  mode: 'create' | 'existing'
  itemId?: number | null
  itemName?: string
  itemSku?: string
  initialPage?: ItemDialogPage
  draft: ItemDraft
  categories: ItemCategoryResponse[]
  templates: ItemAttributeTemplateResponse[]
  saving: boolean
  dataLoading?: boolean
  dataReady?: boolean
  dataError?: string
  metadataError: string
  validationErrors: Record<string, string>
  /** 当前会话是否只能查看已有物品资料。 */
  readOnly?: boolean
  /** 当前用户是否可以查看替代关系。 */
  canViewSubstitutes?: boolean
  /** 当前用户是否可以修改替代关系。 */
  canManageSubstitutes?: boolean
}>(), {
  itemId: null,
  itemName: '',
  itemSku: '',
  initialPage: 'data',
  dataLoading: false,
  dataReady: true,
  dataError: '',
  readOnly: false,
  canViewSubstitutes: false,
  canManageSubstitutes: false,
})

const emit = defineEmits<{
  save: []
  close: []
  'request-data': []
  'substitutes-dirty': [dirty: boolean]
}>()
const formId = `item-editor-${useId()}`
const activePage = ref<ItemDialogPage>('data')
const workspaceNav = ref<HTMLElement | null>(null)
const inventory = ref<ItemInventoryResponse | null>(null)
const batches = ref<ItemBatchStockResponse[]>([])
const inventoryPending = ref(false)
const batchesPending = ref(false)
const inventoryError = ref('')
const batchesError = ref('')
const substitutesDirty = ref(false)
const batchPage = ref(0)
const batchTotalPages = ref(0)
const itemPageCount = computed(() => 2 + (props.canViewSubstitutes ? 1 : 0))
let inventoryController: AbortController | null = null
let batchController: AbortController | null = null

const title = computed(() => props.mode === 'create' ? '新建物品' : '物品详情')

watch(() => props.open, (open) => {
  if (!open) {
    abortRequests()
    substitutesDirty.value = false
    emit('substitutes-dirty', false)
    return
  }
  activePage.value = props.mode === 'existing' ? props.initialPage : 'data'
  inventory.value = null
  batches.value = []
  batchPage.value = 0
  batchTotalPages.value = 0
  inventoryError.value = ''
  batchesError.value = ''
  if (activePage.value === 'inventory') void loadInventory()
})

watch([activePage, itemPageCount], async () => {
  if (itemPageCount.value <= 4) return
  await nextTick()
  workspaceNav.value?.querySelector<HTMLElement>('.is-active')?.scrollIntoView({
    behavior: 'smooth',
    block: 'nearest',
    inline: 'nearest',
  })
}, { flush: 'post' })

onBeforeUnmount(abortRequests)

function selectPage(page: ItemDialogPage): void {
  if (activePage.value === page) return
  activePage.value = page
  if (page === 'data' && !props.dataReady && !props.dataLoading) emit('request-data')
  if (page === 'inventory' && !inventory.value) void loadInventory()
}

function handleSubstitutesDirty(value: boolean): void {
  substitutesDirty.value = value
  emit('substitutes-dirty', value)
}

async function loadInventory(force = false): Promise<void> {
  if (props.mode !== 'existing' || !props.itemId || (inventory.value && !force)) return
  inventoryController?.abort()
  batchController?.abort()
  const controller = new AbortController()
  inventoryController = controller
  inventoryPending.value = true
  inventoryError.value = ''
  try {
    const next = await getItemInventory(props.itemId, controller.signal)
    inventory.value = next
    batches.value = []
    batchPage.value = 0
    batchTotalPages.value = 0
    await loadBatchPage(1)
  } catch (error) {
    if (error instanceof DOMException && error.name === 'AbortError') return
    inventoryError.value = errorMessage(error)
  } finally {
    if (inventoryController === controller) {
      inventoryController = null
      inventoryPending.value = false
    }
  }
}

async function loadBatchPage(page: number): Promise<void> {
  if (!props.itemId) return
  batchController?.abort()
  const controller = new AbortController()
  batchController = controller
  batchesPending.value = true
  batchesError.value = ''
  try {
    const response = await listItemBatches(props.itemId, page, 20, controller.signal)
    batches.value = page === 1 ? response.items : mergeBatches(batches.value, response.items)
    batchPage.value = response.page
    batchTotalPages.value = response.total_pages
  } catch (error) {
    if (error instanceof DOMException && error.name === 'AbortError') return
    batchesError.value = errorMessage(error)
  } finally {
    if (batchController === controller) {
      batchController = null
      batchesPending.value = false
    }
  }
}

function abortRequests(): void {
  inventoryController?.abort()
  batchController?.abort()
  inventoryController = null
  batchController = null
}

function stockStateLabel(state: ItemStockState): string {
  return { out_of_stock: '缺货', reorder_due: '待补货', needs_configuration: '需配置', normal: '库存正常' }[state]
}

function formatQuantity(value: number): string {
  return new Intl.NumberFormat('zh-CN', { maximumFractionDigits: 3 }).format(value)
}

function formatMoney(value: number): string {
  return new Intl.NumberFormat('zh-CN', { style: 'currency', currency: 'CNY', maximumFractionDigits: 2 }).format(value)
}

function formatDate(value: string): string {
  return value.slice(0, 10)
}

function errorMessage(error: unknown): string {
  return error instanceof ApiError ? error.message : '无法连接到 WineStock 服务'
}

function mergeBatches(current: ItemBatchStockResponse[], next: ItemBatchStockResponse[]): ItemBatchStockResponse[] {
  const map = new Map(current.map((batch) => [batch.id, batch]))
  next.forEach((batch) => map.set(batch.id, batch))
  return Array.from(map.values())
}
</script>

<style lang="scss" src="./ItemEditorDialog.scss"></style>
