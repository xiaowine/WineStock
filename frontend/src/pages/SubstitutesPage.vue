<!--
  本文件拥有全局替代关系的查询、分组、搜索、刷新、权限和编辑 Dialog 编排。
  它只使用 HTTP 契约，不复制单物品替代关系的草稿与保存逻辑。
-->
<template>
  <section class="route-page substitutes-page">
    <header class="content-header substitutes-page__header">
      <div>
        <h1>替代关系</h1>
        <p>从全局视角维护缺货时的替代顺序和兼容性说明。</p>
      </div>
    </header>

    <section class="substitutes-workspace" aria-label="替代关系列表">
      <div class="substitutes-toolbar">
        <SearchField
          v-model="searchInput"
          class="substitutes-toolbar__search"
          label="搜索替代关系"
          name="substitute_relation_search"
          placeholder="主物品、替代物品、SKU 或备注"
          hide-label
          :disabled="loading && !loaded"
          @search="applySearch"
        />

        <div class="substitutes-toolbar__commands">
          <div class="substitutes-toolbar__summary">
            <span class="substitutes-toolbar__count">{{ visibleCountLabel }}</span>
            <span v-if="showStableRefreshing" class="substitutes-toolbar__refresh-status" role="status">正在刷新</span>
          </div>
          <div class="substitutes-toolbar__actions">
            <button
              class="icon-button substitutes-toolbar__refresh"
              :class="{ 'substitutes-toolbar__refresh--pending': showStableRefreshing }"
              type="button"
              title="刷新替代关系"
              aria-label="刷新替代关系"
              :aria-busy="loading"
              :disabled="loading"
              @click="refreshRelations"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M20 7v5h-5"/><path d="M18.2 16a7 7 0 1 1 .8-7l1 3"/></svg>
            </button>
            <button
              class="icon-button substitutes-toolbar__network"
              type="button"
              title="查看替代关系网络"
              aria-label="查看替代关系网络"
              :disabled="!loaded"
              @click="networkOpen = true"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M12 5v4M6 19v-4h12v4M6 15v-3h12v3" />
                <rect x="9" y="2" width="6" height="4" rx="1" />
                <rect x="3" y="18" width="6" height="4" rx="1" />
                <rect x="15" y="18" width="6" height="4" rx="1" />
              </svg>
            </button>
            <button
              v-if="canManage && canReadItems"
              class="icon-button icon-button--primary"
              type="button"
              title="新增替代关系"
              aria-label="新增替代关系"
              @click="openCreate"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14"/></svg>
            </button>
          </div>
        </div>
      </div>

      <div class="substitutes-results" :class="{ 'substitutes-results--refreshing': showStableRefreshing }" :aria-busy="loading">
        <div v-if="loadError && !loaded" class="substitutes-state substitutes-state--error" role="alert">
          <strong>无法加载替代关系</strong>
          <span>{{ loadError }}</span>
          <button class="secondary-button" type="button" @click="retryLoad">重试</button>
        </div>
        <div v-else-if="loading && !loaded" class="substitutes-state" role="status">
          <span v-if="showInitialLoading">正在加载替代关系…</span>
        </div>
        <div v-else-if="!visibleGroups.length" class="substitutes-state">
          <strong>{{ activeSearch ? '没有匹配的替代关系' : '暂无已配置的替代关系' }}</strong>
          <span>{{ activeSearch ? '可以清除搜索后查看全部已有关系。' : canManage && canReadItems ? '可以从工具栏新增替代关系。' : '当前没有可查看的替代关系。' }}</span>
          <button v-if="activeSearch" class="text-button" type="button" @click="clearSearch">清除搜索</button>
        </div>
        <template v-else>
          <p v-if="loadError" class="substitutes-results__inline-error" role="alert">{{ loadError }}</p>
          <div class="substitutes-table" role="table" aria-label="全局替代关系">
            <div class="substitutes-table__head" role="row">
              <span role="columnheader">主物品身份</span>
              <span role="columnheader">替代关系摘要</span>
              <span role="columnheader">判断与操作</span>
            </div>
            <SubstituteRelationGroup
              v-for="group in visibleGroups"
              :key="group.itemId"
              :group="group"
              @open="openExisting"
            />
          </div>
        </template>
      </div>
    </section>

    <SubstituteNetworkDialog
      :open="networkOpen"
      :relations="relations"
      :can-manage="canManage"
      :refreshing="loading && loaded"
      @close="networkOpen = false"
      @edit="editFromNetwork"
    />

    <SubstituteEditorDialog
      :open="editorOpen"
      :target="editorTarget"
      :can-manage="canManage"
      :can-search-candidates="canReadItems"
      @close="closeEditor"
      @saved="handleSaved"
      @dirty-change="editorDirty = $event"
    />

    <ModalDialog
      :open="routeDiscardOpen"
      title="离开替代关系页面？"
      description="当前未保存的替代关系修改不会保留。"
      nested
      compact
      @close="cancelRouteLeave"
    >
      <p class="confirmation-copy">此操作不会修改服务端已经保存的数据。</p>
      <template #actions>
        <button class="secondary-button" type="button" @click="cancelRouteLeave">继续编辑</button>
        <button class="danger-button" type="button" @click="confirmRouteLeave">放弃修改并离开</button>
      </template>
    </ModalDialog>
  </section>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { onBeforeRouteLeave } from 'vue-router'
import { listSubstituteRelations, type SubstituteRelationResponse } from '../api/substitutes'
import { authSession } from '../auth/session'
import { hasPermission, stockPermissions } from '../auth/permissions'
import ModalDialog from '../components/ModalDialog.vue'
import SearchField from '../components/SearchField.vue'
import SubstituteEditorDialog from '../components/substitutes/SubstituteEditorDialog.vue'
import SubstituteNetworkDialog from '../components/substitutes/SubstituteNetworkDialog.vue'
import SubstituteRelationGroup from '../components/substitutes/SubstituteRelationGroup.vue'
import { useStablePendingIndicator } from '../composables/useStablePendingIndicator'
import { notice } from '../notices/notice'
import {
  countGroupedRelations,
  filterSubstituteRelationGroups,
  groupSubstituteRelations,
  type SubstituteEditorTarget,
  type SubstituteRelationGroupModel,
} from './substitutes/model'
import { formatRelationCount, substituteErrorMessage } from './substitutes/presentation'
import './SubstitutesPage.scss'

const relations = ref<SubstituteRelationResponse[]>([])
const loaded = ref(false)
const loading = ref(false)
const loadError = ref('')
const searchInput = ref('')
const activeSearch = ref('')
const editorOpen = ref(false)
const editorTarget = ref<SubstituteEditorTarget | null>(null)
const editorDirty = ref(false)
const networkOpen = ref(false)
const returnToNetworkAfterEditor = ref(false)
const routeDiscardOpen = ref(false)
let requestController: AbortController | null = null
let pendingLeaveResolution: ((allow: boolean) => void) | null = null

const canManage = computed(() => hasPermission(authSession.value?.user.permissions, stockPermissions.substituteManage))
const canReadItems = computed(() => hasPermission(authSession.value?.user.permissions, stockPermissions.itemRead))
const groups = computed(() => groupSubstituteRelations(relations.value))
const visibleGroups = computed(() => filterSubstituteRelationGroups(groups.value, activeSearch.value))
const visibleCountLabel = computed(() => formatRelationCount(visibleGroups.value.length, countGroupedRelations(visibleGroups.value)))
const showInitialLoading = useStablePendingIndicator(computed(() => loading.value && !loaded.value), { showDelayMs: 200, minimumVisibleMs: 350 })
const showStableRefreshing = useStablePendingIndicator(computed(() => loading.value && loaded.value), { showDelayMs: 200, minimumVisibleMs: 350 })

onMounted(() => {
  window.addEventListener('beforeunload', handleBeforeUnload)
  void loadRelations()
})

onBeforeUnmount(() => {
  requestController?.abort()
  window.removeEventListener('beforeunload', handleBeforeUnload)
  pendingLeaveResolution?.(false)
})

onBeforeRouteLeave(() => {
  if (!editorDirty.value) return true
  routeDiscardOpen.value = true
  return new Promise<boolean>((resolve) => { pendingLeaveResolution = resolve })
})

async function loadRelations(showSuccessNotice = false): Promise<boolean> {
  requestController?.abort()
  const controller = new AbortController()
  requestController = controller
  loading.value = true
  if (!loaded.value) loadError.value = ''
  try {
    relations.value = await listSubstituteRelations(controller.signal)
    loaded.value = true
    loadError.value = ''
    if (showSuccessNotice) notice.success('替代关系已刷新')
    return true
  } catch (error) {
    if (error instanceof DOMException && error.name === 'AbortError') return false
    loadError.value = substituteErrorMessage(error)
    if (loaded.value) notice.error('刷新替代关系失败', { detail: loadError.value })
    return false
  } finally {
    if (requestController === controller) {
      requestController = null
      loading.value = false
    }
  }
}

function refreshRelations(): void {
  void loadRelations(true)
}

function retryLoad(): void {
  void loadRelations()
}

function applySearch(value: string): void {
  activeSearch.value = value.trim()
}

function clearSearch(): void {
  searchInput.value = ''
  activeSearch.value = ''
}

function openCreate(): void {
  returnToNetworkAfterEditor.value = false
  editorTarget.value = null
  editorOpen.value = true
}

function openExisting(group: SubstituteRelationGroupModel): void {
  returnToNetworkAfterEditor.value = false
  editorTarget.value = { id: group.itemId, name: group.itemName, sku: group.itemSku }
  editorOpen.value = true
}

function closeEditor(): void {
  editorOpen.value = false
  editorTarget.value = null
  editorDirty.value = false
  if (returnToNetworkAfterEditor.value) {
    returnToNetworkAfterEditor.value = false
    networkOpen.value = true
  }
}

function handleSaved(): void {
  editorDirty.value = false
  void loadRelations()
}

function editFromNetwork(target: SubstituteEditorTarget): void {
  networkOpen.value = false
  returnToNetworkAfterEditor.value = true
  editorTarget.value = target
  editorOpen.value = true
}

function cancelRouteLeave(): void {
  routeDiscardOpen.value = false
  pendingLeaveResolution?.(false)
  pendingLeaveResolution = null
}

function confirmRouteLeave(): void {
  routeDiscardOpen.value = false
  editorDirty.value = false
  editorOpen.value = false
  networkOpen.value = false
  returnToNetworkAfterEditor.value = false
  pendingLeaveResolution?.(true)
  pendingLeaveResolution = null
}

function handleBeforeUnload(event: BeforeUnloadEvent): void {
  if (!editorDirty.value) return
  event.preventDefault()
  event.returnValue = ''
}
</script>
