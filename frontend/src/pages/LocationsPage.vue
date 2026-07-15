<!--
  本文件拥有库位管理页面的分组树、库位列表、权限入口和 CRUD 请求编排。
  它不直接修改库存数量，也不在缺少批次查询契约时伪造库位库存汇总。
-->
<template>
  <section class="route-page locations-page">
    <header class="content-header locations-page__header">
      <div>
        <h1>库位管理</h1>
        <p>维护库位分组和入库、库存批次实际使用的存放位置。</p>
      </div>
    </header>

    <div class="locations-page__workspace">
      <aside class="location-groups" :class="{ 'location-groups--open': groupPanelOpen }" aria-label="库位分组面板">
        <header class="location-groups__header">
          <div>
            <span>分组导航</span>
            <strong>库位分组</strong>
          </div>
          <div class="location-groups__header-actions">
            <button
              v-if="canManage"
              class="icon-button"
              type="button"
              title="新建根分组"
              aria-label="新建根分组"
              @click="openCreateGroup(null)"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14"/></svg>
            </button>
            <button class="icon-button location-groups__close" type="button" title="关闭分组面板" aria-label="关闭分组面板" @click="groupPanelOpen = false">
              <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m6 6 12 12M18 6 6 18"/></svg>
            </button>
          </div>
        </header>

        <div class="location-groups__body" :aria-busy="treeLoading">
          <div v-if="treeError && !treeLoaded" class="location-groups__state location-groups__state--error" role="alert">
            <span>{{ treeError }}</span>
            <button class="text-button" type="button" @click="loadTree">重试</button>
          </div>
          <div v-else-if="showTreeLoading && !treeLoaded" class="location-groups__state" role="status">正在加载分组…</div>
          <div v-else-if="!groupTree.length" class="location-groups__state">
            <span>暂无库位分组</span>
            <button v-if="canManage" class="text-button" type="button" @click="openCreateGroup(null)">新建根分组</button>
          </div>
          <template v-else>
            <p v-if="treeError" class="location-groups__inline-error" role="alert">{{ treeError }}</p>
            <LocationGroupTree
              :nodes="groupTree"
              :selected-group-id="selectedGroupId"
              :expanded-group-ids="expandedGroupIds"
              :can-manage="canManage"
              @select="selectGroup"
              @toggle="toggleGroup"
              @create-child="openCreateGroup"
              @edit="openEditGroup"
              @delete="openDeleteGroup"
            />
          </template>
        </div>
      </aside>

      <button
        v-if="groupPanelOpen"
        class="location-groups-backdrop"
        type="button"
        aria-label="关闭分组面板"
        @click="groupPanelOpen = false"
      ></button>

      <section class="locations-catalog" aria-label="库位列表">
        <div class="locations-catalog__toolbar">
          <div class="locations-catalog__context">
            <button
              class="secondary-button locations-catalog__group-trigger"
              type="button"
              title="选择分组"
              aria-label="选择分组"
              @click="groupPanelOpen = true"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M12 5v4M6 19v-4h12v4M6 15v-3h12v3" />
                <rect x="9" y="2" width="6" height="4" rx="1" />
                <rect x="3" y="18" width="6" height="4" rx="1" />
                <rect x="15" y="18" width="6" height="4" rx="1" />
              </svg>
              <span>选择分组</span>
            </button>
            <div>
              <span>{{ selectedGroupPath.length ? selectedGroupPath.join(' / ') : '全部库位' }}</span>
              <strong>{{ selectedGroup?.name ?? '全部库位' }}</strong>
            </div>
          </div>

          <SearchField
            v-model="searchInput"
            class="locations-catalog__search"
            label="搜索库位"
            name="location_search"
            placeholder="搜索名称或备注"
            hide-label
            :disabled="locationsLoading && !locationsLoaded"
            @search="applySearch"
          />

          <div class="locations-catalog__commands">
            <div class="locations-catalog__summary">
              <span class="locations-catalog__count">{{ locations.length }} 个库位</span>
              <span v-if="showStableListRefreshing" class="locations-catalog__refresh-status" role="status">正在刷新</span>
            </div>
            <div class="locations-catalog__actions">
              <button
                class="icon-button locations-catalog__refresh"
                :class="{ 'locations-catalog__refresh--pending': showStableRefreshing }"
                type="button"
                title="刷新库位数据"
                aria-label="刷新库位数据"
                :aria-busy="treeLoading || locationsLoading"
                :disabled="treeLoading || locationsLoading"
                @click="refreshAll"
              >
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M20 7v5h-5"/><path d="M18.2 16a7 7 0 1 1 .8-7l1 3"/></svg>
              </button>
              <button
                v-if="canManage"
                class="icon-button icon-button--primary locations-catalog__create"
                type="button"
                title="新建库位"
                aria-label="新建库位"
                :disabled="groupOptions.length === 0"
                @click="openCreateLocation"
              >
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14"/></svg>
              </button>
            </div>
          </div>
        </div>

        <div class="locations-catalog__body" :class="{ 'locations-catalog__body--refreshing': showStableListRefreshing }" :aria-busy="locationsLoading">
          <div v-if="locationError && !locationsLoaded" class="locations-catalog__state locations-catalog__state--error" role="alert">
            <span>{{ locationError }}</span>
            <button class="secondary-button" type="button" @click="loadLocationList">重试</button>
          </div>
          <div v-else-if="showLocationLoading && !locationsLoaded" class="locations-catalog__state" role="status">正在加载库位…</div>
          <div v-else-if="!locations.length" class="locations-catalog__state">
            <strong>{{ activeSearch ? '没有匹配的库位' : selectedGroup ? '当前分组暂无库位' : '暂无库位' }}</strong>
            <span>{{ activeSearch ? '可以清除搜索或切换分组。' : canManage ? '可以从工具栏新建库位。' : '当前没有可查看的库位。' }}</span>
            <button v-if="activeSearch" class="text-button" type="button" @click="clearSearch">清除搜索</button>
          </div>
          <template v-else>
            <p v-if="locationError" class="locations-catalog__inline-error" role="alert">{{ locationError }}</p>
            <div class="locations-table" :class="{ 'locations-table--readonly': !canManage }" role="table" aria-label="库位主数据">
              <div class="locations-table__head" role="row">
                <span role="columnheader">库位</span>
                <span role="columnheader">备注</span>
                <span role="columnheader">排序与操作</span>
              </div>
              <article v-for="location in locations" :key="location.id" class="locations-table__row" role="row">
                <div class="locations-table__identity" role="cell">
                  <strong :title="location.name">{{ location.name }}</strong>
                  <span :title="location.group_name">{{ location.group_name }}</span>
                </div>
                <div class="locations-table__notes" role="cell" :title="location.notes ?? undefined">
                  {{ location.notes || '暂无备注' }}
                </div>
                <div class="locations-table__decision" role="cell">
                  <div class="locations-table__meta">
                    <span class="locations-table__meta-row">
                      <span class="locations-table__meta-label">排序：</span>
                      <strong class="locations-table__meta-value">{{ location.sort_order }}</strong>
                    </span>
                    <span class="locations-table__meta-row">
                      <span class="locations-table__meta-label">更新：</span>
                      <time class="locations-table__meta-value" :datetime="location.updated_at">{{ formatDateTime(location.updated_at) }}</time>
                    </span>
                  </div>
                  <span v-if="canManage" class="locations-table__actions">
                    <button class="icon-button" type="button" title="编辑库位" :aria-label="`编辑库位 ${location.name}`" @click="openEditLocation(location)">
                      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m5 17-1 3 3-1L19 7l-2-2L5 17Z"/><path d="m15 7 2 2"/></svg>
                    </button>
                    <button class="icon-button locations-table__delete" type="button" title="删除库位" :aria-label="`删除库位 ${location.name}`" @click="openDeleteLocation(location)">
                      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 7h16M9 7V4h6v3M7 7l1 13h8l1-13M10 11v5M14 11v5"/></svg>
                    </button>
                  </span>
                </div>
              </article>
            </div>

            <div class="locations-mobile-list" aria-label="库位主数据">
              <article v-for="location in locations" :key="location.id" class="locations-mobile-list__item">
                <header>
                  <div>
                    <strong :title="location.name">{{ location.name }}</strong>
                  </div>
                  <span v-if="canManage" class="locations-mobile-list__actions">
                    <button class="icon-button" type="button" title="编辑库位" :aria-label="`编辑库位 ${location.name}`" @click="openEditLocation(location)">
                      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m5 17-1 3 3-1L19 7l-2-2L5 17Z"/><path d="m15 7 2 2"/></svg>
                    </button>
                    <button class="icon-button locations-table__delete" type="button" title="删除库位" :aria-label="`删除库位 ${location.name}`" @click="openDeleteLocation(location)">
                      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 7h16M9 7V4h6v3M7 7l1 13h8l1-13M10 11v5M14 11v5"/></svg>
                    </button>
                  </span>
                </header>
                <dl>
                  <div><dt>所属分组</dt><dd>{{ location.group_name }}</dd></div>
                  <div v-if="location.notes" class="locations-mobile-list__notes"><dt>备注</dt><dd>{{ location.notes }}</dd></div>
                  <div><dt>排序</dt><dd>{{ location.sort_order }}</dd></div>
                  <div><dt>更新时间</dt><dd>{{ formatDateTime(location.updated_at) }}</dd></div>
                </dl>
              </article>
            </div>
          </template>
        </div>
      </section>
    </div>

    <LocationGroupDialog
      :open="groupDialogOpen"
      :group="editingGroup"
      :default-parent-id="defaultGroupParentId"
      :parent-options="groupParentOptions"
      :submitting="actionSubmitting"
      :error-message="actionError"
      :field-errors="actionFieldErrors"
      @close="closeDialogs"
      @submit="saveGroup"
    />
    <LocationDialog
      :open="locationDialogOpen"
      :location="editingLocation"
      :default-group-id="defaultLocationGroupId"
      :group-options="groupOptions"
      :submitting="actionSubmitting"
      :error-message="actionError"
      :field-errors="actionFieldErrors"
      @close="closeDialogs"
      @submit="saveLocation"
    />
    <LocationDeleteDialog
      :target="deleteTarget"
      :submitting="actionSubmitting"
      :error-message="actionError"
      @close="closeDialogs"
      @submit="confirmDelete"
    />
  </section>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import {
  createLocation,
  createLocationGroup,
  deleteLocation,
  deleteLocationGroup,
  listLocationGroupTree,
  listLocations,
  updateLocation,
  updateLocationGroup,
  type LocationGroupResponse,
  type LocationGroupTreeNode,
  type LocationGroupUpdateRequest,
  type LocationResponse,
  type LocationUpdateRequest,
} from '../api/locations'
import { ApiConfigurationError, ApiError, ApiNetworkError, ApiResponseError } from '../api/errors'
import { hasPermission, stockPermissions } from '../auth/permissions'
import { authSession } from '../auth/session'
import LocationDeleteDialog from '../components/locations/LocationDeleteDialog.vue'
import LocationDialog from '../components/locations/LocationDialog.vue'
import LocationGroupDialog from '../components/locations/LocationGroupDialog.vue'
import LocationGroupTree from '../components/locations/LocationGroupTree.vue'
import type { LocationDeleteTarget, LocationGroupOption } from '../components/locations/types'
import SearchField from '../components/SearchField.vue'
import { useStablePendingIndicator } from '../composables/useStablePendingIndicator'
import { notice } from '../notices/notice'
import './LocationsPage.scss'

const MAX_LOCATION_GROUP_DEPTH = 10

const groupTree = ref<LocationGroupTreeNode[]>([])
const locations = ref<LocationResponse[]>([])
const selectedGroupId = ref<number | null>(null)
const expandedGroupIds = ref<number[]>([])
const searchInput = ref('')
const activeSearch = ref('')
const treeLoaded = ref(false)
const locationsLoaded = ref(false)
const treeLoading = ref(false)
const locationsLoading = ref(false)
const treeError = ref('')
const locationError = ref('')
const groupPanelOpen = ref(false)
const groupDialogOpen = ref(false)
const locationDialogOpen = ref(false)
const editingGroup = ref<LocationGroupResponse | null>(null)
const editingLocation = ref<LocationResponse | null>(null)
const defaultGroupParentId = ref<number | null>(null)
const deleteTarget = ref<LocationDeleteTarget | null>(null)
const actionSubmitting = ref(false)
const actionError = ref('')
const actionFieldErrors = ref<Record<string, string>>({})
let treeController: AbortController | null = null
let locationsController: AbortController | null = null

const currentPermissions = computed(() => authSession.value?.user.permissions)
const canManage = computed(() => hasPermission(currentPermissions.value, stockPermissions.locationManage))
const selectedGroup = computed(() => selectedGroupId.value === null ? null : findGroup(groupTree.value, selectedGroupId.value))
const selectedGroupPath = computed(() => selectedGroupId.value === null ? [] : findGroupPath(groupTree.value, selectedGroupId.value) ?? [])
const groupOptions = computed<LocationGroupOption[]>(() => flattenGroupOptions(groupTree.value))
const groupParentOptions = computed<LocationGroupOption[]>(() => {
  if (!editingGroup.value) {
    return groupOptions.value.filter((option) => option.depth < MAX_LOCATION_GROUP_DEPTH)
  }
  const node = findGroup(groupTree.value, editingGroup.value.id)
  const excludedIds = node ? collectGroupIds(node) : new Set([editingGroup.value.id])
  const subtreeHeight = node ? locationGroupSubtreeHeight(node) : 1
  return flattenGroupOptions(groupTree.value, excludedIds)
    .filter((option) => option.depth + subtreeHeight <= MAX_LOCATION_GROUP_DEPTH)
})
const defaultLocationGroupId = computed(() => selectedGroupId.value ?? groupOptions.value[0]?.id ?? null)
const refreshPending = computed(() => (treeLoaded.value && treeLoading.value) || (locationsLoaded.value && locationsLoading.value))
const listRefreshPending = computed(() => locationsLoaded.value && locationsLoading.value)
const showStableRefreshing = useStablePendingIndicator(refreshPending, { showDelayMs: 200, minimumVisibleMs: 350 })
const showStableListRefreshing = useStablePendingIndicator(listRefreshPending, { showDelayMs: 200, minimumVisibleMs: 350 })
const showTreeLoading = useStablePendingIndicator(treeLoading, { showDelayMs: 200, minimumVisibleMs: 350 })
const showLocationLoading = useStablePendingIndicator(locationsLoading, { showDelayMs: 200, minimumVisibleMs: 350 })

onMounted(() => {
  void Promise.all([loadTree(), loadLocationList()])
})

onBeforeUnmount(() => {
  treeController?.abort()
  locationsController?.abort()
})

/** 加载完整分组树；刷新期间保留旧树和展开状态。 */
async function loadTree(): Promise<boolean> {
  treeController?.abort()
  const controller = new AbortController()
  treeController = controller
  treeLoading.value = true
  treeError.value = ''
  try {
    const nextTree = await listLocationGroupTree(controller.signal)
    const wasLoaded = treeLoaded.value
    groupTree.value = nextTree
    if (!wasLoaded) expandedGroupIds.value = nextTree.map((group) => group.id)
    if (selectedGroupId.value !== null && !findGroup(nextTree, selectedGroupId.value)) selectedGroupId.value = null
    treeLoaded.value = true
    return true
  } catch (error) {
    if (isAbortError(error)) return false
    treeError.value = locationManagementErrorMessage(error, '加载库位分组失败')
    notice.error(treeError.value)
    return false
  } finally {
    if (treeController === controller) {
      treeController = null
      treeLoading.value = false
    }
  }
}

/** 按当前分组和搜索词加载库位；刷新失败时保留旧列表。 */
async function loadLocationList(): Promise<boolean> {
  locationsController?.abort()
  const controller = new AbortController()
  locationsController = controller
  locationsLoading.value = true
  locationError.value = ''
  try {
    locations.value = await listLocations({
      group_id: selectedGroupId.value ?? undefined,
      search: activeSearch.value || undefined,
    }, controller.signal)
    locationsLoaded.value = true
    return true
  } catch (error) {
    if (isAbortError(error)) return false
    locationError.value = locationManagementErrorMessage(error, '加载库位失败')
    notice.error(locationError.value)
    return false
  } finally {
    if (locationsController === controller) {
      locationsController = null
      locationsLoading.value = false
    }
  }
}

async function refreshAll(): Promise<void> {
  const treeSucceeded = await loadTree()
  const locationsSucceeded = await loadLocationList()
  if (treeSucceeded && locationsSucceeded) notice.success('库位数据已刷新')
}

function selectGroup(groupId: number | null): void {
  groupPanelOpen.value = false
  if (selectedGroupId.value === groupId) return
  selectedGroupId.value = groupId
  void loadLocationList()
}

function toggleGroup(groupId: number): void {
  const expanded = new Set(expandedGroupIds.value)
  if (expanded.has(groupId)) expanded.delete(groupId)
  else expanded.add(groupId)
  expandedGroupIds.value = Array.from(expanded)
}

function applySearch(value: string): void {
  if (value === activeSearch.value) return
  activeSearch.value = value
  void loadLocationList()
}

function clearSearch(): void {
  searchInput.value = ''
  applySearch('')
}

function openCreateGroup(parent: LocationGroupTreeNode | null): void {
  if (parent && (findGroupPath(groupTree.value, parent.id)?.length ?? 0) >= MAX_LOCATION_GROUP_DEPTH) {
    notice.warning('无法新建子分组', { detail: '库位分组最多只能有 10 层' })
    return
  }
  closeDialogs()
  editingGroup.value = null
  defaultGroupParentId.value = parent?.id ?? null
  groupDialogOpen.value = true
}

function openEditGroup(group: LocationGroupTreeNode): void {
  closeDialogs()
  editingGroup.value = group
  defaultGroupParentId.value = group.parent_id
  groupDialogOpen.value = true
}

function openDeleteGroup(group: LocationGroupTreeNode): void {
  closeDialogs()
  deleteTarget.value = { kind: 'group', id: group.id, label: group.name, parentId: group.parent_id }
}

function openCreateLocation(): void {
  closeDialogs()
  editingLocation.value = null
  locationDialogOpen.value = true
}

function openEditLocation(location: LocationResponse): void {
  closeDialogs()
  editingLocation.value = location
  locationDialogOpen.value = true
}

function openDeleteLocation(location: LocationResponse): void {
  closeDialogs()
  deleteTarget.value = { kind: 'location', id: location.id, label: location.name }
}

function closeDialogs(): void {
  if (actionSubmitting.value) return
  groupDialogOpen.value = false
  locationDialogOpen.value = false
  editingGroup.value = null
  editingLocation.value = null
  deleteTarget.value = null
  actionError.value = ''
  actionFieldErrors.value = {}
}

async function saveGroup(request: LocationGroupUpdateRequest): Promise<void> {
  actionSubmitting.value = true
  actionError.value = ''
  actionFieldErrors.value = {}
  try {
    const wasEditing = Boolean(editingGroup.value)
    const saved = editingGroup.value
      ? await updateLocationGroup(editingGroup.value.id, request)
      : await createLocationGroup(request)
    if (saved.parent_id !== null) expandGroup(saved.parent_id)
    selectedGroupId.value = saved.id
    closeDialogsAfterSubmit()
    await Promise.all([loadTree(), loadLocationList()])
    notice.success(wasEditing ? '库位分组已更新' : '库位分组已创建')
  } catch (error) {
    applyActionError(error, '保存库位分组失败')
  } finally {
    actionSubmitting.value = false
  }
}

async function saveLocation(request: LocationUpdateRequest): Promise<void> {
  actionSubmitting.value = true
  actionError.value = ''
  actionFieldErrors.value = {}
  try {
    const wasEditing = Boolean(editingLocation.value)
    await (editingLocation.value
      ? updateLocation(editingLocation.value.id, request)
      : createLocation(request))
    closeDialogsAfterSubmit()
    await Promise.all([loadTree(), loadLocationList()])
    notice.success(wasEditing ? '库位已更新' : '库位已创建')
  } catch (error) {
    applyActionError(error, '保存库位失败')
  } finally {
    actionSubmitting.value = false
  }
}

async function confirmDelete(): Promise<void> {
  const target = deleteTarget.value
  if (!target) return
  actionSubmitting.value = true
  actionError.value = ''
  actionFieldErrors.value = {}
  try {
    if (target.kind === 'group') {
      await deleteLocationGroup(target.id)
      selectedGroupId.value = target.parentId ?? null
    } else {
      await deleteLocation(target.id)
    }
    closeDialogsAfterSubmit()
    await Promise.all([loadTree(), loadLocationList()])
    notice.success(target.kind === 'group' ? '库位分组已删除' : '库位已删除')
  } catch (error) {
    applyActionError(error, target.kind === 'group' ? '删除库位分组失败' : '删除库位失败')
  } finally {
    actionSubmitting.value = false
  }
}

function closeDialogsAfterSubmit(): void {
  groupDialogOpen.value = false
  locationDialogOpen.value = false
  editingGroup.value = null
  editingLocation.value = null
  deleteTarget.value = null
  actionError.value = ''
  actionFieldErrors.value = {}
}

function expandGroup(groupId: number): void {
  if (!expandedGroupIds.value.includes(groupId)) expandedGroupIds.value = [...expandedGroupIds.value, groupId]
}

function applyActionError(error: unknown, fallback: string): void {
  const result = locationManagementError(error, fallback)
  actionError.value = result.message
  actionFieldErrors.value = result.fieldErrors
  notice.error(result.message)
}

function findGroup(nodes: LocationGroupTreeNode[], groupId: number): LocationGroupTreeNode | null {
  for (const node of nodes) {
    if (node.id === groupId) return node
    const child = findGroup(node.children, groupId)
    if (child) return child
  }
  return null
}

function findGroupPath(nodes: LocationGroupTreeNode[], groupId: number, path: string[] = []): string[] | null {
  for (const node of nodes) {
    const nextPath = [...path, node.name]
    if (node.id === groupId) return nextPath
    const childPath = findGroupPath(node.children, groupId, nextPath)
    if (childPath) return childPath
  }
  return null
}

function flattenGroupOptions(
  nodes: LocationGroupTreeNode[],
  excludedIds: ReadonlySet<number> = new Set(),
  depth = 1,
): LocationGroupOption[] {
  const options: LocationGroupOption[] = []
  for (const node of nodes) {
    if (excludedIds.has(node.id)) continue
    options.push({ id: node.id, label: `${'— '.repeat(depth - 1)}${node.name}`, depth })
    options.push(...flattenGroupOptions(node.children, excludedIds, depth + 1))
  }
  return options
}

function collectGroupIds(node: LocationGroupTreeNode): Set<number> {
  return new Set([node.id, ...node.children.flatMap((child) => Array.from(collectGroupIds(child)))])
}

function locationGroupSubtreeHeight(node: LocationGroupTreeNode): number {
  return 1 + Math.max(0, ...node.children.map(locationGroupSubtreeHeight))
}

function locationManagementError(error: unknown, fallback: string): { message: string; fieldErrors: Record<string, string> } {
  if (error instanceof ApiError) {
    const messages: Record<string, string> = {
      location_group_name_taken: '同一上级分组下已经存在同名分组',
      location_group_cycle: '不能把分组移动到自身或子分组下',
      location_group_depth_exceeded: '库位分组最多只能有 10 层',
      location_group_in_use: '该分组仍包含子分组或有效库位，请先移动或删除子项',
      location_group_not_found: '所选库位分组不存在或已被删除',
      location_name_taken: '该库位名称已被使用',
      location_in_use: '该库位仍有当前库存批次引用，请先移库或清空库存',
      location_not_found: '目标库位不存在或已被删除',
      invalid_request: '提交的库位信息无效，请检查后重试',
    }
    const fieldErrors = Object.fromEntries(
      Object.entries(error.fieldErrors).map(([path, values]) => [path.split('.').at(-1) ?? path, values[0] ?? error.message]),
    )
    if (error.code === 'location_group_name_taken') fieldErrors.name = messages[error.code]
    if (error.code === 'location_group_cycle') fieldErrors.parent_id = messages[error.code]
    if (error.code === 'location_group_depth_exceeded') fieldErrors.parent_id = messages[error.code]
    if (error.code === 'location_name_taken') fieldErrors.name = messages[error.code]
    if (error.code === 'location_group_not_found') {
      fieldErrors[groupDialogOpen.value ? 'parent_id' : 'group_id'] = messages[error.code]
    }
    return { message: messages[error.code] ?? error.message ?? fallback, fieldErrors }
  }
  return { message: locationManagementErrorMessage(error, fallback), fieldErrors: {} }
}

function locationManagementErrorMessage(error: unknown, fallback: string): string {
  if (error instanceof ApiError) return error.message || fallback
  if (error instanceof ApiNetworkError) return '无法连接服务，请检查服务状态后重试'
  if (error instanceof ApiConfigurationError || error instanceof ApiResponseError) return error.message
  return fallback
}

function isAbortError(error: unknown): boolean {
  return error instanceof DOMException && error.name === 'AbortError'
}

function formatDateTime(value: string): string {
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : new Intl.DateTimeFormat('zh-CN', {
    year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit',
  }).format(date)
}
</script>
