<!--
  本组件拥有单个物品的替代关系查看、编辑和保存会话，属于 frontend 物品工作区层。
  它通过 HTTP 使用替代关系 API，不负责全局替代关系列表，也不绕过后端循环校验。
-->
<template>
  <section class="item-substitutes-panel" aria-labelledby="item-substitutes-title">
    <header
      v-if="showHeading || (canManage && showSaveAction)"
      class="item-substitutes-panel__header"
    >
      <div v-if="showHeading">
        <h3 id="item-substitutes-title">{{ $t('items.maintainSubstitutes') }}</h3>
        <p>
          {{
            canManage ? $t('items.substitutesManageHint') : $t('items.substitutesViewHint')
          }}
        </p>
      </div>
      <button
        v-if="canManage && showSaveAction"
        class="primary-button item-substitutes-panel__save"
        type="button"
        :disabled="saving || loading || !dirty"
        @click="requestSave"
      >
        {{ saving ? $t('common.saving') : $t('items.saveSubstitutes') }}
      </button>
    </header>

    <div
      v-if="loadError && !loaded"
      class="item-substitutes-panel__state item-substitutes-panel__state--error"
      role="alert"
    >
      <span>{{ loadError }}</span>
      <button class="secondary-button" type="button" @click="loadSubstitutes">
        {{ $t('items.retry') }}
      </button>
    </div>
    <div v-else-if="loading && !loaded" class="item-substitutes-panel__state" role="status">
      <span v-if="showLoading">{{ $t('items.loadingSubstitutes') }}</span>
    </div>
    <template v-else>
      <section
        v-if="canManage && canSearchCandidates"
        class="item-substitutes-panel__add"
        aria-labelledby="item-substitutes-add-title"
      >
        <h4 id="item-substitutes-add-title">{{ $t('items.addSubstitute') }}</h4>
        <SearchField
          v-model="searchInput"
          :label="$t('items.searchSubstitutes')"
          name="substitute_item_search"
          :placeholder="$t('items.nameOrSku')"
          @search="applySearch"
        />
        <div v-if="candidateError" class="item-substitutes-panel__inline-error" role="alert">
          <span>{{ candidateError }}</span>
          <button class="text-button" type="button" @click="loadCandidates(1)">
            {{ $t('items.retry') }}
          </button>
        </div>
        <div
          v-else-if="candidateLoading && !candidates.length"
          class="item-substitutes-panel__candidate-state"
          role="status"
        >
          {{ $t('items.searchingItems') }}
        </div>
        <div
          v-else-if="activeSearch && !visibleCandidates.length"
          class="item-substitutes-panel__candidate-state"
        >
          {{ $t('items.noAddableItems') }}
        </div>
        <div
          v-else-if="visibleCandidates.length"
          v-overlay-scrollbar
          class="item-substitutes-panel__candidates"
          :aria-label="$t('items.addableSubstitutes')"
        >
          <button
            v-for="candidate in visibleCandidates"
            :key="candidate.id"
            class="item-substitutes-panel__candidate"
            type="button"
            :aria-label="$t('items.addSubstituteNamed', { name: candidate.name })"
            :title="$t('items.addSubstitute')"
            @click="addSubstitute(candidate)"
          >
            <span
              ><strong>{{ candidate.name }}</strong
              ><small>{{ candidate.sku }} · {{ candidate.unit }}</small></span
            >
            <span class="item-substitutes-panel__candidate-action" aria-hidden="true">
              <svg viewBox="0 0 24 24" focusable="false">
                <path d="M12 5v14M5 12h14" />
              </svg>
            </span>
          </button>
        </div>
      </section>

      <section class="item-substitutes-panel__list" aria-labelledby="item-substitutes-list-title">
        <header class="item-substitutes-panel__list-header">
          <h4 id="item-substitutes-list-title">{{ $t('items.configuredSubstitutes') }}</h4>
          <span>{{ $t('items.itemCount', { n: drafts.length }) }}</span>
        </header>
        <div v-if="!drafts.length" class="item-substitutes-panel__empty">
          {{ $t('items.noSubstitutes') }}
        </div>
        <div v-else class="item-substitutes-panel__relations">
          <article
            v-for="(draft, index) in drafts"
            :key="draft.substituteItemId"
            class="item-substitutes-panel__relation"
          >
            <div class="item-substitutes-panel__relation-identity">
              <span class="item-substitutes-panel__priority">{{ draft.priority }}</span>
              <AuthenticatedImage
                :file-id="draft.imageFileId"
                :alt="$t('items.mainImageName', { name: draft.name })"
                :size="64"
                previewable
              />
              <div class="item-substitutes-panel__relation-identity-content">
                <strong
                  class="item-substitutes-panel__relation-identity-name"
                  :title="draft.name"
                  >{{ draft.name }}</strong
                >
                <dl class="item-substitutes-panel__relation-identity-meta">
                  <div>
                    <dt>{{ $t('items.sku') }}</dt>
                    <dd :title="draft.sku">{{ draft.sku }}</dd>
                  </div>
                  <div>
                    <dt>{{ $t('items.category') }}</dt>
                    <dd :title="draft.categoryName ?? $t('items.uncategorized')">
                      {{ draft.categoryName ?? $t('items.uncategorized') }}
                    </dd>
                  </div>
                </dl>
              </div>
            </div>
            <dl class="item-substitutes-panel__relation-extra">
              <div>
                <dt>{{ $t('items.unitField') }}</dt>
                <dd>{{ draft.unit }}</dd>
              </div>
              <div>
                <dt>{{ $t('items.currentStock') }}</dt>
                <dd>
                  {{
                    draft.stockState === null
                      ? $t('items.pendingSaveLoad')
                      : formatQuantity(draft.quantity)
                  }}
                </dd>
              </div>
              <div>
                <dt>{{ $t('items.stockStateLabel') }}</dt>
                <dd v-if="draft.stockState === null">{{ $t('items.pendingLoad') }}</dd>
                <dd v-else :class="`stock-state stock-state--${draft.stockState}`">
                  {{ stockStateLabel(draft.stockState) }}
                </dd>
              </div>
              <div>
                <dt>{{ $t('items.reorderPoint') }}</dt>
                <dd>
                  {{
                    draft.stockState === null
                      ? $t('items.pendingLoad')
                      : draft.reorderPoint === null
                        ? $t('items.notSet')
                        : formatQuantity(draft.reorderPoint)
                  }}
                </dd>
              </div>
            </dl>
            <label class="item-substitutes-panel__notes">
              <span
                ><span>{{ $t('items.remark') }}</span
                ><small>{{ $t('items.remainingChars', { n: 1024 - draft.notes.length }) }}</small></span
              >
              <textarea
                v-model="draft.notes"
                :name="`substitute_notes_${draft.substituteItemId}`"
                :disabled="!canManage || saving"
                rows="2"
                maxlength="1024"
                :placeholder="$t('items.notesPlaceholder')"
              />
            </label>
            <div v-if="canManage" class="item-substitutes-panel__relation-actions">
              <button
                class="icon-button"
                type="button"
                :title="$t('items.moveUp')"
                :aria-label="$t('items.moveUpNamed', { name: draft.name })"
                :disabled="index === 0 || saving"
                @click="moveSubstitute(index, -1)"
              >
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m5 14 7-7 7 7" /></svg>
              </button>
              <button
                class="icon-button"
                type="button"
                :title="$t('items.moveDown')"
                :aria-label="$t('items.moveDownNamed', { name: draft.name })"
                :disabled="index === drafts.length - 1 || saving"
                @click="moveSubstitute(index, 1)"
              >
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m5 10 7 7 7-7" /></svg>
              </button>
              <button
                class="icon-button item-substitutes-panel__remove"
                type="button"
                :title="$t('items.removeSubstituteRelation')"
                :aria-label="$t('items.removeSubstituteNamed', { name: draft.name })"
                :disabled="saving"
                @click="removeSubstitute(index)"
              >
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  <path d="M4 7h16M9 7V4h6v3M7 7l1 13h8l1-13M10 11v5M14 11v5" />
                </svg>
              </button>
            </div>
          </article>
        </div>
      </section>

      <p v-if="saveError && showSaveAction" class="item-substitutes-panel__save-error" role="alert">
        {{ saveError }}
      </p>
    </template>
  </section>

  <ModalDialog
    :open="clearConfirmOpen"
    :title="$t('items.clearAllSubstitutesTitle')"
    :description="$t('items.clearAllSubstitutesDescription')"
    :busy="saving"
    nested
    compact
    @close="clearConfirmOpen = false"
  >
    <p class="confirmation-copy">{{ $t('items.clearAllSubstitutesConfirm') }}</p>
    <template #actions>
      <button
        class="secondary-button"
        type="button"
        :disabled="saving"
        @click="clearConfirmOpen = false"
      >
        {{ $t('items.continueEditing') }}
      </button>
      <button class="danger-button" type="button" :disabled="saving" @click="confirmClearAll">
        {{ saving ? $t('common.saving') : $t('items.confirmClearAndSave') }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { listItemOptions, type ItemOptionResponse, type ItemStockState } from "../../api/items";
import { ApiError } from "../../api/errors";
import {
  listItemSubstitutes,
  replaceItemSubstitutes,
  type ItemSubstituteResponse,
} from "../../api/substitutes";
import SearchField from "../SearchField.vue";
import AuthenticatedImage from "../attributes/AuthenticatedImage.vue";
import { notice } from "../../notices/notice";
import { useStablePendingIndicator } from "../../composables/useStablePendingIndicator";
import ModalDialog from "../ModalDialog.vue";
import "./ItemSubstitutesPanel.scss";

interface SubstituteDraft {
  substituteItemId: number;
  name: string;
  sku: string;
  categoryName: string | null;
  imageFileId: number;
  unit: string;
  quantity: number;
  reorderPoint: number | null;
  stockState: ItemStockState | null;
  priority: number;
  notes: string;
}

const props = withDefaults(
  defineProps<{
    itemId: number;
    canManage: boolean;
    canSearchCandidates?: boolean;
    showHeading?: boolean;
    showSaveAction?: boolean;
  }>(),
  {
    canSearchCandidates: true,
    showHeading: true,
    showSaveAction: true,
  },
);

const emit = defineEmits<{
  /** 通知工作区父级当前替代关系草稿是否有未保存修改。 */
  "dirty-change": [dirty: boolean];
  /** 通知 Dialog 当前保存请求状态，避免请求期间关闭会话。 */
  "saving-change": [saving: boolean];
  /** 让外层 Dialog 可将保存错误固定展示在 modal-actions 上方。 */
  "save-error-change": [message: string];
  /** 通知全局页面当前主物品的替代关系已保存。 */
  saved: [];
}>();
const { t } = useI18n();

const drafts = ref<SubstituteDraft[]>([]);
const baseline = ref("[]");
const loaded = ref(false);
const loading = ref(false);
const loadError = ref("");
const saving = ref(false);
const saveError = ref("");
const searchInput = ref("");
const activeSearch = ref("");
const candidates = ref<ItemOptionResponse[]>([]);
const candidateLoading = ref(false);
const candidateError = ref("");
const clearConfirmOpen = ref(false);
let substituteController: AbortController | null = null;
let candidateController: AbortController | null = null;

const dirty = computed(() => fingerprint(drafts.value) !== baseline.value);
const canSearchCandidates = computed(() => props.canSearchCandidates);
const visibleCandidates = computed(() =>
  candidates.value.filter(
    (candidate) => candidate.id !== props.itemId && !isSelected(candidate.id),
  ),
);
const showLoading = useStablePendingIndicator(loading, { showDelayMs: 200, minimumVisibleMs: 350 });

onMounted(() => {
  void loadSubstitutes();
});
watch(
  () => props.itemId,
  () => {
    void loadSubstitutes();
  },
);
watch(dirty, (value) => emit("dirty-change", value), { immediate: true });
watch(saveError, (value) => emit("save-error-change", value), { immediate: true });
onBeforeUnmount(() => {
  substituteController?.abort();
  candidateController?.abort();
});

async function loadSubstitutes(): Promise<void> {
  substituteController?.abort();
  const controller = new AbortController();
  substituteController = controller;
  loading.value = true;
  loaded.value = false;
  loadError.value = "";
  saveError.value = "";
  clearConfirmOpen.value = false;
  drafts.value = [];
  try {
    const response = await listItemSubstitutes(props.itemId, controller.signal);
    drafts.value = response
      .slice()
      .sort((left, right) => left.priority - right.priority)
      .map(toDraft);
    normalizePriorities();
    baseline.value = fingerprint(drafts.value);
    loaded.value = true;
  } catch (error) {
    if (!isAbortError(error)) loadError.value = errorMessage(error);
  } finally {
    if (substituteController === controller) {
      substituteController = null;
      loading.value = false;
    }
  }
}

function applySearch(value: string): void {
  activeSearch.value = value.trim();
  void loadCandidates(1);
}

async function loadCandidates(page: number): Promise<void> {
  if (!props.canManage || !canSearchCandidates.value || !activeSearch.value) {
    candidates.value = [];
    return;
  }
  candidateController?.abort();
  const controller = new AbortController();
  candidateController = controller;
  candidateLoading.value = true;
  candidateError.value = "";
  try {
    const response = await listItemOptions(activeSearch.value, page, 30, controller.signal);
    candidates.value =
      page === 1 ? response.items : mergeCandidates(candidates.value, response.items);
  } catch (error) {
    if (!isAbortError(error)) candidateError.value = errorMessage(error);
  } finally {
    if (candidateController === controller) {
      candidateController = null;
      candidateLoading.value = false;
    }
  }
}

function addSubstitute(candidate: ItemOptionResponse): void {
  if (candidate.id === props.itemId || isSelected(candidate.id)) return;
  drafts.value.push({
    substituteItemId: candidate.id,
    name: candidate.name,
    sku: candidate.sku,
    categoryName: candidate.category_name,
    imageFileId: candidate.image_file_id,
    unit: candidate.unit,
    quantity: 0,
    reorderPoint: null,
    stockState: null,
    priority: drafts.value.length + 1,
    notes: "",
  });
  normalizePriorities();
}

function moveSubstitute(index: number, direction: -1 | 1): void {
  const target = index + direction;
  if (target < 0 || target >= drafts.value.length) return;
  const [draft] = drafts.value.splice(index, 1);
  drafts.value.splice(target, 0, draft);
  normalizePriorities();
}

function removeSubstitute(index: number): void {
  drafts.value.splice(index, 1);
  normalizePriorities();
}

async function saveSubstitutes(): Promise<void> {
  if (!props.canManage || !dirty.value || saving.value) return;
  saving.value = true;
  emit("saving-change", true);
  saveError.value = "";
  try {
    const response = await replaceItemSubstitutes(props.itemId, {
      substitutes: drafts.value.map((draft) => ({
        substitute_item_id: draft.substituteItemId,
        priority: draft.priority,
        notes: draft.notes.trim() || null,
      })),
    });
    drafts.value = response
      .slice()
      .sort((left, right) => left.priority - right.priority)
      .map(toDraft);
    normalizePriorities();
    baseline.value = fingerprint(drafts.value);
    notice.success(t("items.substitutesSaved"));
    emit("saved");
  } catch (error) {
    saveError.value = errorMessage(error);
    notice.error(t("items.saveSubstitutesFailed"), { detail: saveError.value });
  } finally {
    saving.value = false;
    emit("saving-change", false);
  }
}

function requestSave(): void {
  if (drafts.value.length === 0 && baseline.value !== "[]") {
    clearConfirmOpen.value = true;
    return;
  }
  void saveSubstitutes();
}

defineExpose({ requestSave });

function confirmClearAll(): void {
  clearConfirmOpen.value = false;
  void saveSubstitutes();
}

function isSelected(itemId: number): boolean {
  return drafts.value.some((draft) => draft.substituteItemId === itemId);
}

function normalizePriorities(): void {
  drafts.value.forEach((draft, index) => {
    draft.priority = index + 1;
  });
}

function toDraft(response: ItemSubstituteResponse): SubstituteDraft {
  return {
    substituteItemId: response.substitute_item_id,
    name: response.substitute_item_name,
    sku: response.substitute_item_sku,
    categoryName: response.substitute_item_category_name,
    imageFileId: response.substitute_item_image_file_id,
    unit: response.substitute_item_unit,
    quantity: response.quantity,
    reorderPoint: response.substitute_item_reorder_point,
    stockState: response.substitute_item_stock_state,
    priority: response.priority,
    notes: response.notes ?? "",
  };
}

function fingerprint(values: SubstituteDraft[]): string {
  return JSON.stringify(
    values.map(({ substituteItemId, priority, notes }) => ({
      substituteItemId,
      priority,
      notes: notes.trim(),
    })),
  );
}

function mergeCandidates(
  current: ItemOptionResponse[],
  next: ItemOptionResponse[],
): ItemOptionResponse[] {
  const map = new Map(current.map((item) => [item.id, item]));
  next.forEach((item) => map.set(item.id, item));
  return Array.from(map.values());
}

function formatQuantity(value: number): string {
  return new Intl.NumberFormat("zh-CN", { maximumFractionDigits: 3 }).format(value);
}
function stockStateLabel(state: ItemStockState): string {
  return {
    out_of_stock: t("items.stockStateOutOfStock"),
    reorder_due: t("items.stockStateReorderDue"),
    needs_configuration: t("items.stockStateNeedsConfiguration"),
    normal: t("items.stockStateNormal"),
  }[state];
}
function isAbortError(error: unknown): boolean {
  return error instanceof DOMException && error.name === "AbortError";
}
function errorMessage(error: unknown): string {
  return error instanceof ApiError ? error.message : t("error.network_unavailable");
}
</script>
