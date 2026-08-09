<!--
  本文件拥有分类与模板页面的两业务域状态、权限入口和 CRUD 请求编排。
  它不编辑具体物品或入库记录，也不虚构服务端未提供的引用数量。
-->
<template>
  <section class="route-page templates-page">
    <header class="content-header templates-page__header">
      <div>
        <h1>{{ $title($route.meta.title) }}</h1>
        <p>{{ $t('templates.subtitle') }}</p>
      </div>
    </header>

    <section class="templates-workspace">
      <div
        v-overlay-scrollbar
        class="templates-tabs"
        role="tablist"
        :aria-label="$t('templates.domainTabsAria')"
        @keydown="handleTabKeydown"
      >
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
          {{ t(domain.label) }}
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
            <div class="templates-toolbar" :class="{ 'templates-toolbar--readonly': !canManage }">
              <SearchField
                :model-value="searchInputs[activeDomain]"
                class="templates-toolbar__search"
                :label="searchLabel"
                :name="`template_search_${activeDomain}`"
                :placeholder="searchPlaceholder"
                :disabled="currentState.loading && !currentState.loaded"
                @update:model-value="searchInputs[activeDomain] = $event"
                @search="applySearch"
              />
              <div class="templates-toolbar__commands">
                <div class="templates-toolbar__summary">
                  <span>{{ filteredCount }} {{ domainCountLabel }}</span>
                  <span v-if="showRefreshing" role="status">{{ $t('templates.refreshing') }}</span>
                </div>
                <div class="templates-toolbar__actions">
                  <button
                    class="icon-button templates-toolbar__refresh"
                    :class="{ 'templates-toolbar__refresh--pending': showRefreshing }"
                    type="button"
                    :title="t('templates.refreshTitle', { domain: activeDomainLabel })"
                    :aria-label="t('templates.refreshTitle', { domain: activeDomainLabel })"
                    :aria-busy="currentState.loading"
                    :disabled="currentState.loading"
                    @click="refreshCurrent"
                  >
                    <svg viewBox="0 0 24 24" aria-hidden="true">
                      <path d="M20 7v5h-5" />
                      <path d="M18.2 16a7 7 0 1 1 .8-7l1 3" />
                    </svg>
                  </button>
                  <button
                    v-if="canManage"
                    class="icon-button icon-button--primary templates-toolbar__create"
                    type="button"
                    :title="t('templates.createTitle', { domain: activeDomainLabel })"
                    :aria-label="t('templates.createTitle', { domain: activeDomainLabel })"
                    @click="openCreate"
                  >
                    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
                  </button>
                </div>
              </div>
            </div>

            <div class="templates-list-stage" :aria-busy="currentState.loading">
              <div
                v-if="currentState.error && !currentState.loaded"
                class="templates-state templates-state--error"
                role="alert"
              >
                <strong>{{ currentState.error }}</strong>
                <button class="secondary-button" type="button" @click="loadDomain(activeDomain)">
                  {{ $t('common.retry') }}
                </button>
              </div>
              <div
                v-else-if="showInitialLoading && !currentState.loaded"
                class="templates-state"
                role="status"
              >
                {{ $t('templates.loadingDomain', { domain: activeDomainLabel }) }}
              </div>
              <div v-else-if="filteredCount === 0" class="templates-state">
                <strong>{{
                  currentSearch
                    ? $t('templates.noSearchResults')
                    : $t('templates.noDomainRecords', { domain: activeDomainLabel })
                }}</strong>
                <span>{{
                  currentSearch
                    ? $t('templates.clearSearchToSeeAll')
                    : canManage
                      ? $t('templates.createFromToolbar', { domain: activeDomainLabel })
                      : $t('templates.noDataAvailable')
                }}</span>
                <button v-if="currentSearch" class="text-button" type="button" @click="clearSearch">
                  {{ $t('templates.clearSearch') }}
                </button>
              </div>
              <template v-else>
                <p
                  v-if="currentState.error"
                  class="templates-list-stage__inline-error"
                  role="alert"
                >
                  {{ currentState.error }}
                </p>

                <div
                  v-if="activeDomain === 'category'"
                  class="templates-table templates-table--category"
                  role="table"
                  :aria-label="$t('templates.categoryListAria')"
                >
                  <div class="templates-table__head" role="row">
                    <span>{{ $t('templates.categoryHeaderName') }}</span
                    ><span>{{ $t('templates.categoryHeaderDescription') }}</span
                    ><span>{{ $t('templates.categoryHeaderOrderActions') }}</span>
                  </div>
                  <article
                    v-for="category in filteredCategories"
                    :key="category.id"
                    class="templates-table__row"
                    role="row"
                  >
                    <div class="templates-table__identity" role="cell">
                      <strong>{{ category.name }}</strong
                      ><span>{{ $t('templates.categoryIdLabel', { id: category.id }) }}</span>
                    </div>
                    <div
                      class="templates-table__description"
                      role="cell"
                      :title="category.description ?? undefined"
                    >
                      {{ category.description || $t('templates.noDescription') }}
                    </div>
                    <div class="templates-table__decision" role="cell">
                      <span class="templates-table__meta">
                        <span
                          :class="{
                            'templates-table__usage--empty': category.item_usage_count === 0,
                          }"
                          >{{ usageLabel(category.item_usage_count) }}</span
                        ><span>{{ $t('templates.sortOrder') }}
                          <strong>{{ category.sort_order }}</strong></span
                        ><span>{{ $t('templates.updated') }}
                          <time
                            :datetime="category.updated_at"
                            :title="formatFullDateTime(category.updated_at)"
                            >{{ formatTime(category.updated_at) }}</time
                          ></span
                        >
                      </span>
                      <span v-if="canManage" class="templates-table__actions">
                        <button
                          class="icon-button"
                          type="button"
                          :title="$t('templates.editCategory')"
                          :aria-label="$t('templates.editCategoryAria', { name: category.name })"
                          @click="openCategory(category)"
                        >
                          <EditIcon />
                        </button>
                        <button
                          class="icon-button templates-table__delete"
                          type="button"
                          :title="$t('templates.deleteCategory')"
                          :aria-label="$t('templates.deleteCategoryAria', { name: category.name })"
                          @click="openDelete('category', category)"
                        >
                          <DeleteIcon />
                        </button>
                      </span>
                    </div>
                  </article>
                </div>

                <div
                  v-else-if="activeDomain === 'item'"
                  class="templates-table templates-table--template"
                  role="table"
                  :aria-label="$t('templates.templateListAria')"
                >
                  <div class="templates-table__head" role="row">
                    <span>{{ $t('templates.templateHeaderName') }}</span
                    ><span>{{ $t('templates.templateHeaderFields') }}</span
                    ><span>{{ $t('templates.templateHeaderUpdatedActions') }}</span>
                  </div>
                  <article
                    v-for="template in filteredItemTemplates"
                    :key="template.id"
                    class="templates-table__row"
                    role="row"
                  >
                    <button
                      class="templates-table__identity templates-table__identity--button"
                      type="button"
                      role="cell"
                      @click="openTemplate(template, true)"
                    >
                      <strong
                        >{{ template.name
                        }}<em v-if="template.is_default" class="templates-table__default-badge"
                          >{{ $t('templates.defaultBadge') }}</em
                        ></strong
                      ><span>{{ $t('templates.templateIdLabel', { id: template.id }) }}</span
                      ><small>{{ template.description || $t('templates.noDescription') }}</small>
                    </button>
                    <div class="templates-table__information" role="cell">
                      <span class="templates-table__metrics"
                        ><span
                          >{{ $t('templates.fieldCountLabel') }}
                          <strong>{{ template.fields.length }}</strong></span
                        ><span
                          >{{ $t('templates.requiredCountLabel') }}
                          <strong>{{ countRequired(template.fields) }}</strong></span
                        ><span
                          >{{ $t('templates.searchableCountLabel') }}
                          <strong>{{ countSearchable(template.fields) }}</strong></span
                        ><span
                          >{{ $t('templates.catalogCountLabel') }}
                          <strong>{{ countCatalogVisible(template) }}/3</strong></span
                        ><span
                          :class="{
                            'templates-table__usage--empty': template.item_usage_count === 0,
                          }"
                          >{{ usageLabel(template.item_usage_count) }}</span
                        ></span
                      >
                    </div>
                    <div class="templates-table__decision" role="cell">
                      <time
                        :datetime="template.updated_at"
                        :title="formatFullDateTime(template.updated_at)"
                        >{{ formatTime(template.updated_at) }}</time
                      >
                      <span class="templates-table__actions">
                        <button
                          class="icon-button"
                          type="button"
                          :title="$t('templates.viewTemplate')"
                          :aria-label="$t('templates.viewTemplateAria', { name: template.name })"
                          @click="openTemplate(template, true)"
                        >
                          <ViewIcon />
                        </button>
                        <template v-if="canManage">
                          <button
                            class="icon-button"
                            :class="{ 'templates-table__default-active': template.is_default }"
                            type="button"
                            :title="
                              template.is_default
                                ? $t('templates.unsetDefaultTemplate')
                                : $t('templates.setDefaultTemplate')
                            "
                            :aria-label="
                              template.is_default
                                ? $t('templates.unsetDefaultTemplateAria', { name: template.name })
                                : $t('templates.setDefaultTemplateAria', { name: template.name })
                            "
                            :aria-pressed="template.is_default"
                            :disabled="defaultUpdatingId !== null"
                            @click="toggleDefaultTemplate(template)"
                          >
                            <StarIcon />
                          </button>
                          <button
                            class="icon-button"
                            type="button"
                            :title="$t('templates.editTemplate')"
                            :aria-label="$t('templates.editTemplateAria', { name: template.name })"
                            @click="openTemplate(template, false)"
                          >
                            <EditIcon />
                          </button>
                          <button
                            class="icon-button"
                            type="button"
                            :title="$t('templates.copyTemplate')"
                            :aria-label="$t('templates.copyTemplateAria', { name: template.name })"
                            @click="openCopy(template)"
                          >
                            <CopyIcon />
                          </button>
                          <button
                            class="icon-button templates-table__delete"
                            type="button"
                            :title="$t('templates.deleteTemplate')"
                            :aria-label="$t('templates.deleteTemplateAria', { name: template.name })"
                            @click="openDelete('item', template)"
                          >
                            <DeleteIcon />
                          </button>
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
      :template="editorState?.template ?? null"
      :read-only="editorState?.readOnly ?? false"
      :can-edit="canManage"
      :submitting="actionSubmitting"
      :error-message="actionError"
      :field-errors="actionFieldErrors"
      @close="closeActions"
      @edit="enableEditor"
      @submit="saveTemplate"
    />
    <TemplateCopyDialog
      :target="copyTarget"
      :submitting="actionSubmitting"
      :error-message="actionError"
      :field-error="actionFieldErrors.name ?? ''"
      @close="closeActions"
      @submit="copyTemplate"
    />
    <TemplateDeleteDialog
      :target="deleteTarget"
      :submitting="actionSubmitting"
      :error-message="actionError"
      @close="closeActions"
      @submit="deleteTargetRecord"
    />
  </section>
</template>

<script setup lang="ts">
import {
  computed,
  defineComponent,
  h,
  onBeforeUnmount,
  onMounted,
  reactive,
  ref,
  watch,
} from "vue";
import { useI18n } from "vue-i18n";
import { ApiConfigurationError, ApiError, ApiNetworkError, ApiResponseError } from "../api/errors";
import {
  createItemCategory,
  deleteItemCategory,
  listItemCategories,
  updateItemCategory,
  type ItemCategoryResponse,
  type ItemCategoryWriteRequest,
} from "../api/itemCategories";
import {
  copyItemAttributeTemplate,
  createItemAttributeTemplate,
  deleteItemAttributeTemplate,
  listItemAttributeTemplates,
  updateItemAttributeTemplate,
  type ItemAttributeTemplateResponse,
} from "../api/itemAttributeTemplates";
import type { TemplateFieldResponse } from "../api/templateFields";
import { hasPermission, stockPermissions } from "../auth/permissions";
import { authSession } from "../auth/session";
import { translateMessageOrNull } from "../i18n";
import type { MessageKeyPath } from "../i18n/schema";
import CategoryDialog from "../components/templates/CategoryDialog.vue";
import TemplateCopyDialog, {
  type TemplateCopyTarget,
} from "../components/templates/TemplateCopyDialog.vue";
import TemplateDeleteDialog, {
  type TemplateDeleteTarget,
} from "../components/templates/TemplateDeleteDialog.vue";
import TemplateEditorDialog from "../components/templates/TemplateEditorDialog.vue";
import SearchField from "../components/SearchField.vue";
import { useStablePendingIndicator } from "../composables/useStablePendingIndicator";
import { notice } from "../notices/notice";
import {
  buildItemTemplateRequest,
  type TemplateDomain,
  type TemplateDraft,
} from "./templates/model";
import "./TemplatesPage.scss";

interface DomainState {
  loaded: boolean;
  loading: boolean;
  error: string;
}

interface EditorState {
  template: ItemAttributeTemplateResponse | null;
  readOnly: boolean;
}

const { t } = useI18n();

const domains: { value: TemplateDomain; label: MessageKeyPath }[] = [
  { value: "category", label: "templates.domainCategory" },
  { value: "item", label: "templates.domainItemTemplate" },
];

const activeDomain = ref<TemplateDomain>("category");
const domainTransitionDirection = ref<"next" | "previous">("next");
const categories = ref<ItemCategoryResponse[]>([]);
const itemTemplates = ref<ItemAttributeTemplateResponse[]>([]);
const states = reactive<Record<TemplateDomain, DomainState>>({
  category: { loaded: false, loading: false, error: "" },
  item: { loaded: false, loading: false, error: "" },
});
const searchInputs = reactive<Record<TemplateDomain, string>>({ category: "", item: "" });
const searches = reactive<Record<TemplateDomain, string>>({ category: "", item: "" });
const categoryDialogOpen = ref(false);
const editingCategory = ref<ItemCategoryResponse | null>(null);
const editorState = ref<EditorState | null>(null);
const copyTarget = ref<TemplateCopyTarget | null>(null);
const deleteTarget = ref<TemplateDeleteTarget | null>(null);
const actionSubmitting = ref(false);
const actionError = ref("");
const actionFieldErrors = ref<Record<string, string>>({});
const controllers = new Map<TemplateDomain, AbortController>();

const currentPermissions = computed(() => authSession.value?.user.permissions);
const canManage = computed(() =>
  hasPermission(currentPermissions.value, stockPermissions.templateManage),
);
const currentState = computed(() => states[activeDomain.value]);
const currentSearch = computed(() => searches[activeDomain.value]);
const activeDomainLabel = computed(() => domainLabel(activeDomain.value));
const domainTransitionName = computed(() => `templates-domain-${domainTransitionDirection.value}`);
const domainCountLabel = computed(() =>
  activeDomain.value === "category"
    ? t("templates.categoryCountUnit")
    : t("templates.itemTemplateCountUnit"),
);
const searchLabel = computed(() => t("templates.searchDomain", { domain: activeDomainLabel.value }));
const searchPlaceholder = computed(() =>
  activeDomain.value === "category"
    ? t("templates.searchCategoryPlaceholder")
    : t("templates.searchTemplatePlaceholder"),
);
const refreshPending = computed(() => currentState.value.loaded && currentState.value.loading);
const initialLoading = computed(() => !currentState.value.loaded && currentState.value.loading);
const showRefreshing = useStablePendingIndicator(refreshPending, {
  showDelayMs: 200,
  minimumVisibleMs: 350,
});
const showInitialLoading = useStablePendingIndicator(initialLoading, {
  showDelayMs: 200,
  minimumVisibleMs: 350,
});
const filteredCategories = computed(() =>
  filterRecords(categories.value, searches.category).sort(
    (left, right) =>
      left.sort_order - right.sort_order ||
      left.name.localeCompare(right.name, "zh-CN") ||
      left.id - right.id,
  ),
);
const filteredItemTemplates = computed(() => filterRecords(itemTemplates.value, searches.item));
const filteredCount = computed(() =>
  activeDomain.value === "category"
    ? filteredCategories.value.length
    : filteredItemTemplates.value.length,
);
const defaultCategorySortOrder = computed(
  () => Math.max(-1, ...categories.value.map((item) => item.sort_order)) + 1,
);

onMounted(() => {
  void loadDomain("category");
});
onBeforeUnmount(() => controllers.forEach((controller) => controller.abort()));

watch(canManage, (allowed) => {
  if (allowed) return;
  categoryDialogOpen.value = false;
  copyTarget.value = null;
  deleteTarget.value = null;
  if (editorState.value) editorState.value = { ...editorState.value, readOnly: true };
});

function domainLabel(domain: TemplateDomain): string {
  const key = domains.find((item) => item.value === domain)?.label;
  return key ? t(key) : "";
}

async function loadDomain(domain: TemplateDomain, announce = false): Promise<boolean> {
  controllers.get(domain)?.abort();
  const controller = new AbortController();
  controllers.set(domain, controller);
  states[domain].loading = true;
  states[domain].error = "";
  try {
    if (domain === "category") categories.value = await listItemCategories(controller.signal);
    else itemTemplates.value = await listItemAttributeTemplates(controller.signal);
    states[domain].loaded = true;
    if (announce)
      notice.success(t("templates.refreshedDomain", { domain: domainLabel(domain) }));
    return true;
  } catch (error) {
    if (isAbortError(error)) return false;
    states[domain].error = errorMessage(
      error,
      t("templates.loadDomainFailed", { domain: domainLabel(domain) }),
    );
    notice.error(states[domain].error);
    return false;
  } finally {
    if (controllers.get(domain) === controller) {
      controllers.delete(domain);
      states[domain].loading = false;
    }
  }
}

function selectDomain(domain: TemplateDomain): void {
  if (activeDomain.value === domain) return;
  const currentIndex = domains.findIndex((item) => item.value === activeDomain.value);
  const nextIndex = domains.findIndex((item) => item.value === domain);
  domainTransitionDirection.value = nextIndex > currentIndex ? "next" : "previous";
  activeDomain.value = domain;
  if (!states[domain].loaded && !states[domain].loading) void loadDomain(domain);
}

function handleTabKeydown(event: KeyboardEvent): void {
  if (!["ArrowLeft", "ArrowRight"].includes(event.key)) return;
  event.preventDefault();
  const index = domains.findIndex((domain) => domain.value === activeDomain.value);
  const next = (index + (event.key === "ArrowRight" ? 1 : -1) + domains.length) % domains.length;
  selectDomain(domains[next].value);
  requestAnimationFrame(() =>
    document.getElementById(`templates-tab-${domains[next].value}`)?.focus(),
  );
}

function refreshCurrent(): void {
  void loadDomain(activeDomain.value, true);
}
function applySearch(value: string): void {
  searches[activeDomain.value] = value.trim();
}
function clearSearch(): void {
  searchInputs[activeDomain.value] = "";
  searches[activeDomain.value] = "";
}

function openCreate(): void {
  resetActionState();
  if (activeDomain.value === "category") {
    editingCategory.value = null;
    categoryDialogOpen.value = true;
  } else editorState.value = { template: null, readOnly: false };
}

function openCategory(category: ItemCategoryResponse): void {
  resetActionState();
  editingCategory.value = category;
  categoryDialogOpen.value = true;
}

function openTemplate(template: ItemAttributeTemplateResponse, readOnly: boolean): void {
  resetActionState();
  editorState.value = { template, readOnly };
}

function enableEditor(): void {
  if (editorState.value && canManage.value)
    editorState.value = { ...editorState.value, readOnly: false };
}

function openCopy(template: ItemAttributeTemplateResponse): void {
  resetActionState();
  copyTarget.value = { id: template.id, name: template.name };
}

function openDelete(
  kind: TemplateDomain,
  record: { id: number; name: string; item_usage_count?: number },
): void {
  resetActionState();
  deleteTarget.value = {
    id: record.id,
    name: record.name,
    kind,
    itemUsageCount: record.item_usage_count ?? null,
  };
}

function closeActions(): void {
  if (actionSubmitting.value) return;
  categoryDialogOpen.value = false;
  editingCategory.value = null;
  editorState.value = null;
  copyTarget.value = null;
  deleteTarget.value = null;
  resetActionState();
}

async function saveCategory(request: ItemCategoryWriteRequest): Promise<void> {
  actionSubmitting.value = true;
  resetActionErrors();
  try {
    const updated = editingCategory.value
      ? await updateItemCategory(editingCategory.value.id, request)
      : await createItemCategory(request);
    categories.value = upsert(categories.value, updated);
    notice.success(
      editingCategory.value ? t("templates.categoryUpdated") : t("templates.categoryCreated"),
    );
    closeActionsAfterSuccess();
  } catch (error) {
    handleActionError(error, t("templates.saveCategoryFailed"));
  } finally {
    actionSubmitting.value = false;
  }
}

async function saveTemplate(draft: TemplateDraft): Promise<void> {
  if (!editorState.value) return;
  actionSubmitting.value = true;
  resetActionErrors();
  const { template } = editorState.value;
  try {
    const request = buildItemTemplateRequest(draft);
    const updated = template
      ? await updateItemAttributeTemplate(template.id, request)
      : await createItemAttributeTemplate(request);
    itemTemplates.value = upsert(itemTemplates.value, updated);
    notice.success(template ? t("templates.templateUpdated") : t("templates.templateCreated"));
    closeActionsAfterSuccess();
  } catch (error) {
    handleActionError(error, t("templates.saveTemplateFailed"));
  } finally {
    actionSubmitting.value = false;
  }
}

async function copyTemplate(name: string): Promise<void> {
  const target = copyTarget.value;
  if (!target) return;
  actionSubmitting.value = true;
  resetActionErrors();
  try {
    const copied = await copyItemAttributeTemplate(target.id, { name });
    itemTemplates.value = upsert(itemTemplates.value, copied);
    editorState.value = { template: copied, readOnly: false };
    copyTarget.value = null;
    notice.success(t("templates.templateCopied"));
  } catch (error) {
    handleActionError(error, t("templates.copyTemplateFailed"));
  } finally {
    actionSubmitting.value = false;
  }
}

async function deleteTargetRecord(): Promise<void> {
  const target = deleteTarget.value;
  if (!target) return;
  actionSubmitting.value = true;
  resetActionErrors();
  try {
    if (target.kind === "category") {
      const result = await deleteItemCategory(target.id);
      categories.value = categories.value.filter((item) => item.id !== target.id);
      notice.success(
        result.affected_active_item_count > 0
          ? t("templates.categoryDeletedAffected", { n: result.affected_active_item_count })
          : t("templates.categoryDeletedClean"),
      );
    } else {
      const result = await deleteItemAttributeTemplate(target.id);
      itemTemplates.value = itemTemplates.value.filter((item) => item.id !== target.id);
      notice.success(
        result.affected_active_item_count > 0
          ? t("templates.templateDeletedAffected", { n: result.affected_active_item_count })
          : t("templates.templateDeletedClean"),
      );
    }
    closeActionsAfterSuccess();
  } catch (error) {
    handleActionError(error, t("templates.deleteFailed"));
  } finally {
    actionSubmitting.value = false;
  }
}

function countRequired(fields: readonly TemplateFieldResponse[]): number {
  return fields.filter((field) => field.required).length;
}
function countSearchable(fields: readonly TemplateFieldResponse[]): number {
  return fields.filter((field) => field.searchable).length;
}
function countCatalogVisible(template: ItemAttributeTemplateResponse): number {
  return template.fields.filter((field) => field.catalog_visible).length;
}
function usageLabel(count: number): string {
  return count > 0 ? t("templates.usedByItems", { n: count }) : t("templates.unusedByItems");
}

function filterRecords<T extends { name: string; description: string | null }>(
  records: readonly T[],
  search: string,
): T[] {
  const normalized = search.trim().toLocaleLowerCase();
  if (!normalized) return [...records];
  return records.filter((item) =>
    `${item.name}\n${item.description ?? ""}`.toLocaleLowerCase().includes(normalized),
  );
}

function upsert<T extends { id: number }>(records: readonly T[], record: T): T[] {
  const existing = records.findIndex((item) => item.id === record.id);
  if (existing < 0) return [record, ...records];
  const next = [...records];
  next.splice(existing, 1, record);
  return next;
}

function formatTime(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime())
    ? "--:--"
    : new Intl.DateTimeFormat("zh-CN", {
        hour: "2-digit",
        minute: "2-digit",
        hour12: false,
      }).format(date);
}

function formatFullDateTime(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime())
    ? value
    : new Intl.DateTimeFormat("zh-CN", { dateStyle: "medium", timeStyle: "short" }).format(date);
}

function resetActionState(): void {
  actionError.value = "";
  actionFieldErrors.value = {};
}
function resetActionErrors(): void {
  actionError.value = "";
  actionFieldErrors.value = {};
}
function closeActionsAfterSuccess(): void {
  categoryDialogOpen.value = false;
  editingCategory.value = null;
  editorState.value = null;
  copyTarget.value = null;
  deleteTarget.value = null;
  resetActionState();
}

function handleActionError(error: unknown, fallback: string): void {
  actionError.value = errorMessage(error, fallback);
  actionFieldErrors.value = apiFieldErrors(error);
  if (
    error instanceof ApiError &&
    ["category_name_taken", "template_name_taken"].includes(error.code)
  ) {
    actionFieldErrors.value.name =
      translateMessageOrNull(`error.${error.code}`) ?? error.message;
  }
  notice.error(actionError.value, { detail: Object.values(actionFieldErrors.value)[0] });
  if (error instanceof ApiError && error.status === 404) void loadDomain(activeDomain.value);
}

function apiFieldErrors(error: unknown): Record<string, string> {
  if (!(error instanceof ApiError)) return {};
  return Object.fromEntries(
    Object.entries(error.fieldErrors).map(([path, messages]) => [
      normalizeFieldPath(path),
      messages[0] ?? t("templates.fieldInvalid"),
    ]),
  );
}

function normalizeFieldPath(path: string): string {
  return path
    .replace(/\[(\d+)\]/g, ".$1")
    .replace(/^body\./, "")
    .replace(/\.unit\.value$/, ".unit_value")
    .replace(/\.unit\.options(?=\.|$)/, ".unit_options");
}

const defaultUpdatingId = ref<number | null>(null);

/** 切换全站默认模板；服务端事务保证唯一，本地据结果同步清掉其它默认标记。 */
async function toggleDefaultTemplate(template: ItemAttributeTemplateResponse): Promise<void> {
  if (defaultUpdatingId.value !== null) return;
  defaultUpdatingId.value = template.id;
  try {
    const updated = await updateItemAttributeTemplate(template.id, {
      is_default: !template.is_default,
    });
    itemTemplates.value = itemTemplates.value.map((entry) => {
      if (entry.id === updated.id) return updated;
      return updated.is_default && entry.is_default ? { ...entry, is_default: false } : entry;
    });
    notice.success(
      updated.is_default
        ? t("templates.defaultTemplateSet", { name: updated.name })
        : t("templates.defaultTemplateUnset"),
      { detail: updated.is_default ? t("templates.defaultTemplateSetDetail") : undefined },
    );
  } catch (error) {
    notice.error(t("templates.defaultTemplateUpdateFailed"), {
      detail: errorMessage(error, t("templates.tryLater")),
    });
  } finally {
    defaultUpdatingId.value = null;
  }
}

function errorMessage(error: unknown, fallback: string): string {
  if (error instanceof ApiError) {
    if (error.code === "category_name_taken" || error.code === "template_name_taken") {
      return translateMessageOrNull(`error.${error.code}`) ?? error.message;
    }
    if (error.status === 403) return t("templates.actionForbidden");
    if (error.status === 404) return t("templates.recordDeletedRefresh");
    return error.message || fallback;
  }
  if (error instanceof ApiNetworkError) return error.message || fallback;
  if (error instanceof ApiConfigurationError || error instanceof ApiResponseError)
    return error.message;
  return fallback;
}

function isAbortError(error: unknown): boolean {
  return error instanceof DOMException && error.name === "AbortError";
}

const icon = (paths: string[]) =>
  defineComponent({
    setup: () => () =>
      h(
        "svg",
        { viewBox: "0 0 24 24", "aria-hidden": "true" },
        paths.map((path) => h("path", { d: path })),
      ),
  });
const ViewIcon = icon([
  "M2.5 12s3.5-6 9.5-6 9.5 6 9.5 6-3.5 6-9.5 6S2.5 12 2.5 12Z",
  "M12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6Z",
]);
const EditIcon = icon(["m5 17-1 3 3-1L19 7l-2-2L5 17Z", "m15 7 2 2"]);
const StarIcon = icon([
  "m12 3.5 2.5 5.4 5.9.6-4.4 4 1.3 5.8-5.3-3.1-5.3 3.1 1.3-5.8-4.4-4 5.9-.6z",
]);
const CopyIcon = icon(["M8 8h11v11H8z", "M5 16H4V5h11v1"]);
const DeleteIcon = icon(["M4 7h16M9 7V4h6v3M7 7l1 13h8l1-13M10 11v5M14 11v5"]);
</script>
