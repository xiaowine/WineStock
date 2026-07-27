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
        <h3 id="item-substitutes-title">维护替代物品</h3>
        <p>
          {{
            canManage ? "维护缺货时可替代的物品和优先级。" : "查看该物品缺货时可使用的替代物品。"
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
        {{ saving ? "保存中…" : "保存替代关系" }}
      </button>
    </header>

    <div
      v-if="loadError && !loaded"
      class="item-substitutes-panel__state item-substitutes-panel__state--error"
      role="alert"
    >
      <span>{{ loadError }}</span>
      <button class="secondary-button" type="button" @click="loadSubstitutes">重试</button>
    </div>
    <div v-else-if="loading && !loaded" class="item-substitutes-panel__state" role="status">
      <span v-if="showLoading">正在加载替代关系…</span>
    </div>
    <template v-else>
      <section
        v-if="canManage && canSearchCandidates"
        class="item-substitutes-panel__add"
        aria-labelledby="item-substitutes-add-title"
      >
        <h4 id="item-substitutes-add-title">添加替代物品</h4>
        <SearchField
          v-model="searchInput"
          label="搜索替代物品"
          name="substitute_item_search"
          placeholder="名称或编号"
          @search="applySearch"
        />
        <div v-if="candidateError" class="item-substitutes-panel__inline-error" role="alert">
          <span>{{ candidateError }}</span>
          <button class="text-button" type="button" @click="loadCandidates(1)">重试</button>
        </div>
        <div
          v-else-if="candidateLoading && !candidates.length"
          class="item-substitutes-panel__candidate-state"
          role="status"
        >
          正在搜索物品…
        </div>
        <div
          v-else-if="activeSearch && !visibleCandidates.length"
          class="item-substitutes-panel__candidate-state"
        >
          没有找到可添加的物品。
        </div>
        <div
          v-else-if="visibleCandidates.length"
          class="item-substitutes-panel__candidates"
          aria-label="可添加的替代物品"
        >
          <button
            v-for="candidate in visibleCandidates"
            :key="candidate.id"
            class="item-substitutes-panel__candidate"
            type="button"
            :aria-label="`添加替代物品：${candidate.name}`"
            title="添加替代物品"
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
          <h4 id="item-substitutes-list-title">已配置替代物品</h4>
          <span>{{ drafts.length }} 项</span>
        </header>
        <div v-if="!drafts.length" class="item-substitutes-panel__empty">暂无替代物品</div>
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
                :alt="`${draft.name} 主图`"
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
                    <dt>编号</dt>
                    <dd :title="draft.sku">{{ draft.sku }}</dd>
                  </div>
                  <div>
                    <dt>分类</dt>
                    <dd :title="draft.categoryName ?? '未分类'">
                      {{ draft.categoryName ?? "未分类" }}
                    </dd>
                  </div>
                </dl>
              </div>
            </div>
            <dl class="item-substitutes-panel__relation-extra">
              <div>
                <dt>单位</dt>
                <dd>{{ draft.unit }}</dd>
              </div>
              <div>
                <dt>当前库存</dt>
                <dd>
                  {{ draft.stockState === null ? "待保存后加载" : formatQuantity(draft.quantity) }}
                </dd>
              </div>
              <div>
                <dt>库存状态</dt>
                <dd v-if="draft.stockState === null">待加载</dd>
                <dd v-else :class="`stock-state stock-state--${draft.stockState}`">
                  {{ stockStateLabel(draft.stockState) }}
                </dd>
              </div>
              <div>
                <dt>再订货点</dt>
                <dd>
                  {{
                    draft.stockState === null
                      ? "待加载"
                      : draft.reorderPoint === null
                        ? "未设置"
                        : formatQuantity(draft.reorderPoint)
                  }}
                </dd>
              </div>
            </dl>
            <label class="item-substitutes-panel__notes">
              <span
                ><span>备注</span><small>剩余 {{ 1024 - draft.notes.length }} 字</small></span
              >
              <textarea
                v-model="draft.notes"
                :name="`substitute_notes_${draft.substituteItemId}`"
                :disabled="!canManage || saving"
                rows="2"
                maxlength="1024"
                placeholder="可填写兼容性说明"
              />
            </label>
            <div v-if="canManage" class="item-substitutes-panel__relation-actions">
              <button
                class="icon-button"
                type="button"
                title="上移优先级"
                :aria-label="`将 ${draft.name} 上移`"
                :disabled="index === 0 || saving"
                @click="moveSubstitute(index, -1)"
              >
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m5 14 7-7 7 7" /></svg>
              </button>
              <button
                class="icon-button"
                type="button"
                title="下移优先级"
                :aria-label="`将 ${draft.name} 下移`"
                :disabled="index === drafts.length - 1 || saving"
                @click="moveSubstitute(index, 1)"
              >
                <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m5 10 7 7 7-7" /></svg>
              </button>
              <button
                class="icon-button item-substitutes-panel__remove"
                type="button"
                title="移除替代关系"
                :aria-label="`移除 ${draft.name}`"
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
    title="清空全部替代关系？"
    description="保存后当前主物品将不再拥有任何替代物品。"
    :busy="saving"
    nested
    compact
    @close="clearConfirmOpen = false"
  >
    <p class="confirmation-copy">此操作会整体替换服务端关系，之后仍可重新添加替代物品。</p>
    <template #actions>
      <button
        class="secondary-button"
        type="button"
        :disabled="saving"
        @click="clearConfirmOpen = false"
      >
        继续编辑
      </button>
      <button class="danger-button" type="button" :disabled="saving" @click="confirmClearAll">
        {{ saving ? "保存中…" : "确认清空并保存" }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
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
    notice.success("替代关系已保存");
    emit("saved");
  } catch (error) {
    saveError.value = errorMessage(error);
    notice.error("保存替代关系失败", { detail: saveError.value });
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
    out_of_stock: "缺货",
    reorder_due: "待补货",
    needs_configuration: "需配置",
    normal: "库存正常",
  }[state];
}
function isAbortError(error: unknown): boolean {
  return error instanceof DOMException && error.name === "AbortError";
}
function errorMessage(error: unknown): string {
  return error instanceof ApiError ? error.message : "无法连接到 WineStock 服务";
}
</script>
