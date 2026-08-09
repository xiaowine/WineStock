<!--
  本文件拥有库位管理页面的分组树、库位列表、权限入口和 CRUD 请求编排。
  它不直接修改库存数量，也不在缺少批次查询契约时伪造库位库存汇总。
-->
<template>
  <section class="route-page locations-page">
    <header class="content-header locations-page__header">
      <div>
        <h1>{{ $title($route.meta.title) }}</h1>
        <p>{{ $t('locations.subtitle') }}</p>
      </div>
    </header>

    <div class="locations-page__workspace">
      <aside
        class="location-groups"
        :class="{ 'location-groups--open': groupPanelOpen }"
        :aria-label="$t('locations.groupPanel')"
      >
        <header class="location-groups__header">
          <div>
            <span>{{ $t('locations.groupNav') }}</span>
            <strong>{{ $t('locations.group') }}</strong>
          </div>
          <div class="location-groups__header-actions">
            <button
              v-if="canManage"
              class="icon-button"
              type="button"
              :title="$t('locations.createRootGroup')"
              :aria-label="$t('locations.createRootGroup')"
              @click="openCreateGroup(null)"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
            </button>
            <button
              class="icon-button location-groups__close"
              type="button"
              :title="$t('locations.closeGroupPanel')"
              :aria-label="$t('locations.closeGroupPanel')"
              @click="closeGroupPanel"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m6 6 12 12M18 6 6 18" /></svg>
            </button>
          </div>
        </header>

        <div v-overlay-scrollbar class="location-groups__body" :aria-busy="treeLoading">
          <div
            v-if="treeError && !treeLoaded"
            class="location-groups__state location-groups__state--error"
            role="alert"
          >
            <span>{{ treeError }}</span>
            <button class="text-button" type="button" @click="loadTree">
              {{ $t('locations.retry') }}
            </button>
          </div>
          <div
            v-else-if="showTreeLoading && !treeLoaded"
            class="location-groups__state"
            role="status"
          >
            {{ $t('locations.loadingGroups') }}
          </div>
          <div v-else-if="!groupTree.length" class="location-groups__state">
            <span>{{ $t('locations.noGroups') }}</span>
            <button
              v-if="canManage"
              class="text-button"
              type="button"
              @click="openCreateGroup(null)"
            >
              {{ $t('locations.createRootGroup') }}
            </button>
          </div>
          <template v-else>
            <p v-if="treeError" class="location-groups__inline-error" role="alert">
              {{ treeError }}
            </p>
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
        :aria-label="$t('locations.closeGroupPanel')"
        @click="closeGroupPanel"
      ></button>

      <section class="locations-catalog" :aria-label="$t('locations.listAria')">
        <div
          class="locations-catalog__toolbar"
          :class="{ 'locations-catalog__toolbar--readonly': !canManage }"
        >
          <SearchField
            v-model="searchInput"
            class="locations-catalog__search"
            :label="$t('locations.searchLabel')"
            name="location_search"
            :placeholder="$t('locations.searchPlaceholder')"
            :disabled="locationsLoading && !locationsLoaded"
            @search="applySearch"
          />

          <div class="locations-catalog__context">
            <button
              ref="groupPanelTrigger"
              class="secondary-button locations-catalog__group-trigger"
              type="button"
              :title="
                $t('locations.selectGroupTitle', {
                  current: selectedGroupPath.length
                    ? selectedGroupPath.join(' / ')
                    : $t('locations.allLocations'),
                })
              "
              :aria-label="
                $t('locations.selectGroupTitle', {
                  current: selectedGroupPath.length
                    ? selectedGroupPath.join(' / ')
                    : $t('locations.allLocations'),
                })
              "
              @click="groupPanelOpen = true"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M12 5v4M6 19v-4h12v4M6 15v-3h12v3" />
                <rect x="9" y="2" width="6" height="4" rx="1" />
                <rect x="3" y="18" width="6" height="4" rx="1" />
                <rect x="15" y="18" width="6" height="4" rx="1" />
              </svg>
              <span>{{ selectedGroup?.name ?? $t('locations.allLocations') }}</span>
            </button>
            <div class="locations-catalog__context-copy">
              <span>{{ $t('locations.group') }}</span>
              <strong>{{ selectedGroup?.name ?? $t('locations.allLocations') }}</strong>
            </div>
          </div>

          <div class="locations-catalog__commands">
            <div class="locations-catalog__summary">
              <span class="locations-catalog__count">{{
                $t('locations.count', { n: locations.length })
              }}</span>
              <span
                v-if="showStableListRefreshing"
                class="locations-catalog__refresh-status"
                role="status"
                >{{ $t('locations.refreshing') }}</span
              >
            </div>
            <div class="locations-catalog__actions">
              <button
                class="icon-button locations-catalog__refresh"
                :class="{ 'locations-catalog__refresh--pending': showStableRefreshing }"
                type="button"
                :title="$t('locations.refreshData')"
                :aria-label="$t('locations.refreshData')"
                :aria-busy="treeLoading || locationsLoading"
                :disabled="treeLoading || locationsLoading"
                @click="refreshAll"
              >
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  <path d="M20 7v5h-5" />
                  <path d="M18.2 16a7 7 0 1 1 .8-7l1 3" />
                </svg>
              </button>
              <button
                v-if="canManage"
                class="icon-button icon-button--primary locations-catalog__create"
                type="button"
                :title="$t('locations.createLocation')"
                :aria-label="$t('locations.createLocation')"
                :disabled="groupOptions.length === 0"
                @click="openCreateLocation"
              >
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
              </button>
            </div>
          </div>
        </div>

        <div
          v-overlay-scrollbar
          class="locations-catalog__body"
          :class="{ 'locations-catalog__body--refreshing': showStableListRefreshing }"
          :aria-busy="locationsLoading"
        >
          <div
            v-if="locationError && !locationsLoaded"
            class="locations-catalog__state locations-catalog__state--error"
            role="alert"
          >
            <span>{{ locationError }}</span>
            <button class="secondary-button" type="button" @click="loadLocationList">
              {{ $t('locations.retry') }}
            </button>
          </div>
          <div
            v-else-if="showLocationLoading && !locationsLoaded"
            class="locations-catalog__state"
            role="status"
          >
            {{ $t('locations.loadingLocations') }}
          </div>
          <div v-else-if="!locations.length" class="locations-catalog__state">
            <strong>{{
              activeSearch
                ? $t('locations.noMatchingLocations')
                : selectedGroup
                  ? $t('locations.noLocationsInGroup')
                  : $t('locations.noLocations')
            }}</strong>
            <span>{{
              activeSearch
                ? $t('locations.clearSearchOrSwitchHint')
                : canManage
                  ? $t('locations.createFromToolbarHint')
                  : $t('locations.nothingToViewHint')
            }}</span>
            <button v-if="activeSearch" class="text-button" type="button" @click="clearSearch">
              {{ $t('locations.clearSearch') }}
            </button>
          </div>
          <template v-else>
            <p v-if="locationError" class="locations-catalog__inline-error" role="alert">
              {{ locationError }}
            </p>
            <div
              class="locations-table"
              :class="{ 'locations-table--readonly': !canManage }"
              role="table"
              :aria-label="$t('locations.mainDataAria')"
            >
              <div class="locations-table__head" role="row">
                <span role="columnheader">{{ $t('locations.location') }}</span>
                <span role="columnheader">{{ $t('locations.remark') }}</span>
                <span role="columnheader">{{ $t('locations.sortAndActions') }}</span>
              </div>
              <article
                v-for="location in locations"
                :key="location.id"
                class="locations-table__row"
                role="row"
              >
                <div class="locations-table__identity" role="cell">
                  <strong :title="location.name"
                    >{{ location.name
                    }}<em v-if="location.is_default" class="locations-table__default-badge"
                      >{{ $t('locations.default') }}</em
                    ></strong
                  >
                  <span :title="location.group_name">{{ location.group_name }}</span>
                </div>
                <div
                  class="locations-table__notes"
                  role="cell"
                  :title="location.notes ?? undefined"
                >
                  {{ location.notes || $t('locations.noNotes') }}
                </div>
                <div class="locations-table__decision" role="cell">
                  <div class="locations-table__meta">
                    <span class="locations-table__meta-row">
                      <span class="locations-table__meta-label">{{ $t('locations.sortOrderLabel') }}</span>
                      <strong class="locations-table__meta-value" :title="String(location.sort_order)">{{
                        location.sort_order
                      }}</strong>
                    </span>
                    <span class="locations-table__meta-row">
                      <span class="locations-table__meta-label">{{ $t('locations.updatedLabel') }}</span>
                      <time
                        class="locations-table__meta-value"
                        :datetime="location.updated_at"
                        :title="formatDateTime(location.updated_at)"
                        >{{ formatDateTime(location.updated_at) }}</time
                      >
                    </span>
                  </div>
                  <span v-if="canManage" class="locations-table__actions">
                    <button
                      class="icon-button"
                      :class="{ 'locations-table__default-active': location.is_default }"
                      type="button"
                      :title="location.is_default ? $t('locations.unsetDefault') : $t('locations.setDefault')"
                      :aria-label="
                        location.is_default
                          ? $t('locations.unsetDefaultNamed', { name: location.name })
                          : $t('locations.setDefaultNamed', { name: location.name })
                      "
                      :aria-pressed="location.is_default"
                      :disabled="defaultUpdatingId !== null"
                      @click="toggleDefaultLocation(location)"
                    >
                      <svg viewBox="0 0 24 24" aria-hidden="true">
                        <path
                          d="m12 3.5 2.5 5.4 5.9.6-4.4 4 1.3 5.8-5.3-3.1-5.3 3.1 1.3-5.8-4.4-4 5.9-.6z"
                        />
                      </svg>
                    </button>
                    <button
                      class="icon-button"
                      type="button"
                      :title="$t('locations.editLocation')"
                      :aria-label="$t('locations.editLocationNamed', { name: location.name })"
                      @click="openEditLocation(location)"
                    >
                      <svg viewBox="0 0 24 24" aria-hidden="true">
                        <path d="m5 17-1 3 3-1L19 7l-2-2L5 17Z" />
                        <path d="m15 7 2 2" />
                      </svg>
                    </button>
                    <button
                      class="icon-button locations-table__delete"
                      type="button"
                      :title="$t('locations.deleteLocation')"
                      :aria-label="$t('locations.deleteLocationNamed', { name: location.name })"
                      @click="openDeleteLocation(location)"
                    >
                      <svg viewBox="0 0 24 24" aria-hidden="true">
                        <path d="M4 7h16M9 7V4h6v3M7 7l1 13h8l1-13M10 11v5M14 11v5" />
                      </svg>
                    </button>
                  </span>
                </div>
              </article>
            </div>

            <div class="locations-mobile-list" :aria-label="$t('locations.mainDataAria')">
              <article
                v-for="location in locations"
                :key="location.id"
                class="locations-mobile-list__item"
              >
                <header>
                  <div class="locations-mobile-list__identity">
                    <div class="locations-mobile-list__title">
                      <strong :title="location.name">{{ location.name }}</strong>
                      <em v-if="location.is_default" class="locations-table__default-badge"
                        >{{ $t('locations.default') }}</em
                      >
                    </div>
                    <span :title="location.group_name">{{ location.group_name }}</span>
                  </div>
                  <span v-if="canManage" class="locations-mobile-list__actions">
                    <button
                      class="icon-button"
                      :class="{ 'locations-table__default-active': location.is_default }"
                      type="button"
                      :title="location.is_default ? $t('locations.unsetDefault') : $t('locations.setDefault')"
                      :aria-label="
                        location.is_default
                          ? $t('locations.unsetDefaultNamed', { name: location.name })
                          : $t('locations.setDefaultNamed', { name: location.name })
                      "
                      :aria-pressed="location.is_default"
                      :disabled="defaultUpdatingId !== null"
                      @click="toggleDefaultLocation(location)"
                    >
                      <svg viewBox="0 0 24 24" aria-hidden="true">
                        <path
                          d="m12 3.5 2.5 5.4 5.9.6-4.4 4 1.3 5.8-5.3-3.1-5.3 3.1 1.3-5.8-4.4-4 5.9-.6z"
                        />
                      </svg>
                    </button>
                    <button
                      class="icon-button"
                      type="button"
                      :title="$t('locations.editLocation')"
                      :aria-label="$t('locations.editLocationNamed', { name: location.name })"
                      @click="openEditLocation(location)"
                    >
                      <svg viewBox="0 0 24 24" aria-hidden="true">
                        <path d="m5 17-1 3 3-1L19 7l-2-2L5 17Z" />
                        <path d="m15 7 2 2" />
                      </svg>
                    </button>
                    <button
                      class="icon-button locations-table__delete"
                      type="button"
                      :title="$t('locations.deleteLocation')"
                      :aria-label="$t('locations.deleteLocationNamed', { name: location.name })"
                      @click="openDeleteLocation(location)"
                    >
                      <svg viewBox="0 0 24 24" aria-hidden="true">
                        <path d="M4 7h16M9 7V4h6v3M7 7l1 13h8l1-13M10 11v5M14 11v5" />
                      </svg>
                    </button>
                  </span>
                </header>
                <dl>
                  <div>
                    <dt>{{ $t('locations.updatedAt') }}</dt>
                    <dd>{{ formatDateTime(location.updated_at) }}</dd>
                  </div>
                  <div v-if="location.notes" class="locations-mobile-list__notes">
                    <dt>{{ $t('locations.remark') }}</dt>
                    <dd>{{ location.notes }}</dd>
                  </div>
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
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
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
} from "../api/locations";
import { ApiConfigurationError, ApiError, ApiNetworkError, ApiResponseError } from "../api/errors";
import { hasPermission, stockPermissions } from "../auth/permissions";
import { authSession } from "../auth/session";
import LocationDeleteDialog from "../components/locations/LocationDeleteDialog.vue";
import LocationDialog from "../components/locations/LocationDialog.vue";
import LocationGroupDialog from "../components/locations/LocationGroupDialog.vue";
import LocationGroupTree from "../components/locations/LocationGroupTree.vue";
import type { LocationDeleteTarget, LocationGroupOption } from "../components/locations/types";
import SearchField from "../components/SearchField.vue";
import { useNativeBackHandler } from "../composables/useNativeBackHandler";
import { useStablePendingIndicator } from "../composables/useStablePendingIndicator";
import { translateMessageOrNull } from "../i18n";
import { useI18n } from "vue-i18n";
import { NativeBackPriority } from "../navigation/nativeBack";
import { notice } from "../notices/notice";
import "./LocationsPage.scss";

const { t } = useI18n();

const MAX_LOCATION_GROUP_DEPTH = 10;

const groupTree = ref<LocationGroupTreeNode[]>([]);
const locations = ref<LocationResponse[]>([]);
const selectedGroupId = ref<number | null>(null);
const expandedGroupIds = ref<number[]>([]);
const searchInput = ref("");
const activeSearch = ref("");
const treeLoaded = ref(false);
const locationsLoaded = ref(false);
const treeLoading = ref(false);
const locationsLoading = ref(false);
const treeError = ref("");
const locationError = ref("");
const groupPanelOpen = ref(false);
const groupPanelTrigger = ref<HTMLButtonElement | null>(null);
const groupDialogOpen = ref(false);
const locationDialogOpen = ref(false);
const editingGroup = ref<LocationGroupResponse | null>(null);
const editingLocation = ref<LocationResponse | null>(null);
const defaultGroupParentId = ref<number | null>(null);
const deleteTarget = ref<LocationDeleteTarget | null>(null);
const actionSubmitting = ref(false);
const actionError = ref("");
const actionFieldErrors = ref<Record<string, string>>({});
let treeController: AbortController | null = null;
let locationsController: AbortController | null = null;

const currentPermissions = computed(() => authSession.value?.user.permissions);
const canManage = computed(() =>
  hasPermission(currentPermissions.value, stockPermissions.locationManage),
);
const selectedGroup = computed(() =>
  selectedGroupId.value === null ? null : findGroup(groupTree.value, selectedGroupId.value),
);
const selectedGroupPath = computed(() =>
  selectedGroupId.value === null
    ? []
    : (findGroupPath(groupTree.value, selectedGroupId.value) ?? []),
);
const groupOptions = computed<LocationGroupOption[]>(() => flattenGroupOptions(groupTree.value));
const groupParentOptions = computed<LocationGroupOption[]>(() => {
  if (!editingGroup.value) {
    return groupOptions.value.filter((option) => option.depth < MAX_LOCATION_GROUP_DEPTH);
  }
  const node = findGroup(groupTree.value, editingGroup.value.id);
  const excludedIds = node ? collectGroupIds(node) : new Set([editingGroup.value.id]);
  const subtreeHeight = node ? locationGroupSubtreeHeight(node) : 1;
  return flattenGroupOptions(groupTree.value, excludedIds).filter(
    (option) => option.depth + subtreeHeight <= MAX_LOCATION_GROUP_DEPTH,
  );
});
const defaultLocationGroupId = computed(
  () => selectedGroupId.value ?? groupOptions.value[0]?.id ?? null,
);
const refreshPending = computed(
  () =>
    (treeLoaded.value && treeLoading.value) || (locationsLoaded.value && locationsLoading.value),
);
const listRefreshPending = computed(() => locationsLoaded.value && locationsLoading.value);
const showStableRefreshing = useStablePendingIndicator(refreshPending, {
  showDelayMs: 200,
  minimumVisibleMs: 350,
});
const showStableListRefreshing = useStablePendingIndicator(listRefreshPending, {
  showDelayMs: 200,
  minimumVisibleMs: 350,
});
const showTreeLoading = useStablePendingIndicator(treeLoading, {
  showDelayMs: 200,
  minimumVisibleMs: 350,
});
const showLocationLoading = useStablePendingIndicator(locationsLoading, {
  showDelayMs: 200,
  minimumVisibleMs: 350,
});

useNativeBackHandler({
  id: "locations-group-drawer",
  active: groupPanelOpen,
  priority: NativeBackPriority.Drawer,
  handle: () => {
    if (!groupPanelOpen.value) return { handled: false };
    closeGroupPanel();
    return { handled: true, reason: "drawer" };
  },
});

onMounted(() => {
  void Promise.all([loadTree(), loadLocationList()]);
});

onBeforeUnmount(() => {
  treeController?.abort();
  locationsController?.abort();
});

/** 加载完整分组树；刷新期间保留旧树和展开状态。 */
async function loadTree(): Promise<boolean> {
  treeController?.abort();
  const controller = new AbortController();
  treeController = controller;
  treeLoading.value = true;
  treeError.value = "";
  try {
    const nextTree = await listLocationGroupTree(controller.signal);
    const wasLoaded = treeLoaded.value;
    groupTree.value = nextTree;
    if (!wasLoaded) expandedGroupIds.value = nextTree.map((group) => group.id);
    if (selectedGroupId.value !== null && !findGroup(nextTree, selectedGroupId.value))
      selectedGroupId.value = null;
    treeLoaded.value = true;
    return true;
  } catch (error) {
    if (isAbortError(error)) return false;
    treeError.value = locationManagementErrorMessage(error, t("locations.loadGroupsFailed"));
    notice.error(treeError.value);
    return false;
  } finally {
    if (treeController === controller) {
      treeController = null;
      treeLoading.value = false;
    }
  }
}

/** 按当前分组和搜索词加载库位；刷新失败时保留旧列表。 */
async function loadLocationList(): Promise<boolean> {
  locationsController?.abort();
  const controller = new AbortController();
  locationsController = controller;
  locationsLoading.value = true;
  locationError.value = "";
  try {
    locations.value = await listLocations(
      {
        group_id: selectedGroupId.value ?? undefined,
        search: activeSearch.value || undefined,
      },
      controller.signal,
    );
    locationsLoaded.value = true;
    return true;
  } catch (error) {
    if (isAbortError(error)) return false;
    locationError.value = locationManagementErrorMessage(error, t("locations.loadLocationsFailed"));
    notice.error(locationError.value);
    return false;
  } finally {
    if (locationsController === controller) {
      locationsController = null;
      locationsLoading.value = false;
    }
  }
}

async function refreshAll(): Promise<void> {
  const treeSucceeded = await loadTree();
  const locationsSucceeded = await loadLocationList();
  if (treeSucceeded && locationsSucceeded) notice.success(t("locations.dataRefreshed"));
}

function selectGroup(groupId: number | null): void {
  closeGroupPanel();
  if (selectedGroupId.value === groupId) return;
  selectedGroupId.value = groupId;
  void loadLocationList();
}

function closeGroupPanel(): void {
  if (!groupPanelOpen.value) return;
  groupPanelOpen.value = false;
  void nextTick(() => groupPanelTrigger.value?.focus());
}

function toggleGroup(groupId: number): void {
  const expanded = new Set(expandedGroupIds.value);
  if (expanded.has(groupId)) expanded.delete(groupId);
  else expanded.add(groupId);
  expandedGroupIds.value = Array.from(expanded);
}

function applySearch(value: string): void {
  if (value === activeSearch.value) return;
  activeSearch.value = value;
  void loadLocationList();
}

function clearSearch(): void {
  searchInput.value = "";
  applySearch("");
}

function openCreateGroup(parent: LocationGroupTreeNode | null): void {
  if (
    parent &&
    (findGroupPath(groupTree.value, parent.id)?.length ?? 0) >= MAX_LOCATION_GROUP_DEPTH
  ) {
    notice.warning(t("locations.cannotCreateChildGroup"), { detail: t("locations.maxDepthHint") });
    return;
  }
  closeDialogs();
  editingGroup.value = null;
  defaultGroupParentId.value = parent?.id ?? null;
  groupDialogOpen.value = true;
}

function openEditGroup(group: LocationGroupTreeNode): void {
  closeDialogs();
  editingGroup.value = group;
  defaultGroupParentId.value = group.parent_id;
  groupDialogOpen.value = true;
}

function openDeleteGroup(group: LocationGroupTreeNode): void {
  closeDialogs();
  deleteTarget.value = {
    kind: "group",
    id: group.id,
    label: group.name,
    parentId: group.parent_id,
  };
}

function openCreateLocation(): void {
  closeDialogs();
  editingLocation.value = null;
  locationDialogOpen.value = true;
}

const defaultUpdatingId = ref<number | null>(null);

/** 切换全局默认库位；服务端事务保证唯一，本地据结果同步清掉其它默认标记。 */
async function toggleDefaultLocation(location: LocationResponse): Promise<void> {
  if (defaultUpdatingId.value !== null) return;
  defaultUpdatingId.value = location.id;
  try {
    const updated = await updateLocation(location.id, {
      group_id: location.group_id,
      name: location.name,
      notes: location.notes ?? undefined,
      sort_order: location.sort_order,
      is_default: !location.is_default,
    });
    locations.value = locations.value.map((entry) => {
      if (entry.id === updated.id) return updated;
      return updated.is_default && entry.is_default ? { ...entry, is_default: false } : entry;
    });
    notice.success(
      updated.is_default
        ? t("locations.setDefaultSuccess", { name: updated.name })
        : t("locations.unsetDefaultSuccess"),
      { detail: updated.is_default ? t("locations.setDefaultDetail") : undefined },
    );
  } catch (error) {
    notice.error(t("locations.setDefaultFailed"), {
      detail: locationManagementErrorMessage(error, t("locations.retryLater")),
    });
  } finally {
    defaultUpdatingId.value = null;
  }
}

function openEditLocation(location: LocationResponse): void {
  closeDialogs();
  editingLocation.value = location;
  locationDialogOpen.value = true;
}

function openDeleteLocation(location: LocationResponse): void {
  closeDialogs();
  deleteTarget.value = { kind: "location", id: location.id, label: location.name };
}

function closeDialogs(): void {
  if (actionSubmitting.value) return;
  groupDialogOpen.value = false;
  locationDialogOpen.value = false;
  editingGroup.value = null;
  editingLocation.value = null;
  deleteTarget.value = null;
  actionError.value = "";
  actionFieldErrors.value = {};
}

async function saveGroup(request: LocationGroupUpdateRequest): Promise<void> {
  actionSubmitting.value = true;
  actionError.value = "";
  actionFieldErrors.value = {};
  try {
    const wasEditing = Boolean(editingGroup.value);
    const saved = editingGroup.value
      ? await updateLocationGroup(editingGroup.value.id, request)
      : await createLocationGroup(request);
    if (saved.parent_id !== null) expandGroup(saved.parent_id);
    selectedGroupId.value = saved.id;
    closeDialogsAfterSubmit();
    await Promise.all([loadTree(), loadLocationList()]);
    notice.success(wasEditing ? t("locations.groupUpdated") : t("locations.groupCreated"));
  } catch (error) {
    applyActionError(error, t("locations.saveGroupFailed"));
  } finally {
    actionSubmitting.value = false;
  }
}

async function saveLocation(request: LocationUpdateRequest): Promise<void> {
  actionSubmitting.value = true;
  actionError.value = "";
  actionFieldErrors.value = {};
  try {
    const wasEditing = Boolean(editingLocation.value);
    await (editingLocation.value
      ? updateLocation(editingLocation.value.id, request)
      : createLocation(request));
    closeDialogsAfterSubmit();
    await Promise.all([loadTree(), loadLocationList()]);
    notice.success(wasEditing ? t("locations.locationUpdated") : t("locations.locationCreated"));
  } catch (error) {
    applyActionError(error, t("locations.saveLocationFailed"));
  } finally {
    actionSubmitting.value = false;
  }
}

async function confirmDelete(): Promise<void> {
  const target = deleteTarget.value;
  if (!target) return;
  actionSubmitting.value = true;
  actionError.value = "";
  actionFieldErrors.value = {};
  try {
    if (target.kind === "group") {
      await deleteLocationGroup(target.id);
      selectedGroupId.value = target.parentId ?? null;
    } else {
      await deleteLocation(target.id);
    }
    closeDialogsAfterSubmit();
    await Promise.all([loadTree(), loadLocationList()]);
    notice.success(target.kind === "group" ? t("locations.groupDeleted") : t("locations.locationDeleted"));
  } catch (error) {
    applyActionError(
      error,
      target.kind === "group" ? t("locations.deleteGroupFailed") : t("locations.deleteLocationFailed"),
    );
  } finally {
    actionSubmitting.value = false;
  }
}

function closeDialogsAfterSubmit(): void {
  groupDialogOpen.value = false;
  locationDialogOpen.value = false;
  editingGroup.value = null;
  editingLocation.value = null;
  deleteTarget.value = null;
  actionError.value = "";
  actionFieldErrors.value = {};
}

function expandGroup(groupId: number): void {
  if (!expandedGroupIds.value.includes(groupId))
    expandedGroupIds.value = [...expandedGroupIds.value, groupId];
}

function applyActionError(error: unknown, fallback: string): void {
  const result = locationManagementError(error, fallback);
  actionError.value = result.message;
  actionFieldErrors.value = result.fieldErrors;
  notice.error(result.message, { detail: Object.values(result.fieldErrors)[0] });
}

function findGroup(nodes: LocationGroupTreeNode[], groupId: number): LocationGroupTreeNode | null {
  for (const node of nodes) {
    if (node.id === groupId) return node;
    const child = findGroup(node.children, groupId);
    if (child) return child;
  }
  return null;
}

function findGroupPath(
  nodes: LocationGroupTreeNode[],
  groupId: number,
  path: string[] = [],
): string[] | null {
  for (const node of nodes) {
    const nextPath = [...path, node.name];
    if (node.id === groupId) return nextPath;
    const childPath = findGroupPath(node.children, groupId, nextPath);
    if (childPath) return childPath;
  }
  return null;
}

function flattenGroupOptions(
  nodes: LocationGroupTreeNode[],
  excludedIds: ReadonlySet<number> = new Set(),
  depth = 1,
): LocationGroupOption[] {
  const options: LocationGroupOption[] = [];
  for (const node of nodes) {
    if (excludedIds.has(node.id)) continue;
    options.push({ id: node.id, label: `${"— ".repeat(depth - 1)}${node.name}`, depth });
    options.push(...flattenGroupOptions(node.children, excludedIds, depth + 1));
  }
  return options;
}

function collectGroupIds(node: LocationGroupTreeNode): Set<number> {
  return new Set([
    node.id,
    ...node.children.flatMap((child) => Array.from(collectGroupIds(child))),
  ]);
}

function locationGroupSubtreeHeight(node: LocationGroupTreeNode): number {
  return 1 + Math.max(0, ...node.children.map(locationGroupSubtreeHeight));
}

function locationManagementError(
  error: unknown,
  fallback: string,
): { message: string; fieldErrors: Record<string, string> } {
  if (error instanceof ApiError) {
    const message = translateMessageOrNull(`error.${error.code}`) ?? error.message;
    const fieldErrors = Object.fromEntries(
      Object.entries(error.fieldErrors).map(([path, values]) => [
        path.split(".").at(-1) ?? path,
        values[0] ?? message,
      ]),
    );
    if (error.code === "location_group_name_taken") fieldErrors.name = message;
    if (error.code === "location_group_cycle") fieldErrors.parent_id = message;
    if (error.code === "location_group_depth_exceeded") fieldErrors.parent_id = message;
    if (error.code === "location_name_taken") fieldErrors.name = message;
    if (error.code === "location_group_not_found") {
      fieldErrors[groupDialogOpen.value ? "parent_id" : "group_id"] = message;
    }
    return { message, fieldErrors };
  }
  return { message: locationManagementErrorMessage(error, fallback), fieldErrors: {} };
}

function locationManagementErrorMessage(error: unknown, fallback: string): string {
  if (error instanceof ApiError) return error.message || fallback;
  if (error instanceof ApiNetworkError) return t("locations.networkError");
  if (error instanceof ApiConfigurationError || error instanceof ApiResponseError)
    return error.message;
  return fallback;
}

function isAbortError(error: unknown): boolean {
  return error instanceof DOMException && error.name === "AbortError";
}

function formatDateTime(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime())
    ? value
    : new Intl.DateTimeFormat("zh-CN", {
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
      }).format(date);
}
</script>
