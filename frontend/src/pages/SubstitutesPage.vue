<!--
  本文件拥有全局替代关系的查询、分组、搜索、刷新、权限和编辑 Dialog 编排。
  它只使用 HTTP 契约，不复制单物品替代关系的草稿与保存逻辑。
-->
<template>
  <section class="route-page substitutes-page">
    <header class="content-header substitutes-page__header">
      <div>
        <h1>{{ $title($route.meta.title) }}</h1>
        <p>{{ $t('substitutes.subtitle') }}</p>
      </div>
    </header>

    <section class="substitutes-workspace" :aria-label="$t('substitutes.relationListLabel')">
      <div
        class="substitutes-toolbar"
        :class="{
          'substitutes-toolbar--readonly': !(canManage && canReadItems),
        }"
      >
        <SearchField
          v-model="searchInput"
          class="substitutes-toolbar__search"
          :label="$t('substitutes.searchLabel')"
          name="substitute_relation_search"
          :placeholder="$t('substitutes.searchPlaceholder')"
          :disabled="loading && !loaded"
          @search="applySearch"
        />

        <div class="substitutes-toolbar__commands">
          <div class="substitutes-toolbar__summary">
            <span class="substitutes-toolbar__count">{{ visibleCountLabel }}</span>
            <span
              v-if="showStableRefreshing"
              class="substitutes-toolbar__refresh-status"
              role="status"
              >{{ $t('substitutes.refreshing') }}</span
            >
          </div>
          <div class="substitutes-toolbar__actions">
            <button
              class="icon-button substitutes-toolbar__refresh"
              :class="{ 'substitutes-toolbar__refresh--pending': showStableRefreshing }"
              type="button"
              :title="$t('substitutes.refresh')"
              :aria-label="$t('substitutes.refresh')"
              :aria-busy="loading"
              :disabled="loading"
              @click="refreshRelations"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M20 7v5h-5" />
                <path d="M18.2 16a7 7 0 1 1 .8-7l1 3" />
              </svg>
            </button>
            <button
              class="icon-button substitutes-toolbar__network"
              type="button"
              :title="$t('substitutes.viewNetwork')"
              :aria-label="$t('substitutes.viewNetwork')"
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
              class="icon-button icon-button--primary substitutes-toolbar__create"
              type="button"
              :title="$t('substitutes.create')"
              :aria-label="$t('substitutes.create')"
              @click="openCreate"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
            </button>
          </div>
        </div>
      </div>

      <div
        v-overlay-scrollbar
        class="substitutes-results"
        :class="{ 'substitutes-results--refreshing': showStableRefreshing }"
        :aria-busy="loading"
      >
        <div
          v-if="loadError && !loaded"
          class="substitutes-state substitutes-state--error"
          role="alert"
        >
          <strong>{{ $t('substitutes.loadFailedTitle') }}</strong>
          <span>{{ loadError }}</span>
          <button class="secondary-button" type="button" @click="retryLoad">
            {{ $t('common.retry') }}
          </button>
        </div>
        <div v-else-if="loading && !loaded" class="substitutes-state" role="status">
          <span v-if="showInitialLoading">{{ $t('substitutes.loadingRelations') }}</span>
        </div>
        <div v-else-if="!visibleGroups.length" class="substitutes-state">
          <strong>{{
            activeSearch
              ? $t('substitutes.noSearchMatches')
              : $t('substitutes.noRelations')
          }}</strong>
          <span>{{
            activeSearch
              ? $t('substitutes.noSearchMatchesHint')
              : canManage && canReadItems
                ? $t('substitutes.noRelationsCreateHint')
                : $t('substitutes.noRelationsReadonlyHint')
          }}</span>
          <button v-if="activeSearch" class="text-button" type="button" @click="clearSearch">
            {{ $t('substitutes.clearSearch') }}
          </button>
        </div>
        <template v-else>
          <p v-if="loadError" class="substitutes-results__inline-error" role="alert">
            {{ loadError }}
          </p>
          <div class="substitutes-table" role="table" :aria-label="$t('substitutes.tableLabel')">
            <div class="substitutes-table__head" role="row">
              <span role="columnheader">{{ $t('substitutes.columnMainItem') }}</span>
              <span role="columnheader">{{ $t('substitutes.columnSummary') }}</span>
              <span role="columnheader">{{ $t('substitutes.columnActions') }}</span>
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
      :title="$t('substitutes.leaveTitle')"
      :description="$t('substitutes.leaveDescription')"
      nested
      compact
      @close="cancelRouteLeave"
    >
      <p class="confirmation-copy">{{ $t('substitutes.leaveHint') }}</p>
      <template #actions>
        <button class="secondary-button" type="button" @click="cancelRouteLeave">
          {{ $t('substitutes.continueEditing') }}
        </button>
        <button class="danger-button" type="button" @click="confirmRouteLeave">
          {{ $t('substitutes.discardAndLeave') }}
        </button>
      </template>
    </ModalDialog>
  </section>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { onBeforeRouteLeave } from "vue-router";
import { listSubstituteRelations, type SubstituteRelationResponse } from "../api/substitutes";
import { authSession } from "../auth/session";
import { hasPermission, stockPermissions } from "../auth/permissions";
import ModalDialog from "../components/ModalDialog.vue";
import SearchField from "../components/SearchField.vue";
import SubstituteEditorDialog from "../components/substitutes/SubstituteEditorDialog.vue";
import SubstituteNetworkDialog from "../components/substitutes/SubstituteNetworkDialog.vue";
import SubstituteRelationGroup from "../components/substitutes/SubstituteRelationGroup.vue";
import { useStablePendingIndicator } from "../composables/useStablePendingIndicator";
import { notice } from "../notices/notice";
import {
  countGroupedRelations,
  filterSubstituteRelationGroups,
  groupSubstituteRelations,
  type SubstituteEditorTarget,
  type SubstituteRelationGroupModel,
} from "./substitutes/model";
import { formatRelationCount, substituteErrorMessage } from "./substitutes/presentation";
import "./SubstitutesPage.scss";

const { t } = useI18n();
const relations = ref<SubstituteRelationResponse[]>([]);
const loaded = ref(false);
const loading = ref(false);
const loadError = ref("");
const searchInput = ref("");
const activeSearch = ref("");
const editorOpen = ref(false);
const editorTarget = ref<SubstituteEditorTarget | null>(null);
const editorDirty = ref(false);
const networkOpen = ref(false);
const returnToNetworkAfterEditor = ref(false);
const routeDiscardOpen = ref(false);
let requestController: AbortController | null = null;
let pendingLeaveResolution: ((allow: boolean) => void) | null = null;

const canManage = computed(() =>
  hasPermission(authSession.value?.user.permissions, stockPermissions.substituteManage),
);
const canReadItems = computed(() =>
  hasPermission(authSession.value?.user.permissions, stockPermissions.itemRead),
);
const groups = computed(() => groupSubstituteRelations(relations.value));
const visibleGroups = computed(() =>
  filterSubstituteRelationGroups(groups.value, activeSearch.value),
);
const visibleCountLabel = computed(() =>
  formatRelationCount(visibleGroups.value.length, countGroupedRelations(visibleGroups.value)),
);
const showInitialLoading = useStablePendingIndicator(
  computed(() => loading.value && !loaded.value),
  { showDelayMs: 200, minimumVisibleMs: 350 },
);
const showStableRefreshing = useStablePendingIndicator(
  computed(() => loading.value && loaded.value),
  { showDelayMs: 200, minimumVisibleMs: 350 },
);

onMounted(() => {
  window.addEventListener("beforeunload", handleBeforeUnload);
  void loadRelations();
});

onBeforeUnmount(() => {
  requestController?.abort();
  window.removeEventListener("beforeunload", handleBeforeUnload);
  pendingLeaveResolution?.(false);
});

onBeforeRouteLeave(() => {
  if (!editorDirty.value) return true;
  routeDiscardOpen.value = true;
  return new Promise<boolean>((resolve) => {
    pendingLeaveResolution = resolve;
  });
});

async function loadRelations(showSuccessNotice = false): Promise<boolean> {
  requestController?.abort();
  const controller = new AbortController();
  requestController = controller;
  loading.value = true;
  if (!loaded.value) loadError.value = "";
  try {
    relations.value = await listSubstituteRelations(controller.signal);
    loaded.value = true;
    loadError.value = "";
    if (showSuccessNotice) notice.success(t("substitutes.refreshed"));
    return true;
  } catch (error) {
    if (error instanceof DOMException && error.name === "AbortError") return false;
    loadError.value = substituteErrorMessage(error);
    if (loaded.value) notice.error(t("substitutes.refreshFailed"), { detail: loadError.value });
    return false;
  } finally {
    if (requestController === controller) {
      requestController = null;
      loading.value = false;
    }
  }
}

function refreshRelations(): void {
  void loadRelations(true);
}

function retryLoad(): void {
  void loadRelations();
}

function applySearch(value: string): void {
  activeSearch.value = value.trim();
}

function clearSearch(): void {
  searchInput.value = "";
  activeSearch.value = "";
}

function openCreate(): void {
  returnToNetworkAfterEditor.value = false;
  editorTarget.value = null;
  editorOpen.value = true;
}

function openExisting(group: SubstituteRelationGroupModel): void {
  returnToNetworkAfterEditor.value = false;
  editorTarget.value = { id: group.itemId, name: group.itemName, sku: group.itemSku };
  editorOpen.value = true;
}

function closeEditor(): void {
  editorOpen.value = false;
  editorTarget.value = null;
  editorDirty.value = false;
  if (returnToNetworkAfterEditor.value) {
    returnToNetworkAfterEditor.value = false;
    networkOpen.value = true;
  }
}

function handleSaved(): void {
  editorDirty.value = false;
  void loadRelations();
}

function editFromNetwork(target: SubstituteEditorTarget): void {
  networkOpen.value = false;
  returnToNetworkAfterEditor.value = true;
  editorTarget.value = target;
  editorOpen.value = true;
}

function cancelRouteLeave(): void {
  routeDiscardOpen.value = false;
  pendingLeaveResolution?.(false);
  pendingLeaveResolution = null;
}

function confirmRouteLeave(): void {
  routeDiscardOpen.value = false;
  editorDirty.value = false;
  editorOpen.value = false;
  networkOpen.value = false;
  returnToNetworkAfterEditor.value = false;
  pendingLeaveResolution?.(true);
  pendingLeaveResolution = null;
}

function handleBeforeUnload(event: BeforeUnloadEvent): void {
  if (!editorDirty.value) return;
  event.preventDefault();
  event.returnValue = "";
}
</script>
