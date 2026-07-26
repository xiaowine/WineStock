<!--
  本文件拥有可跨桌面与移动端使用的多明细入库工作台，属于 frontend 页面层。
  它管理本地草稿和入库单创建，不拥有后续审批或单据详情展示。
-->
<template>
  <section class="route-page inbound-draft-page">
    <header class="content-header inbound-draft-page__header">
      <div class="inbound-page-title">
        <div>
          <h1>{{ $route.meta.title }}</h1>
        </div>
      </div>
      <div
        v-if="draftItems.length > 0"
        class="content-summary inbound-draft-summary"
        aria-label="当前入库草稿摘要"
      >
        <span
          ><strong>{{ draftItems.length }}</strong> 条明细</span
        >
        <span aria-hidden="true">·</span>
        <span
          >入库数量 <strong>{{ quantitySummary }}</strong></span
        >
        <template v-if="draftAmountReady">
          <span aria-hidden="true">·</span>
          <span
            >预计金额 <strong>¥{{ formatMoney(draftTotal) }}</strong></span
          >
        </template>
      </div>
      <div class="inbound-page-actions">
        <button
          class="text-button inbound-clear-button"
          type="button"
          :disabled="!hasDraft || submitting"
          @click="openClearConfirmation"
        >
          清空草稿
        </button>
        <template v-if="draftItems.length > 0">
          <button
            class="primary-button"
            type="button"
            :disabled="submitting"
            @click="reviewDraft(defaultSubmissionMode)"
          >
            {{ submitting ? "正在提交…" : canDirectInbound ? "直接入库" : "提交审核" }}
          </button>
        </template>
      </div>
    </header>

    <div class="inbound-workspace">
      <InboundDraftStep
        :lines="draftItems"
        :locations="locations"
        :location-error="locationError"
        :source="source"
        :notes="notes"
        :notes-open="notesOpen"
        :validation-attempted="validationAttempted"
        :selected-line-id="selectedLineId"
        :dialog-open="selectedLine !== null"
        @update:source="source = $event"
        @update:notes="notes = $event"
        @update:notes-open="notesOpen = $event"
        @retry-locations="loadLocationOptions"
        @select-line="selectLine"
        @remove-line="removeLine"
        @add-item="openItemPicker"
      />
    </div>

    <ModalDialog
      :open="selectedLine !== null"
      title="入库物品明细"
      description="数量、单价和收货信息属于同一条明细；完成后才能继续添加下一项。"
      workspace
      @close="closeLineEditor"
      @after-close="handleLineEditorAfterClose"
    >
      <template v-if="selectedLine" #context>
        <div class="inbound-line-editor-context">
          <AuthenticatedImage
            :file-id="selectedLine.item.image_file_id"
            :alt="selectedLine.item.name + ' 主图'"
            :size="34"
            previewable
          />
          <div>
            <strong :title="selectedLine.item.name">{{ selectedLine.item.name }}</strong>
            <span>{{ selectedLine.item.sku }} · {{ selectedLine.item.unit }}</span>
          </div>
        </div>
      </template>
      <InboundLineEditor
        v-if="selectedLine"
        :line="selectedLine"
        :locations="locations"
        :location-error="locationError"
        :validation-attempted="validationAttempted"
        @retry-locations="loadLocationOptions"
      />
      <template #actions>
        <button
          class="secondary-button inbound-line-editor-action"
          type="button"
          @click="closeLineEditor"
        >
          暂存并关闭
        </button>
        <button
          class="primary-button inbound-line-editor-action"
          type="button"
          @click="completeLineAndContinue"
        >
          完成并继续添加
        </button>
      </template>
    </ModalDialog>

    <ItemSelectionDialog
      :open="itemPickerOpen"
      title="选择入库物品"
      description="选择一项后进入明细配置。"
      search-name="inbound_item_search"
      :items="items"
      :search-input="searchInput"
      :loading-items="loadingItems"
      :item-error="itemError"
      :items-exhausted="itemsExhausted"
      :selected-item-ids="draftItemIds"
      :can-create-item="canCreateItem"
      @close="closeItemPicker"
      @after-close="handleItemPickerAfterClose"
      @update:search-input="searchInput = $event"
      @search="applySearch"
      @reset-items="resetItems"
      @load-next-items="loadNextItems"
      @scroll-items="handleItemScroll"
      @list-element="itemList = $event"
      @select-item="handleItemSelected"
      @create-item="openCreateItemFromPicker"
    />

    <ItemCreateDialog
      :open="itemCreateOpen"
      @close="itemCreateOpen = false"
      @created="handleItemCreated"
    />

    <ModalDialog
      :open="confirmationMode !== null"
      :title="confirmationMode === 'clear' ? '清空入库草稿？' : '离开当前页面？'"
      :description="
        confirmationMode === 'clear'
          ? '所有未提交明细都会被删除。'
          : '当前草稿已自动保存在本机，离开后仍可恢复。'
      "
      @close="cancelConfirmation"
    >
      <p>{{ confirmationMode === "clear" ? "此操作无法撤销。" : "确认离开当前入库流程吗？" }}</p>
      <template #actions>
        <button class="secondary-button" type="button" @click="cancelConfirmation">取消</button>
        <button class="primary-button" type="button" @click="confirmCurrentAction">
          {{ confirmationMode === "clear" ? "确认清空" : "确认离开" }}
        </button>
      </template>
    </ModalDialog>

    <ModalDialog
      :open="submissionConfirmationMode !== null"
      :title="submissionConfirmationMode === 'direct' ? '确认直接入库？' : '确认提交审核？'"
      :description="
        submissionConfirmationMode === 'direct'
          ? '提交后将立即增加库存并写入库存流水。'
          : '提交后单据进入待审批状态，审批通过前不会增加库存。'
      "
      :busy="submitting"
      @close="cancelSubmissionConfirmation"
    >
      <dl class="inbound-submit-summary">
        <div>
          <dt>入库来源</dt>
          <dd>{{ source.trim() }}</dd>
        </div>
        <div>
          <dt>明细数量</dt>
          <dd>{{ draftItems.length }} 条</dd>
        </div>
        <div>
          <dt>入库总量</dt>
          <dd>{{ quantitySummary }}</dd>
        </div>
        <div>
          <dt>预计金额</dt>
          <dd>¥{{ formatMoney(draftTotal) }}</dd>
        </div>
      </dl>
      <p v-if="submissionConfirmationMode === 'direct'" class="inbound-submit-warning">
        请确认库位、数量和单价无误。直接入库完成后应通过后续库存业务进行调整。
      </p>
      <template #actions>
        <button
          class="secondary-button"
          type="button"
          :disabled="submitting"
          @click="cancelSubmissionConfirmation"
        >
          返回检查
        </button>
        <button
          class="primary-button"
          type="button"
          :disabled="submitting"
          @click="submitConfirmedDraft"
        >
          {{
            submitting
              ? "正在提交…"
              : submissionConfirmationMode === "direct"
                ? "确认并入库"
                : "确认提交"
          }}
        </button>
      </template>
    </ModalDialog>
  </section>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { onBeforeRouteLeave } from "vue-router";
import ModalDialog from "../components/ModalDialog.vue";
import AuthenticatedImage from "../components/attributes/AuthenticatedImage.vue";
import InboundDraftStep from "../components/inbound/InboundDraftStep.vue";
import InboundLineEditor from "../components/inbound/InboundLineEditor.vue";
import ItemCreateDialog from "../components/items/ItemCreateDialog.vue";
import ItemSelectionDialog from "../components/items/ItemSelectionDialog.vue";
import {
  createInbound,
  listLocations,
  type InboundSubmissionMode,
  type LocationResponse,
} from "../api/inbound";
import type { ItemOptionResponse } from "../api/items";
import { ApiError } from "../api/errors";
import { useInboundDraftPersistence } from "../composables/useInboundDraftPersistence";
import { useInboundItemCatalog } from "../composables/useInboundItemCatalog";
import { notice } from "../notices/notice";
import { authSession } from "../auth/session";
import { hasPermission, stockPermissions } from "../auth/permissions";
import {
  buildInboundRequest,
  createDraftLine,
  lineReady,
  lineSubtotal,
  positiveNumber,
  validQuantity,
  validUnitPrice,
  type InboundDraftLine,
} from "./inbound-draft/model";
import {
  formatMoney,
  formatQuantity,
  inboundSubmitErrorMessage,
  isAbortError,
  itemErrorMessage,
} from "./inbound-draft/presentation";

type ConfirmationMode = "clear" | "leave" | null;
const restoredNoticeSessionKey = "winestock.inbound.restored-notice";
const draftItems = ref<InboundDraftLine[]>([]);
const locations = ref<LocationResponse[]>([]);
const locationError = ref("");
const source = ref("");
const notes = ref("");
const notesOpen = ref(false);
const selectedLineId = ref<string | null>(null);
const itemPickerOpen = ref(false);
const submitting = ref(false);
const validationAttempted = ref(false);
const confirmationMode = ref<ConfirmationMode>(null);
const submissionConfirmationMode = ref<InboundSubmissionMode | null>(null);
const itemCreateOpen = ref(false);
let locationAbortController: AbortController | null = null;
let pendingLeaveResolution: ((allowed: boolean) => void) | null = null;
let pendingPickerItem: ItemOptionResponse | null = null;
let openCreateItemAfterPicker = false;
let openPickerAfterLineEditor = false;

const {
  items,
  searchInput,
  loadingItems,
  itemError,
  itemList,
  itemsExhausted,
  resetItems,
  loadNextItems,
  applySearch,
  handleItemScroll,
} = useInboundItemCatalog((error) => itemErrorMessage(error));

const draftItemCounts = computed(() => {
  const counts = new Map<number, number>();
  for (const line of draftItems.value)
    counts.set(line.item.id, (counts.get(line.item.id) ?? 0) + 1);
  return counts;
});
const draftItemIds = computed<ReadonlySet<number>>(() => new Set(draftItemCounts.value.keys()));
const selectedLine = computed(
  () => draftItems.value.find((line) => line.lineId === selectedLineId.value) ?? null,
);
const draftQuantity = computed(() =>
  draftItems.value.reduce((total, line) => total + positiveNumber(line.quantity), 0),
);
const draftTotal = computed(() =>
  draftItems.value.reduce((total, line) => total + lineSubtotal(line), 0),
);
const quantitySummary = computed(() => {
  const units = new Set(draftItems.value.map((line) => line.item.unit).filter(Boolean));
  if (units.size === 1) return `${formatQuantity(draftQuantity.value)} ${Array.from(units)[0]}`;
  return draftItems.value.length ? "按明细分别计量" : "0";
});
const draftAmountReady = computed(
  () => draftItems.value.length > 0 && draftItems.value.every(lineReady),
);
const hasDraft = computed(
  () =>
    source.value.trim().length > 0 || notes.value.trim().length > 0 || draftItems.value.length > 0,
);
const draftReady = computed(
  () =>
    source.value.trim().length > 0 &&
    draftItems.value.length > 0 &&
    draftItems.value.every(lineReady),
);
const incompleteLine = computed(() => draftItems.value.find((line) => !lineReady(line)) ?? null);
const canDirectInbound = computed(() =>
  hasPermission(authSession.value?.user.permissions, stockPermissions.inboundApprove),
);
const canCreateItem = computed(() =>
  hasPermission(authSession.value?.user.permissions, stockPermissions.itemManage),
);
const defaultSubmissionMode = computed<InboundSubmissionMode>(() =>
  canDirectInbound.value ? "direct" : "pending_approval",
);
const { restoreDraft, resumeDraftSaving, removePersistedDraft } = useInboundDraftPersistence(
  source,
  notes,
  notesOpen,
  draftItems,
  hasDraft,
);

onMounted(() => {
  const restored = restoreDraft();
  const removedDuplicates = removeRestoredDuplicateItems();
  // 恢复历史草稿只恢复数据，不主动进入任一明细的详情编辑模式。
  selectedLineId.value = null;
  if (restored && sessionStorage.getItem(restoredNoticeSessionKey) !== "shown") {
    sessionStorage.setItem(restoredNoticeSessionKey, "shown");
    notice.info("已恢复上次未提交的入库草稿");
  }
  if (removedDuplicates > 0) notice.info(`已移除 ${removedDuplicates} 条重复物品明细`);
  resumeDraftSaving();
  void resetItems();
  void loadLocationOptions();
  window.addEventListener("keydown", handlePageKeydown);
});

/** 兼容旧版草稿数据，恢复时按物品 ID 保留第一条明细。 */
function removeRestoredDuplicateItems(): number {
  const seen = new Set<number>();
  const unique: InboundDraftLine[] = [];
  let duplicates = 0;
  for (const line of draftItems.value) {
    if (seen.has(line.item.id)) duplicates += 1;
    else {
      seen.add(line.item.id);
      unique.push(line);
    }
  }
  if (!duplicates) return 0;
  draftItems.value = unique;
  return duplicates;
}

onBeforeUnmount(() => {
  locationAbortController?.abort();
  window.removeEventListener("keydown", handlePageKeydown);
});

onBeforeRouteLeave(() => {
  if (itemCreateOpen.value) return true;
  if (!hasDraft.value) return true;
  confirmationMode.value = "leave";
  return new Promise<boolean>((resolve) => {
    pendingLeaveResolution = resolve;
  });
});

async function loadLocationOptions(): Promise<void> {
  locationAbortController?.abort();
  const controller = new AbortController();
  locationAbortController = controller;
  locationError.value = "";
  try {
    locations.value = await listLocations({}, controller.signal);
  } catch (error) {
    if (!isAbortError(error)) locationError.value = itemErrorMessage(error, "加载库位失败");
  } finally {
    if (locationAbortController === controller) locationAbortController = null;
  }
}

/** 选择阶段按物品去重，每个物品在当前入库单中只保留一条独立明细。 */
function addItem(item: ItemOptionResponse, showNotice = true): InboundDraftLine | null {
  const existing = draftItems.value.find((line) => line.item.id === item.id);
  if (existing) {
    selectedLineId.value = existing.lineId;
    return existing;
  }
  const line = createDraftLine(item);
  draftItems.value.push(line);
  // 物品加入草稿后立即打开该明细，避免先批量选品再重复配置。
  selectedLineId.value = line.lineId;
  if (showNotice) notice.info(`已加入 ${item.name}`);
  return line;
}

function openItemPicker(): void {
  if (incompleteLine.value) {
    selectedLineId.value = incompleteLine.value.lineId;
    notice.warning("请先完成当前入库明细", {
      detail: `已重新打开“${incompleteLine.value.item.name}”的配置界面。`,
    });
    return;
  }
  itemPickerOpen.value = true;
  void resetItems();
}

function closeItemPicker(): void {
  pendingPickerItem = null;
  openCreateItemAfterPicker = false;
  itemPickerOpen.value = false;
}

function handleItemSelected(item: ItemOptionResponse): void {
  pendingPickerItem = item;
  itemPickerOpen.value = false;
}

function openCreateItemFromPicker(): void {
  pendingPickerItem = null;
  openCreateItemAfterPicker = true;
  itemPickerOpen.value = false;
}

function handleItemPickerAfterClose(): void {
  if (openCreateItemAfterPicker) {
    openCreateItemAfterPicker = false;
    itemCreateOpen.value = true;
    return;
  }
  const item = pendingPickerItem;
  pendingPickerItem = null;
  if (item) addItem(item);
}

async function handleItemCreated(item: ItemOptionResponse): Promise<void> {
  itemCreateOpen.value = false;
  addItem(item, false);
  notice.success("物品已创建并加入入库单", { detail: item.name });
}

function removeLine(lineId: string): void {
  const line = draftItems.value.find((candidate) => candidate.lineId === lineId);
  if (!line) return;
  draftItems.value = draftItems.value.filter((candidate) => candidate.lineId !== lineId);
  if (selectedLineId.value === lineId) selectedLineId.value = null;
  notice.info(`已移除 ${line.item.name}`);
}

async function completeLineAndContinue(): Promise<void> {
  const line = selectedLine.value;
  if (!line) return;
  validationAttempted.value = true;
  if (!lineReady(line)) {
    notice.warning("当前明细尚未完成", { detail: `请先补齐“${line.item.name}”的必填信息。` });
    await focusLineError(line);
    return;
  }
  openPickerAfterLineEditor = true;
  selectedLineId.value = null;
}

function selectLine(lineId: string): void {
  selectedLineId.value = lineId;
}

function closeLineEditor(): void {
  openPickerAfterLineEditor = false;
  selectedLineId.value = null;
}

function handleLineEditorAfterClose(): void {
  if (!openPickerAfterLineEditor) return;
  openPickerAfterLineEditor = false;
  itemPickerOpen.value = true;
  void resetItems();
}

async function reviewDraft(submissionMode: InboundSubmissionMode): Promise<void> {
  validationAttempted.value = true;
  if (!draftReady.value) {
    notice.warning("入库单信息尚未填写完整", { detail: draftBlockingReason() });
    await focusFirstError();
    return;
  }
  submissionConfirmationMode.value = submissionMode;
}

async function submitConfirmedDraft(): Promise<void> {
  const submissionMode = submissionConfirmationMode.value;
  if (!submissionMode || submitting.value) return;
  submitting.value = true;
  try {
    const created = await createInbound(
      buildInboundRequest(source.value, notes.value, draftItems.value, submissionMode),
    );
    if (created.submission_mode === "direct") {
      notice.success("入库成功", { detail: `单号 #${created.id} 已完成入库，库存已更新。` });
    } else {
      notice.success("入库单已提交", { detail: `单号 #${created.id} 已进入待审批状态。` });
    }
    submissionConfirmationMode.value = null;
    clearLocalDraftState();
  } catch (error) {
    const message = inboundSubmitErrorMessage(error);
    const errorLine = backendErrorLine(error);
    if (error instanceof ApiError && error.code === "item_not_found" && errorLine) {
      message.title = `“${errorLine.item.name}”已失效，请从入库单移除`;
    }
    notice.error(message.title, { detail: message.detail });
    submissionConfirmationMode.value = null;
    await nextTick();
    await focusBackendError(error);
  } finally {
    submitting.value = false;
  }
}

function cancelSubmissionConfirmation(): void {
  if (!submitting.value) submissionConfirmationMode.value = null;
}

function handlePageKeydown(event: KeyboardEvent): void {
  if (
    event.key !== "Escape" ||
    submissionConfirmationMode.value !== null ||
    confirmationMode.value !== null
  )
    return;
  if (selectedLineId.value) closeLineEditor();
}

function openClearConfirmation(): void {
  if (hasDraft.value) confirmationMode.value = "clear";
}

function cancelConfirmation(): void {
  if (confirmationMode.value === "leave") pendingLeaveResolution?.(false);
  pendingLeaveResolution = null;
  confirmationMode.value = null;
}

function confirmCurrentAction(): void {
  if (confirmationMode.value === "leave") {
    const resolve = pendingLeaveResolution;
    pendingLeaveResolution = null;
    confirmationMode.value = null;
    resolve?.(true);
    return;
  }
  if (confirmationMode.value !== "clear") return;
  clearLocalDraftState();
  confirmationMode.value = null;
}

function clearLocalDraftState(): void {
  source.value = "";
  notes.value = "";
  notesOpen.value = false;
  draftItems.value = [];
  selectedLineId.value = null;
  validationAttempted.value = false;
  removePersistedDraft();
}

async function focusFirstError(): Promise<void> {
  if (draftItems.value.length === 0) {
    openItemPicker();
    await nextTick();
    document.querySelector<HTMLElement>('[name="inbound_item_search"]')?.focus();
    return;
  }
  selectedLineId.value = null;
  await nextTick();
  if (!source.value.trim()) {
    document.querySelector<HTMLElement>("[data-inbound-source]")?.focus();
    return;
  }
  for (const line of draftItems.value) {
    if (!lineReady(line)) return focusLineError(line);
  }
}

async function focusLineError(line: InboundDraftLine): Promise<void> {
  if (!validQuantity(line.quantity)) return focusLineControl(line, "quantity");
  if (!validUnitPrice(line.unitPrice)) return focusLineControl(line, "unitPrice");
  if (line.locationId === null) return focusLineControl(line, "locationId");
}

async function focusLineControl(line: InboundDraftLine, field: string): Promise<void> {
  selectedLineId.value = line.lineId;
  await nextTick();
  document
    .querySelector<HTMLElement>(`[data-line-id="${line.lineId}"][data-field="${field}"]`)
    ?.focus();
}

async function focusBackendError(error: unknown): Promise<void> {
  if (!(error instanceof ApiError) || !isRecord(error.details)) return;
  const line = backendErrorLine(error);
  if (!line) return;
  validationAttempted.value = true;
  if (error.code === "item_not_found") await focusLineControl(line, "remove");
  else if (error.code === "location_not_found") await focusLineControl(line, "locationId");
  else {
    selectedLineId.value = line.lineId;
    await nextTick();
  }
}

function backendErrorLine(error: unknown): InboundDraftLine | null {
  if (!(error instanceof ApiError) || !isRecord(error.details)) return null;
  const lineIndex = typeof error.details.line_index === "number" ? error.details.line_index : -1;
  return draftItems.value[lineIndex] ?? null;
}

function draftBlockingReason(): string {
  if (!source.value.trim()) return "请填写入库来源。";
  if (!draftItems.value.length) return "请至少添加一条入库明细。";
  const invalid = draftItems.value.find((line) => !lineReady(line));
  return invalid ? `请检查“${invalid.item.name}”的数量、单价和库位。` : "请检查入库单信息。";
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
</script>

<style lang="scss" src="./InboundDraftPage.scss"></style>
