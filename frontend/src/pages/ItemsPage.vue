<!--
  本文件拥有物品目录库存工作台、服务端筛选排序和物品 Dialog 会话编排。
  目录只持有目录投影；编辑资料、库存详情和批次分别按需请求。
-->
<template>
  <section class="route-page items-page">
    <header class="content-header items-page__header">
      <div>
        <h1>{{ $title($route.meta.title) }}</h1>
        <p>{{ $t('items.subtitle') }}</p>
      </div>
    </header>

    <div class="items-page__workspace">
      <section class="items-catalog" :aria-label="$t('items.catalogAriaLabel')">
        <div class="items-catalog__toolbar">
          <SearchField
            v-model="searchInput"
            class="items-catalog__search"
            :label="$t('items.searchItems')"
            name="item_search"
            :placeholder="$t('items.searchPlaceholder')"
            @search="applySearch"
          />
          <div class="items-catalog__toolbar-actions">
            <label class="items-catalog__filter">
              <span>{{ $t('items.stockFilter') }}</span>
              <SelectControl
                v-model="activeFilter"
                name="item_catalog_filter"
                compact
                @change="reloadCatalog"
              >
                <option v-for="filter in stockFilters" :key="filter.value" :value="filter.value">
                  {{ $t('items.filterWithCount', { label: filter.label, count: filter.count }) }}
                </option>
              </SelectControl>
            </label>
            <label class="items-catalog__sort">
              <span>{{ $t('items.sort') }}</span>
              <SelectControl
                v-model="activeSort"
                name="item_catalog_sort"
                compact
                @change="reloadCatalog"
              >
                <option value="replenishment_priority">
                  {{ $t('items.sortReplenishmentPriority') }}
                </option>
                <option value="name">{{ $t('items.sortName') }}</option>
                <option value="quantity_asc">{{ $t('items.sortQuantityAsc') }}</option>
                <option value="quantity_desc">{{ $t('items.sortQuantityDesc') }}</option>
                <option value="inventory_value_desc">
                  {{ $t('items.sortInventoryValueDesc') }}
                </option>
                <option value="updated_desc">{{ $t('items.sortUpdatedDesc') }}</option>
              </SelectControl>
            </label>
            <div class="items-catalog__commands">
              <span
                v-if="showStableCatalogLoading && items.length"
                class="items-catalog__refresh-status"
                role="status"
                >{{ $t('items.refreshing') }}</span
              >
              <button
                class="icon-button items-catalog__advanced-filter"
                type="button"
                :title="advancedFilterLabel"
                :aria-label="advancedFilterLabel"
                @click="openCatalogFilters"
              >
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  <path d="M4 6h16M7 12h10M10 18h4" />
                </svg>
                <span v-if="activeAdvancedFilterCount" aria-hidden="true">{{
                  activeAdvancedFilterCount
                }}</span>
              </button>
              <button
                class="icon-button items-catalog__refresh"
                :class="{ 'is-pending': showStableCatalogLoading }"
                type="button"
                :title="$t('items.refreshCatalog')"
                :aria-label="$t('items.refreshCatalog')"
                :disabled="catalogPending"
                @click="requestRefreshCatalog"
              >
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  <path d="M20 7v5h-5" />
                  <path d="M18.2 16a7 7 0 1 1 .8-7l1 3" />
                </svg>
              </button>
              <button
                v-if="canManageTemplates"
                class="icon-button items-catalog__display-settings"
                type="button"
                :title="$t('items.displaySettings')"
                :aria-label="$t('items.displaySettings')"
                :disabled="!templates.length"
                @click="catalogAttributeDialogOpen = true"
              >
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  <path d="M4 6h10M18 6h2M4 12h2M10 12h10M4 18h7M15 18h5" />
                  <circle cx="16" cy="6" r="2" />
                  <circle cx="8" cy="12" r="2" />
                  <circle cx="13" cy="18" r="2" />
                </svg>
              </button>
              <button
                v-if="canManageItems"
                class="icon-button icon-button--primary items-catalog__create"
                type="button"
                :title="$t('items.createItem')"
                :aria-label="$t('items.createItem')"
                @click="requestStartNew"
              >
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
              </button>
            </div>
          </div>
        </div>

        <div
          v-overlay-scrollbar
          class="items-catalog__body"
          :class="{ 'items-catalog__body--refreshing': showStableCatalogLoading && items.length }"
          :aria-busy="catalogPending"
        >
          <div
            v-if="loadError && !items.length"
            class="items-catalog__state items-catalog__state--error"
            role="alert"
          >
            <strong>{{ $t('items.loadFailed') }}</strong><span>{{ loadError }}</span
            ><button class="secondary-button" type="button" @click="reloadCatalog">{{
              $t('items.retry')
            }}</button>
          </div>
          <div v-else-if="showCatalogLoadingState" class="items-catalog__state" role="status">
            {{ $t('items.loadingItems') }}
          </div>
          <div v-else-if="!items.length" class="items-catalog__state">
            <strong>{{
              hasActiveCatalogConditions ? $t('items.noMatchingItems') : $t('items.noItemsYet')
            }}</strong>
            <button
              v-if="hasActiveCatalogConditions"
              class="text-button"
              type="button"
              @click="clearFilters"
            >
              {{ $t('items.clearFilters') }}
            </button>
          </div>
          <template v-else>
            <div class="items-catalog__table" role="table" :aria-label="$t('items.catalogTableAriaLabel')">
              <div class="items-catalog__table-head" role="row">
                <span>{{ $t('items.item') }}</span
                ><span>{{ $t('items.keyAttributes') }}</span
                ><span>{{ $t('items.stockAndData') }}</span
                ><span>{{ $t('items.stockJudgement') }}</span>
              </div>
              <article
                v-for="item in items"
                :key="item.id"
                class="items-catalog__item"
                :class="{
                  'items-catalog__item--selected': editorOpen && selectedItemId === item.id,
                }"
                role="row"
                tabindex="0"
                @click="requestOpenItem(item)"
                @keydown.enter="requestOpenItem(item)"
              >
                <div class="items-catalog__fixed-info" role="cell">
                  <AuthenticatedImage
                    :file-id="item.image_file_id"
                    :alt="$t('items.mainImageName', { name: item.name })"
                    :size="76"
                    previewable
                    @click.stop
                  />
                  <div class="items-catalog__identity-content">
                    <strong
                      v-copyable="{ text: item.name, label: $t('items.itemName') }"
                      class="items-catalog__identity-name"
                      :title="item.name"
                      @click.stop
                      @keydown.stop
                      >{{ item.name }}</strong
                    >
                    <dl class="items-catalog__identity-meta">
                      <div class="items-catalog__identity-sku">
                        <dt>{{ $t('items.sku') }}</dt>
                        <!-- 点击复制不得触发行级打开编辑，必须阻断冒泡。 -->
                        <dd
                          v-copyable="{ text: item.sku, label: $t('items.itemSkuLabel') }"
                          :title="item.sku"
                          @click.stop
                          @keydown.stop
                        >
                          {{ item.sku }}
                        </dd>
                      </div>
                      <div class="items-catalog__identity-category">
                        <dt>{{ $t('items.category') }}</dt>
                        <dd :title="item.category_name ?? $t('items.uncategorized')">
                          {{ item.category_name ?? $t('items.uncategorized') }}
                        </dd>
                      </div>
                    </dl>
                  </div>
                  <div class="items-catalog__row-actions">
                    <button
                      class="icon-button"
                      type="button"
                      :title="$t('items.viewItemDetails')"
                      :aria-label="$t('items.viewItemDetailsNamed', { name: item.name })"
                      @click.stop="requestOpenItem(item)"
                    >
                      <svg viewBox="0 0 24 24" aria-hidden="true">
                        <circle cx="12" cy="12" r="8" />
                        <path d="M12 11v5M12 8h.01" />
                      </svg>
                    </button>
                    <button
                      v-if="canManageItems"
                      class="icon-button items-catalog__delete"
                      type="button"
                      :title="$t('items.deleteItem')"
                      :aria-label="$t('items.deleteItemNamed', { name: item.name })"
                      @click.stop="requestDeleteItem(item)"
                    >
                      <svg viewBox="0 0 24 24" aria-hidden="true">
                        <path d="M4 7h16M10 11v5M14 11v5M9 7l1-2h4l1 2M7 7l1 13h8l1-13" />
                      </svg>
                    </button>
                  </div>
                </div>
                <dl class="items-catalog__attributes" role="cell">
                  <div v-for="attribute in item.catalog_attributes" :key="attribute.name">
                    <dt :title="attribute.name">{{ attribute.name }}</dt>
                    <dd :title="catalogAttributeText(attribute)">
                      {{ catalogAttributeText(attribute) }}
                    </dd>
                  </div>
                  <div v-if="!item.catalog_attributes.length" class="is-empty">
                    <dd>{{ $t('items.noDisplayAttributes') }}</dd>
                  </div>
                </dl>
                <dl class="items-catalog__details" role="cell">
                  <div>
                    <dt>{{ $t('items.inventoryValue') }}</dt>
                    <dd :title="formatMoney(item.inventory_value)">
                      {{ formatMoney(item.inventory_value) }}
                    </dd>
                  </div>
                  <div>
                    <dt>{{ $t('items.locationsAndBatches') }}</dt>
                    <dd :title="`${item.location_count} / ${item.batch_count}`">
                      {{ item.location_count }} / {{ item.batch_count }}
                    </dd>
                  </div>
                  <div>
                    <dt>{{ $t('items.referencePrice') }}</dt>
                    <dd
                      :title="
                        item.default_price === null ? $t('items.notSet') : formatMoney(item.default_price)
                      "
                    >
                      {{
                        item.default_price === null
                          ? $t('items.notSet')
                          : formatMoney(item.default_price)
                      }}
                    </dd>
                  </div>
                  <div>
                    <dt>{{ $t('items.dataUpdated') }}</dt>
                    <dd :title="formatDateTime(item.updated_at)">
                      {{ formatDateTime(item.updated_at) }}
                    </dd>
                  </div>
                </dl>
                <div class="items-catalog__stock" role="cell">
                  <strong
                    >{{ formatQuantity(item.current_quantity) }}
                    <small>{{ item.unit }}</small></strong
                  >
                  <span
                    >{{
                      $t('items.reorderPointPrefix', {
                        value:
                          item.reorder_point === null
                            ? $t('items.notSet')
                            : formatQuantity(item.reorder_point),
                      })
                    }}</span
                  >
                  <em :class="`stock-state stock-state--${item.stock_state}`">{{
                    stockStateLabel(item.stock_state)
                  }}</em>
                </div>
              </article>
            </div>
            <div ref="loadMoreSentinel" class="items-catalog__load-more" aria-live="polite">
              <span v-if="showStableLoadingMore" role="status">{{ $t('items.loadingMore') }}</span>
              <template v-else-if="loadMoreError"
                ><span>{{ loadMoreError }}</span
                ><button class="text-button" type="button" @click="loadNextPage">
                  {{ $t('items.retry') }}
                </button></template
              >
              <button
                v-else-if="hasMoreItems"
                class="text-button"
                type="button"
                :disabled="loadingMore"
                @click="loadNextPage"
              >
                {{ $t('items.loadMore') }}
              </button>
              <span v-else>{{ $t('items.loadedAllItems', { n: total }) }}</span>
            </div>
          </template>
        </div>
      </section>
    </div>

    <ItemEditorDialog
      :open="editorOpen"
      :mode="dialogMode"
      :item-id="selectedItemId"
      :item-name="selectedCatalogItem?.name"
      :item-sku="selectedCatalogItem?.sku"
      :initial-page="dialogInitialPage"
      :draft="draft"
      :categories="categories"
      :templates="templates"
      :saving="saving"
      :data-loading="editorDataLoading"
      :data-ready="editorDataReady"
      :data-error="editorDataError"
      :metadata-error="metadataError"
      :validation-errors="validationErrors"
      :read-only="!canManageItems"
      :can-view-substitutes="canViewSubstitutes"
      :can-manage-substitutes="canManageSubstitutes"
      allow-lcsc-lookup
      @request-data="loadSelectedEditor"
      @save="save"
      @close="requestCloseEditor"
      @substitutes-dirty="substituteDirty = $event"
      @apply-lcsc="applyLcscLookup"
    />

    <ItemCatalogAttributeDialog
      :open="catalogAttributeDialogOpen"
      :templates="templates"
      @close="catalogAttributeDialogOpen = false"
      @saved="handleCatalogAttributeSaved"
    />

    <ItemCatalogFilterDialog
      :open="filterDialogOpen"
      :applied="appliedCatalogFilters"
      :fields="filterFields"
      :categories="categories"
      :templates="templates"
      :loading="filterValuesLoading"
      @close="filterDialogOpen = false"
      @apply="requestApplyCatalogFilters"
    />

    <ModalDialog
      :open="discardDialogOpen"
      :title="$t('items.discardChangesTitle')"
      :description="$t('items.discardChangesDescription')"
      @close="cancelPendingTransition"
    >
      <p>{{ $t('items.discardChangesConfirm') }}</p>
      <template #actions
        ><button class="secondary-button" type="button" @click="cancelPendingTransition">
          {{ $t('items.continueEditing') }}</button
        ><button class="danger-button" type="button" @click="confirmPendingTransition">
          {{ $t('items.discardChanges') }}
        </button></template
      >
    </ModalDialog>

    <ModalDialog
      :open="deleteItemTarget !== null"
      :title="$t('items.deleteItem')"
      :busy="deletingItem"
      @close="cancelDeleteItem"
    >
      <p>{{ $t('items.deleteItemDescription') }}</p>
      <p v-if="deleteItemError" class="form-error" role="alert">{{ deleteItemError }}</p>
      <template #actions>
        <button
          class="secondary-button"
          type="button"
          :disabled="deletingItem"
          @click="cancelDeleteItem"
        >
          {{ $t('items.cancel') }}
        </button>
        <button
          class="danger-button"
          type="button"
          :disabled="deletingItem"
          @click="confirmDeleteItem"
        >
          {{ deletingItem ? $t('items.deleting') : $t('items.confirmDelete') }}
        </button>
      </template>
    </ModalDialog>
  </section>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { onBeforeRouteLeave } from "vue-router";
import {
  cloneItemCatalogFilters,
  createItem,
  deleteItem,
  emptyItemCatalogFilters,
  getItem,
  getItemFilterValues,
  listItemCatalog,
  updateItem,
  type CatalogAttributeResponse,
  type ItemCatalogFilters,
  type ItemCatalogCountsResponse,
  type ItemCatalogEntryResponse,
  type ItemCatalogSort,
  type ItemFilterFieldResponse,
  type ItemStockFilter,
  type ItemStockState,
  type LcscItemLookupResponse,
} from "../api/items";
import { prepareLcscItemImage } from "../components/items/lcscImageDraft";
import { listItemCategories, type ItemCategoryResponse } from "../api/itemCategories";
import {
  listItemAttributeTemplates,
  type ItemAttributeTemplateResponse,
} from "../api/itemAttributeTemplates";
import ItemEditorDialog from "../components/items/ItemEditorDialog.vue";
import ItemCatalogAttributeDialog from "../components/items/ItemCatalogAttributeDialog.vue";
import ItemCatalogFilterDialog from "../components/items/ItemCatalogFilterDialog.vue";
import AuthenticatedImage from "../components/attributes/AuthenticatedImage.vue";
import ModalDialog from "../components/ModalDialog.vue";
import { ApiError } from "../api/errors";
import { notice } from "../notices/notice";
import { translateMessageOrNull } from "../i18n";
import {
  draftFromItem,
  emptyItemDraft,
  applyDefaultTemplateToPristineDraft,
  applyLcscLookupToDraft,
  itemCreateRequest,
  itemDraftFingerprint,
  itemUpdateRequest,
  itemDraftValidationFromApiError,
  validateItemDraft,
  type ItemDraft,
} from "./items/model";
import { discardTemporaryItemFiles } from "./items/fileCleanup";
import {
  createPendingImageDraft,
  isImageDraftValue,
  releaseImageDraft,
  uploadImageDrafts,
} from "../components/attributes/imageDraft";
import { deleteImage } from "../api/files";
import { useStablePendingIndicator } from "../composables/useStablePendingIndicator";
import { useFormValidation } from "../composables/useFormValidation";
import { authSession } from "../auth/session";
import { hasPermission, stockPermissions } from "../auth/permissions";
import SearchField from "../components/SearchField.vue";
import SelectControl from "../components/forms/SelectControl.vue";
import "./ItemsPage.scss";

const PAGE_SIZE = 50;
const { t } = useI18n();
const EMPTY_COUNTS: ItemCatalogCountsResponse = {
  total: 0,
  needs_attention: 0,
  out_of_stock: 0,
  reorder_due: 0,
  needs_configuration: 0,
};

const items = ref<ItemCatalogEntryResponse[]>([]);
const counts = ref<ItemCatalogCountsResponse>({ ...EMPTY_COUNTS });
const categories = ref<ItemCategoryResponse[]>([]);
const templates = ref<ItemAttributeTemplateResponse[]>([]);
const draft = ref<ItemDraft>(emptyItemDraft());
const baselineDraft = ref<ItemDraft>(emptyItemDraft());
const baselineFingerprint = ref(itemDraftFingerprint(draft.value));
const searchInput = ref("");
const activeSearch = ref("");
const activeFilter = ref<ItemStockFilter>("all");
const activeSort = ref<ItemCatalogSort>("replenishment_priority");
const appliedCatalogFilters = ref<ItemCatalogFilters>(emptyItemCatalogFilters());
const filterFields = ref<ItemFilterFieldResponse[]>([]);
const filterDialogOpen = ref(false);
const filterValuesLoading = ref(false);
const total = ref(0);
const page = ref(1);
const totalPages = ref(0);
const loading = ref(true);
const loadingMore = ref(false);
const saving = ref(false);
const editorDataLoading = ref(false);
const editorDataReady = ref(true);
const editorDataError = ref("");
const loadError = ref("");
const loadMoreError = ref("");
const metadataError = ref("");
const deleteItemTarget = ref<ItemCatalogEntryResponse | null>(null);
const deletingItem = ref(false);
const deleteItemError = ref("");
const substituteDirty = ref(false);
const editorOpen = ref(false);
const dialogMode = ref<"create" | "existing">("create");
const dialogInitialPage = ref<"data" | "inventory">("data");
const selectedItemId = ref<number | null>(null);
const selectedCatalogItem = ref<ItemCatalogEntryResponse | null>(null);
const discardDialogOpen = ref(false);
const catalogAttributeDialogOpen = ref(false);
const loadMoreSentinel = ref<HTMLElement | null>(null);
const validationErrors = ref<Record<string, string>>({});
useFormValidation(validationErrors);
const emptyCatalogLoadingGate = ref(true);

let catalogAbortController: AbortController | null = null;
let editorAbortController: AbortController | null = null;
let filterValuesAbortController: AbortController | null = null;
let loadMoreObserver: IntersectionObserver | null = null;
let pendingTransition: (() => Promise<void>) | null = null;
let pendingLeaveResolution: ((allowed: boolean) => void) | null = null;

const hasMoreItems = computed(() => page.value < totalPages.value);
const canManageItems = computed(() =>
  hasPermission(authSession.value?.user.permissions, stockPermissions.itemManage),
);
const canViewSubstitutes = computed(() =>
  hasPermission(authSession.value?.user.permissions, stockPermissions.substituteRead),
);
const canManageSubstitutes = computed(() =>
  hasPermission(authSession.value?.user.permissions, stockPermissions.substituteManage),
);
const canManageTemplates = computed(() =>
  hasPermission(authSession.value?.user.permissions, stockPermissions.templateManage),
);
const catalogPending = computed(() => loading.value || loadingMore.value);
const activeAdvancedFilterCount = computed(
  () =>
    Number(appliedCatalogFilters.value.categoryId !== null) +
    Number(appliedCatalogFilters.value.attributeTemplateId !== null) +
    Object.values(appliedCatalogFilters.value.fields).filter((values) => values.length).length,
);
const hasActiveCatalogConditions = computed(
  () =>
    Boolean(activeSearch.value) ||
    activeFilter.value !== "all" ||
    activeAdvancedFilterCount.value > 0,
);
const advancedFilterLabel = computed(() =>
  activeAdvancedFilterCount.value
    ? t("items.advancedFilterActive", { n: activeAdvancedFilterCount.value })
    : t("items.advancedFilter"),
);
const showStableCatalogLoading = useStablePendingIndicator(loading, {
  showDelayMs: 200,
  minimumVisibleMs: 350,
});
const showStableLoadingMore = useStablePendingIndicator(loadingMore, {
  showDelayMs: 200,
  minimumVisibleMs: 350,
});
const showCatalogLoadingState = computed(
  () => emptyCatalogLoadingGate.value || (showStableCatalogLoading.value && !items.value.length),
);
const hasUnsavedChanges = computed(
  () =>
    (dialogMode.value === "create" || Boolean(draft.value.id)
      ? itemDraftFingerprint(draft.value) !== baselineFingerprint.value
      : false) || substituteDirty.value,
);
const stockFilters = computed(() => [
  { value: "all" as const, label: t("items.filterAll"), count: counts.value.total },
  {
    value: "needs_attention" as const,
    label: t("items.filterNeedsAttention"),
    count: counts.value.needs_attention,
  },
  {
    value: "out_of_stock" as const,
    label: t("items.stockStateOutOfStock"),
    count: counts.value.out_of_stock,
  },
  {
    value: "reorder_due" as const,
    label: t("items.stockStateReorderDue"),
    count: counts.value.reorder_due,
  },
  {
    value: "needs_configuration" as const,
    label: t("items.stockStateNeedsConfiguration"),
    count: counts.value.needs_configuration,
  },
]);

watch(loadMoreSentinel, (element, previous) => {
  if (previous) loadMoreObserver?.unobserve(previous);
  if (element) loadMoreObserver?.observe(element);
});
watch([loading, showStableCatalogLoading], ([pending, visible]) => {
  if (!pending && !visible) emptyCatalogLoadingGate.value = false;
});
watch(
  () => itemDraftFingerprint(draft.value),
  () => {
    if (Object.keys(validationErrors.value).length) validationErrors.value = {};
  },
);

onMounted(() => {
  loadMoreObserver = new IntersectionObserver(
    (entries) => {
      if (entries.some((entry) => entry.isIntersecting)) void loadNextPage();
    },
    { rootMargin: "220px 0px" },
  );
  window.addEventListener("beforeunload", handleBeforeUnload);
  void Promise.all([loadMetadata(), loadCatalog(1)]);
});

onBeforeUnmount(() => {
  catalogAbortController?.abort();
  editorAbortController?.abort();
  filterValuesAbortController?.abort();
  loadMoreObserver?.disconnect();
  window.removeEventListener("beforeunload", handleBeforeUnload);
  pendingLeaveResolution?.(false);
  void discardCurrentTemporaryFiles();
});

onBeforeRouteLeave(() => {
  if (!editorOpen.value || !hasUnsavedChanges.value) return true;
  discardDialogOpen.value = true;
  return new Promise<boolean>((resolve) => {
    pendingLeaveResolution = resolve;
  });
});

async function loadMetadata(): Promise<void> {
  metadataError.value = "";
  try {
    [categories.value, templates.value] = await Promise.all([
      listItemCategories(),
      listItemAttributeTemplates(),
    ]);
  } catch (error) {
    metadataError.value = errorMessage(error);
    notice.error(t("items.editorOptionsLoadFailed"), { detail: metadataError.value });
  }
}

async function loadCatalog(targetPage: number, append = false): Promise<void> {
  catalogAbortController?.abort();
  const controller = new AbortController();
  catalogAbortController = controller;
  const shouldAppend = append && items.value.length > 0;
  if (!shouldAppend && !items.value.length) emptyCatalogLoadingGate.value = true;
  loading.value = !shouldAppend;
  loadingMore.value = shouldAppend;
  loadMoreError.value = "";
  if (!shouldAppend) loadError.value = "";
  try {
    const response = await listItemCatalog(
      activeSearch.value,
      targetPage,
      PAGE_SIZE,
      activeFilter.value,
      activeSort.value,
      appliedCatalogFilters.value,
      controller.signal,
    );
    items.value = shouldAppend ? mergeItems(items.value, response.items) : response.items;
    counts.value = response.counts;
    total.value = response.total;
    page.value = response.page;
    totalPages.value = response.total_pages;
    await nextTick();
    refreshLoadMoreObservation();
  } catch (error) {
    if (error instanceof DOMException && error.name === "AbortError") return;
    const message = errorMessage(error);
    if (shouldAppend) loadMoreError.value = message;
    else loadError.value = message;
    notice.error(shouldAppend ? t("items.loadMoreItemsFailed") : t("items.loadItemsFailed"), {
      detail: message,
    });
  } finally {
    if (catalogAbortController === controller) {
      catalogAbortController = null;
      loading.value = false;
      loadingMore.value = false;
    }
  }
}

async function reloadCatalog(): Promise<void> {
  await loadCatalog(1);
}
async function handleCatalogAttributeSaved(updated: ItemAttributeTemplateResponse): Promise<void> {
  const index = templates.value.findIndex((template) => template.id === updated.id);
  if (index >= 0) templates.value[index] = updated;
  await reloadCatalog();
}
function applySearch(value: string): void {
  if (value !== activeSearch.value) {
    activeSearch.value = value;
    void reloadCatalog();
  }
}
function clearFilters(): void {
  searchInput.value = "";
  activeSearch.value = "";
  activeFilter.value = "all";
  appliedCatalogFilters.value = emptyItemCatalogFilters();
  void reloadCatalog();
}
function openCatalogFilters(): void {
  filterDialogOpen.value = true;
  void loadFilterValues();
}
async function loadFilterValues(): Promise<void> {
  filterValuesAbortController?.abort();
  const controller = new AbortController();
  filterValuesAbortController = controller;
  filterValuesLoading.value = true;
  try {
    const response = await getItemFilterValues(
      activeSearch.value,
      activeFilter.value,
      appliedCatalogFilters.value,
      controller.signal,
    );
    filterFields.value = response.fields;
  } catch (error) {
    if (error instanceof DOMException && error.name === "AbortError") return;
    const message = errorMessage(error);
    notice.error(t("items.loadFiltersFailed"), {
      detail: message,
      onClick: () => void loadFilterValues(),
    });
  } finally {
    if (filterValuesAbortController === controller) {
      filterValuesAbortController = null;
      filterValuesLoading.value = false;
    }
  }
}
function requestApplyCatalogFilters(filters: ItemCatalogFilters): void {
  filterDialogOpen.value = false;
  requestDraftTransition(() => applyCatalogFilters(filters));
}
async function applyCatalogFilters(filters: ItemCatalogFilters): Promise<void> {
  appliedCatalogFilters.value = cloneItemCatalogFilters(filters);
  filterDialogOpen.value = false;
  await clearEditor();
  await reloadCatalog();
}
function requestRefreshCatalog(): void {
  requestDraftTransition(refreshCatalog);
}
async function refreshCatalog(): Promise<void> {
  await clearEditor();
  await reloadCatalog();
  if (!loadError.value) notice.success(t("items.catalogRefreshed"));
}
async function loadNextPage(): Promise<void> {
  if (!catalogPending.value && hasMoreItems.value) await loadCatalog(page.value + 1, true);
}
function refreshLoadMoreObservation(): void {
  const sentinel = loadMoreSentinel.value;
  if (sentinel && loadMoreObserver) {
    loadMoreObserver.unobserve(sentinel);
    loadMoreObserver.observe(sentinel);
  }
}

function requestStartNew(): void {
  if (canManageItems.value) requestDraftTransition(prepareNewDraft);
}
function requestOpenItem(item: ItemCatalogEntryResponse): void {
  requestDraftTransition(() => openExisting(item, "data"));
}
function requestDeleteItem(item: ItemCatalogEntryResponse): void {
  if (canManageItems.value) requestDraftTransition(() => openDeleteItemDialog(item));
}

/** 删除前先处理可能仍在编辑的草稿，避免高风险操作静默丢弃修改。 */
async function openDeleteItemDialog(item: ItemCatalogEntryResponse): Promise<void> {
  await clearEditor();
  deleteItemError.value = "";
  deleteItemTarget.value = item;
}

function cancelDeleteItem(): void {
  if (deletingItem.value) return;
  deleteItemTarget.value = null;
  deleteItemError.value = "";
}

/** 服务端执行物品软删除；成功后重建目录分页并关闭可能打开的对应编辑会话。 */
async function confirmDeleteItem(): Promise<void> {
  const target = deleteItemTarget.value;
  if (!target || deletingItem.value) return;
  deletingItem.value = true;
  deleteItemError.value = "";
  try {
    await deleteItem(target.id);
    deleteItemTarget.value = null;
    await clearEditor();
    await reloadCatalog();
    notice.success(t("items.itemDeleted"), { detail: target.name });
  } catch (error) {
    deleteItemError.value = errorMessage(error);
    notice.error(t("items.deleteItemFailed"), { detail: deleteItemError.value });
  } finally {
    deletingItem.value = false;
  }
}

async function prepareNewDraft(): Promise<void> {
  await clearEditor();
  dialogMode.value = "create";
  dialogInitialPage.value = "data";
  editorDataReady.value = true;
  draft.value = emptyItemDraft();
  baselineDraft.value = emptyItemDraft();
  // 全站默认模板预填新建草稿；基线同步应用，预填不计入未保存修改。
  applyDefaultTemplateToPristineDraft(draft.value, templates.value);
  applyDefaultTemplateToPristineDraft(baselineDraft.value, templates.value);
  baselineFingerprint.value = itemDraftFingerprint(draft.value);
  editorOpen.value = true;
}

async function applyLcscLookup(
  candidate: LcscItemLookupResponse,
  templateId: number | null,
): Promise<void> {
  if (dialogMode.value !== "create" || !canManageItems.value) return;
  const template = templates.value.find((candidate) => candidate.id === templateId) ?? null;
  applyLcscLookupToDraft(draft.value, candidate, template);
  validationErrors.value = {};
  notice.info(t("items.lcscFilled"), { detail: candidate.product_code });
  try {
    const image = await prepareLcscItemImage(candidate.image_url, candidate.product_code);
    if (dialogMode.value !== "create" || draft.value.sku !== candidate.product_code) return;
    releaseImageDraft(draft.value.image ?? undefined);
    draft.value.image = createPendingImageDraft(image.file);
    draft.value.imageTemporary = true;
    if (image.usedPlaceholder) {
      notice.warning(t("items.lcscImageUnavailable"), { detail: t("items.placeholderUsed") });
    }
  } catch {
    notice.warning(t("items.lcscImageFillFailed"), { detail: t("items.placeholderFailed") });
  }
}

async function openExisting(
  item: ItemCatalogEntryResponse,
  pageName: "data" | "inventory",
): Promise<void> {
  await clearEditor();
  dialogMode.value = "existing";
  dialogInitialPage.value = pageName;
  selectedItemId.value = item.id;
  selectedCatalogItem.value = item;
  editorDataReady.value = false;
  editorDataLoading.value = false;
  editorDataError.value = "";
  editorOpen.value = true;
  if (pageName === "data") await loadSelectedEditor();
}

async function loadSelectedEditor(): Promise<void> {
  if (
    !selectedItemId.value ||
    (draft.value.id === selectedItemId.value && !editorDataLoading.value)
  )
    return;
  editorAbortController?.abort();
  const controller = new AbortController();
  editorAbortController = controller;
  editorDataLoading.value = true;
  editorDataError.value = "";
  try {
    const item = await getItem(selectedItemId.value, controller.signal);
    const template =
      templates.value.find((candidate) => candidate.id === item.attribute_template_id) ?? null;
    draft.value = draftFromItem(item, template);
    baselineDraft.value = draftFromItem(item, template);
    baselineFingerprint.value = itemDraftFingerprint(draft.value);
    editorDataReady.value = true;
  } catch (error) {
    if (!(error instanceof DOMException && error.name === "AbortError")) {
      editorDataError.value = errorMessage(error);
      notice.error(t("items.loadItemDataFailed"), { detail: editorDataError.value });
    }
  } finally {
    if (editorAbortController === controller) {
      editorAbortController = null;
      editorDataLoading.value = false;
    }
  }
}

function requestCloseEditor(): void {
  requestDraftTransition(clearEditor);
}
async function clearEditor(): Promise<void> {
  editorAbortController?.abort();
  editorAbortController = null;
  await discardCurrentTemporaryFiles();
  substituteDirty.value = false;
  editorOpen.value = false;
  selectedItemId.value = null;
  selectedCatalogItem.value = null;
  draft.value = emptyItemDraft();
  baselineDraft.value = emptyItemDraft();
  baselineFingerprint.value = itemDraftFingerprint(draft.value);
  validationErrors.value = {};
  editorDataLoading.value = false;
  editorDataReady.value = true;
  editorDataError.value = "";
}

function requestDraftTransition(action: () => Promise<void>): void {
  if (!hasUnsavedChanges.value) {
    void action();
    return;
  }
  pendingTransition = action;
  discardDialogOpen.value = true;
}
function cancelPendingTransition(): void {
  discardDialogOpen.value = false;
  pendingTransition = null;
  pendingLeaveResolution?.(false);
  pendingLeaveResolution = null;
}
async function confirmPendingTransition(): Promise<void> {
  const action = pendingTransition;
  const resolve = pendingLeaveResolution;
  discardDialogOpen.value = false;
  pendingTransition = null;
  pendingLeaveResolution = null;
  if (resolve) {
    await clearEditor();
    resolve(true);
  } else if (action) await action();
}

async function save(): Promise<void> {
  if (!canManageItems.value || substituteDirty.value) return;
  const validation = validateItemDraft(draft.value, templates.value);
  if (validation) {
    validationErrors.value = localizeValidationErrors(validation.errors);
    notice.warning(t("items.checkItemInfo"), {
      detail: localizeMessage(validation.firstMessage),
    });
    return;
  }
  if (!draft.value.image) return;
  saving.value = true;
  try {
    await uploadImageDrafts([
      draft.value.image,
      ...draft.value.attributes.map((attribute) => attribute.value).filter(isImageDraftValue),
    ]);
    const wasEditing = Boolean(draft.value.id);
    if (draft.value.id) {
      const request = itemUpdateRequest(draft.value, baselineDraft.value);
      if (Object.keys(request).length) await updateItem(draft.value.id, request);
    } else await createItem(itemCreateRequest(draft.value));
    if (draft.value.obsoleteImageFileId)
      await deleteImage(draft.value.obsoleteImageFileId).catch(() =>
        notice.warning(t("items.oldImageCleanupFailed"), {
          detail: t("items.autoCleanupNote"),
        }),
      );
    draft.value.attributes.forEach((attribute) => {
      attribute.fileTemporary = false;
    });
    draft.value.imageTemporary = false;
    draft.value.obsoleteImageFileId = null;
    notice.success(wasEditing ? t("items.itemUpdated") : t("items.itemCreated"));
    await clearEditor();
    await reloadCatalog();
  } catch (error) {
    if (error instanceof ApiError) {
      const apiValidation = itemDraftValidationFromApiError(error, draft.value);
      if (apiValidation) {
        validationErrors.value = localizeValidationErrors(apiValidation.errors);
        notice.warning(t("items.checkItemInfo"), {
          detail: localizeMessage(apiValidation.firstMessage),
        });
        return;
      }
    }
    const imageError = [
      draft.value.image,
      ...draft.value.attributes.map((attribute) => attribute.value),
    ].find((value) => isImageDraftValue(value) && value.status === "failed");
    notice.error(imageError ? t("items.imageUploadFailed") : t("items.saveItemFailed"), {
      detail: isImageDraftValue(imageError) ? imageError.error : errorMessage(error),
    });
  } finally {
    saving.value = false;
  }
}

function catalogAttributeText(attribute: CatalogAttributeResponse): string {
  const value =
    typeof attribute.value === "object"
      ? t("items.fieldTypeFile")
      : typeof attribute.value === "boolean"
        ? attribute.value
          ? t("items.yes")
          : t("items.no")
        : String(attribute.value);
  return attribute.unit ? `${value} ${attribute.unit}` : value;
}
function stockStateLabel(state: ItemStockState): string {
  return {
    out_of_stock: t("items.stockStateOutOfStock"),
    reorder_due: t("items.stockStateReorderDue"),
    needs_configuration: t("items.stockStateNeedsConfiguration"),
    normal: t("items.stockStateNormal"),
  }[state];
}
function formatQuantity(value: number): string {
  return new Intl.NumberFormat("zh-CN", { maximumFractionDigits: 3 }).format(value);
}
function formatMoney(value: number): string {
  return new Intl.NumberFormat("zh-CN", {
    style: "currency",
    currency: "CNY",
    maximumFractionDigits: 2,
  }).format(value);
}
function formatDateTime(value: string): string {
  return value.replace("T", " ").slice(0, 16);
}
function errorMessage(error: unknown): string {
  return error instanceof ApiError ? error.message : t("error.network_unavailable");
}
function localizeMessage(message: string): string {
  return translateMessageOrNull(message) ?? message;
}
function localizeValidationErrors(errors: Record<string, string>): Record<string, string> {
  return Object.fromEntries(
    Object.entries(errors).map(([field, message]) => [field, localizeMessage(message)]),
  );
}
async function discardCurrentTemporaryFiles(): Promise<void> {
  try {
    await discardTemporaryItemFiles(draft.value);
  } catch {
    notice.warning(t("items.tempImagesCleanupFailed"), { detail: t("items.autoCleanupNote") });
  }
}
function handleBeforeUnload(event: BeforeUnloadEvent): void {
  if (editorOpen.value && hasUnsavedChanges.value) {
    event.preventDefault();
    event.returnValue = "";
  }
}
function mergeItems(
  current: ItemCatalogEntryResponse[],
  next: ItemCatalogEntryResponse[],
): ItemCatalogEntryResponse[] {
  const map = new Map(current.map((item) => [item.id, item]));
  next.forEach((item) => map.set(item.id, item));
  return Array.from(map.values());
}
</script>
