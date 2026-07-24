<!--
  本文件拥有可跨桌面与移动端使用的多明细入库工作台，属于 frontend 页面层。
  它管理本地草稿、动态模板、临时图片和入库单创建，不拥有后续审批或单据详情展示。
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
          <span>预计金额 <strong>¥{{ formatMoney(draftTotal) }}</strong></span>
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
        :template-options-loading="templateOptionsLoading"
        :template-options-error="templateOptionsError"
        :source="source"
        :notes="notes"
        :notes-open="notesOpen"
        :validation-attempted="validationAttempted"
        :selected-line-id="selectedLineId"
        :drawer-open="selectedLine !== null"
        :can-add-item="canAddItem"
        :add-item-disabled-reason="addItemDisabledReason"
        @update:source="source = $event"
        @update:notes="notes = $event"
        @update:notes-open="notesOpen = $event"
        @retry-locations="loadLocationOptions"
        @retry-templates="loadInboundTemplateOptions"
        @select-line="selectLine"
        @remove-line="removeLine"
        @add-item="openItemPicker"
      />
    </div>

    <Teleport to="body">
      <Transition name="inbound-editor" @after-leave="handleLineEditorAfterLeave">
        <div v-if="selectedLine" class="inbound-line-editor-layer">
          <div
            class="inbound-line-editor-backdrop"
            role="presentation"
            @click="closeLineEditor"
          ></div>
          <InboundLineEditor
            :line="selectedLine"
            :locations="locations"
            :location-error="locationError"
            :templates="inboundTemplates"
            :templates-loading="templateOptionsLoading"
            :templates-error="templateOptionsError"
            :validation-attempted="validationAttempted"
            @close="closeLineEditor"
            @complete-and-continue="completeLineAndContinue"
            @select-template="selectInboundTemplate"
            @retry-template="retryLineTemplate"
            @retry-templates="loadInboundTemplateOptions"
            @retry-locations="loadLocationOptions"
          />
        </div>
      </Transition>
    </Teleport>

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

    <ModalDialog
      :open="pendingTemplateChange !== null"
      title="更换入库模板？"
      description="当前已填写的模板属性和已选择图片会被清除。"
      :busy="templateChangeSubmitting"
      compact
      nested
      @close="cancelTemplateChange"
    >
      <p>确认后将清空当前明细的模板属性，再应用新的入库模板。</p>
      <template #actions>
        <button
          class="secondary-button"
          type="button"
          :disabled="templateChangeSubmitting"
          @click="cancelTemplateChange"
        >
          继续编辑
        </button>
        <button
          class="danger-button"
          type="button"
          :disabled="templateChangeSubmitting"
          @click="confirmTemplateChange"
        >
          {{ templateChangeSubmitting ? "正在更换…" : "清空并更换" }}
        </button>
      </template>
    </ModalDialog>

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
          ? '所有未提交明细和未绑定图片都会被删除。'
          : '当前草稿已自动保存在本机，离开后仍可恢复。'
      "
      :busy="clearingDraft"
      @close="cancelConfirmation"
    >
      <p>{{ confirmationMode === "clear" ? "此操作无法撤销。" : "确认离开当前入库流程吗？" }}</p>
      <template #actions>
        <button
          class="secondary-button"
          type="button"
          :disabled="clearingDraft"
          @click="cancelConfirmation"
        >
          取消
        </button>
        <button
          class="primary-button"
          type="button"
          :disabled="clearingDraft"
          @click="confirmCurrentAction"
        >
          {{ clearingDraft ? "处理中…" : confirmationMode === "clear" ? "确认清空" : "确认离开" }}
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
import {
  getInboundTemplate,
  listInboundTemplates,
  type InboundTemplateResponse,
} from "../api/inboundTemplates";
import type { ItemOptionResponse } from "../api/items";
import { deleteImage } from "../api/files";
import { ApiError } from "../api/errors";
import { useInboundDraftPersistence } from "../composables/useInboundDraftPersistence";
import { useInboundItemCatalog } from "../composables/useInboundItemCatalog";
import { useNativeBackHandler } from "../composables/useNativeBackHandler";
import { NativeBackPriority } from "../navigation/nativeBack";
import { notice } from "../notices/notice";
import { authSession } from "../auth/session";
import { hasPermission, stockPermissions } from "../auth/permissions";
import { isImageDraftValue, uploadImageDrafts } from "../components/attributes/imageDraft";
import {
  buildInboundRequest,
  createDraftLine,
  hasTemplateDraftValues,
  lineReady,
  lineSubtotal,
  positiveNumber,
  revokeLinePreviews,
  templateFieldError,
  validQuantity,
  validUnitPrice,
  type FileDraftValue,
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
const inboundTemplates = ref<InboundTemplateResponse[]>([]);
const locationError = ref("");
const templateOptionsLoading = ref(false);
const templateOptionsError = ref("");
const source = ref("");
const notes = ref("");
const notesOpen = ref(false);
const selectedLineId = ref<string | null>(null);
const itemPickerOpen = ref(false);
const submitting = ref(false);
const validationAttempted = ref(false);
const confirmationMode = ref<ConfirmationMode>(null);
const clearingDraft = ref(false);
const submissionConfirmationMode = ref<InboundSubmissionMode | null>(null);
const itemCreateOpen = ref(false);
const pendingTemplateChange = ref<{ lineId: string; templateId: number | null } | null>(null);
const templateChangeSubmitting = ref(false);
let locationAbortController: AbortController | null = null;
let templateOptionsAbortController: AbortController | null = null;
let pendingLeaveResolution: ((allowed: boolean) => void) | null = null;
let pendingPickerItem: ItemOptionResponse | null = null;
let openCreateItemAfterPicker = false;
let openPickerAfterLineEditor = false;
const templateAbortControllers = new Map<string, AbortController>();
const templateRequestVersions = new Map<string, number>();
const templateCache = new Map<number, InboundTemplateResponse>();

useNativeBackHandler({
  id: "inbound-line-editor-drawer",
  active: () => selectedLineId.value !== null,
  priority: NativeBackPriority.Drawer,
  handle: () => {
    if (selectedLineId.value === null) return { handled: false };
    closeLineEditor();
    return { handled: true, reason: "drawer" };
  },
});

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
const canAddItem = computed(
  () => selectedLineId.value === null && incompleteLine.value === null,
);
const addItemDisabledReason = computed(() => {
  if (selectedLineId.value !== null) return "请先完成或暂存当前打开的明细。";
  if (incompleteLine.value) return `请先完成“${incompleteLine.value.item.name}”的入库明细。`;
  return "";
});
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

onMounted(async () => {
  const restored = await restoreDraft();
  const removedDuplicates = removeRestoredDuplicateItems();
  // 恢复历史草稿只恢复数据，不主动进入任一明细的详情编辑模式。
  selectedLineId.value = null;
  draftItems.value.forEach((line) => {
    if (line.templateId)
      void loadLineTemplate(line, line.templateId, nextTemplateRequestVersion(line));
  });
  if (restored && sessionStorage.getItem(restoredNoticeSessionKey) !== "shown") {
    sessionStorage.setItem(restoredNoticeSessionKey, "shown");
    notice.info("已恢复上次未提交的入库草稿");
  }
  if (removedDuplicates > 0) notice.info(`已移除 ${removedDuplicates} 条重复物品明细`);
  resumeDraftSaving();
  void resetItems();
  void loadLocationOptions();
  void loadInboundTemplateOptions();
  window.addEventListener("keydown", handlePageKeydown);
});

/** 兼容旧版草稿数据，恢复时按物品 ID 保留第一条明细并清理重复项资源。 */
function removeRestoredDuplicateItems(): number {
  const seen = new Set<number>();
  const unique: InboundDraftLine[] = [];
  const duplicates: InboundDraftLine[] = [];
  for (const line of draftItems.value) {
    if (seen.has(line.item.id)) duplicates.push(line);
    else {
      seen.add(line.item.id);
      unique.push(line);
    }
  }
  if (!duplicates.length) return 0;
  duplicates.forEach((line) => {
    void deleteLineUploads(line);
    revokeLinePreviews(line);
  });
  draftItems.value = unique;
  return duplicates.length;
}

onBeforeUnmount(() => {
  locationAbortController?.abort();
  templateOptionsAbortController?.abort();
  templateAbortControllers.forEach((controller) => controller.abort());
  draftItems.value.forEach(revokeLinePreviews);
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
  void loadDefaultInboundTemplate(line);
  if (showNotice) notice.info(`已加入 ${item.name}`);
  return line;
}

function openItemPicker(): void {
  if (!canAddItem.value) {
    if (incompleteLine.value)
      notice.warning(`请先完成“${incompleteLine.value.item.name}”的入库明细。`);
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

async function loadInboundTemplateOptions(): Promise<void> {
  templateOptionsAbortController?.abort();
  const controller = new AbortController();
  templateOptionsAbortController = controller;
  templateOptionsLoading.value = true;
  templateOptionsError.value = "";
  try {
    inboundTemplates.value = await listInboundTemplates(controller.signal);
    markRemovedSelectedTemplates();
  } catch (error) {
    if (isAbortError(error)) return;
    templateOptionsError.value =
      error instanceof ApiError && error.status === 403
        ? "当前账号没有读取入库模板的权限"
        : itemErrorMessage(error, "入库模板加载失败");
    notice.error("加载入库模板失败", { detail: templateOptionsError.value });
  } finally {
    if (templateOptionsAbortController === controller) {
      templateOptionsAbortController = null;
      templateOptionsLoading.value = false;
    }
  }
}

/** 活动候选刷新后，已选但不再返回的模板视为失效，保留 ID 供用户处理。 */
function markRemovedSelectedTemplates(): void {
  const activeTemplateIds = new Set(inboundTemplates.value.map((template) => template.id));
  for (const line of draftItems.value) {
    if (line.templateId === null || activeTemplateIds.has(line.templateId)) continue;
    nextTemplateRequestVersion(line);
    templateAbortControllers.get(line.lineId)?.abort();
    templateCache.delete(line.templateId);
    line.template = null;
    line.templateState = "unresolved";
    line.templateError =
      line.templateSource === "recommended"
        ? "推荐入库模板已删除，请重新选择"
        : "所选入库模板已删除，请重新选择";
  }
}

async function loadDefaultInboundTemplate(line: InboundDraftLine): Promise<void> {
  const templateId = line.recommendedTemplateId;
  if (templateId === null || line.templateState === "unresolved") return;
  line.templateSource = "recommended";
  line.templateState = "resolving";
  await loadLineTemplate(line, templateId, nextTemplateRequestVersion(line));
}

async function loadLineTemplate(
  line: InboundDraftLine,
  templateId: number,
  requestVersion: number,
): Promise<void> {
  const cached = templateCache.get(templateId);
  if (cached) {
    if (
      templateRequestVersions.get(line.lineId) === requestVersion &&
      line.templateId === templateId
    )
      applyTemplate(line, cached);
    return;
  }
  templateAbortControllers.get(line.lineId)?.abort();
  const controller = new AbortController();
  templateAbortControllers.set(line.lineId, controller);
  line.templateState = "resolving";
  line.templateError = "";
  try {
    const template = await getInboundTemplate(templateId, controller.signal);
    if (
      templateRequestVersions.get(line.lineId) !== requestVersion ||
      line.templateId !== templateId
    )
      return;
    templateCache.set(templateId, template);
    applyTemplate(line, template);
  } catch (error) {
    if (!isAbortError(error) && templateRequestVersions.get(line.lineId) === requestVersion) {
      line.templateState =
        error instanceof ApiError && error.status === 404 ? "unresolved" : "error";
      line.templateError =
        error instanceof ApiError && error.status === 404
          ? "所选入库模板已删除，请重新选择"
          : itemErrorMessage(error, `无法加载 ${line.item.name} 的模板`);
    }
  } finally {
    if (templateAbortControllers.get(line.lineId) === controller) {
      templateAbortControllers.delete(line.lineId);
    }
  }
}

function retryLineTemplate(line: InboundDraftLine): void {
  if (line.templateId !== null)
    void loadLineTemplate(line, line.templateId, nextTemplateRequestVersion(line));
}

function applyTemplate(line: InboundDraftLine, template: InboundTemplateResponse): void {
  line.template = template;
  line.templateState = "ready";
  line.templateError = "";
  for (const field of template.fields) {
    if (
      field.default_value !== null &&
      line.extAttributes[field.field_name] === undefined &&
      field.field_type !== "file"
    ) {
      line.extAttributes[field.field_name] =
        field.field_type === "number"
          ? Number(field.default_value)
          : field.field_type === "boolean"
            ? field.default_value === "true"
            : field.default_value;
    }
  }
}

async function selectInboundTemplate(templateId: number | null): Promise<void> {
  const line = selectedLine.value;
  if (!line) return;
  if (line.templateId === templateId) return;
  if (hasTemplateDraftValues(line)) {
    pendingTemplateChange.value = { lineId: line.lineId, templateId };
    return;
  }
  await applyTemplateSelection(line, templateId);
}

async function applyTemplateSelection(
  line: InboundDraftLine,
  templateId: number | null,
): Promise<void> {
  nextTemplateRequestVersion(line);
  templateAbortControllers.get(line.lineId)?.abort();
  await deleteLineUploads(line);
  line.templateId = templateId;
  line.templateSource = templateId === null ? "none" : "manual";
  line.template = null;
  line.templateError = "";
  line.extAttributes = {};
  line.templateState = templateId === null ? "idle" : "resolving";
  if (templateId)
    void loadLineTemplate(line, templateId, templateRequestVersions.get(line.lineId) ?? 0);
}

async function confirmTemplateChange(): Promise<void> {
  const pending = pendingTemplateChange.value;
  if (!pending || templateChangeSubmitting.value) return;
  const line = draftItems.value.find((candidate) => candidate.lineId === pending.lineId);
  if (!line) {
    pendingTemplateChange.value = null;
    return;
  }
  templateChangeSubmitting.value = true;
  try {
    await applyTemplateSelection(line, pending.templateId);
    pendingTemplateChange.value = null;
  } finally {
    templateChangeSubmitting.value = false;
  }
}

function cancelTemplateChange(): void {
  if (!templateChangeSubmitting.value) pendingTemplateChange.value = null;
}

function nextTemplateRequestVersion(line: InboundDraftLine): number {
  const version = (templateRequestVersions.get(line.lineId) ?? 0) + 1;
  templateRequestVersions.set(line.lineId, version);
  return version;
}

function removeLine(lineId: string): void {
  const line = draftItems.value.find((candidate) => candidate.lineId === lineId);
  if (!line) return;
  templateAbortControllers.get(lineId)?.abort();
  templateAbortControllers.delete(lineId);
  templateRequestVersions.delete(lineId);
  void deleteLineUploads(line);
  revokeLinePreviews(line);
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
  const lineId = selectedLineId.value;
  selectedLineId.value = null;
  if (lineId)
    void nextTick(() =>
      document.querySelector<HTMLElement>(`[data-line-action="${lineId}"]`)?.focus(),
    );
}

function handleLineEditorAfterLeave(): void {
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
    await uploadImageDrafts(inboundDraftImages());
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
    const failedImage = firstFailedImage();
    if (failedImage) {
      notice.error("入库图片上传失败", { detail: failedImage.value.error });
      await focusLineTemplate(failedImage.line, failedImage.fieldName);
      return;
    }
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

function inboundDraftImages(): FileDraftValue[] {
  return draftItems.value.flatMap((line) =>
    Object.values(line.extAttributes).filter(isImageDraftValue),
  );
}

function firstFailedImage(): {
  line: InboundDraftLine;
  fieldName: string;
  value: FileDraftValue;
} | null {
  for (const line of draftItems.value) {
    for (const [fieldName, value] of Object.entries(line.extAttributes)) {
      if (isImageDraftValue(value) && value.status === "failed") return { line, fieldName, value };
    }
  }
  return null;
}

function openClearConfirmation(): void {
  if (hasDraft.value) confirmationMode.value = "clear";
}

function cancelConfirmation(): void {
  if (confirmationMode.value === "leave") pendingLeaveResolution?.(false);
  pendingLeaveResolution = null;
  confirmationMode.value = null;
}

async function confirmCurrentAction(): Promise<void> {
  if (confirmationMode.value === "leave") {
    const resolve = pendingLeaveResolution;
    pendingLeaveResolution = null;
    confirmationMode.value = null;
    resolve?.(true);
    return;
  }
  if (confirmationMode.value !== "clear") return;
  clearingDraft.value = true;
  const lines = [...draftItems.value];
  await Promise.allSettled(lines.map(deleteLineUploads));
  lines.forEach(revokeLinePreviews);
  clearLocalDraftState();
  clearingDraft.value = false;
  confirmationMode.value = null;
}

function clearLocalDraftState(): void {
  templateAbortControllers.forEach((controller) => controller.abort());
  templateAbortControllers.clear();
  templateRequestVersions.clear();
  pendingTemplateChange.value = null;
  draftItems.value.forEach(revokeLinePreviews);
  source.value = "";
  notes.value = "";
  notesOpen.value = false;
  draftItems.value = [];
  selectedLineId.value = null;
  validationAttempted.value = false;
  removePersistedDraft();
}

async function deleteLineUploads(line: InboundDraftLine): Promise<void> {
  const deletions = Object.values(line.extAttributes)
    .filter((value): value is FileDraftValue => typeof value === "object" && value?.kind === "file")
    .map((value) => {
      value.abortController?.abort();
      return value.fileId ? deleteImage(value.fileId) : Promise.resolve();
    });
  await Promise.allSettled(deletions);
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
  if (line.templateState === "resolving" || line.templateError) return focusLineTemplate(line);
  const field = line.template?.fields.find(
    (candidate) => templateFieldError(line, candidate) !== null,
  );
  if (field) return focusLineTemplate(line, field.field_name);
}

async function focusLineControl(line: InboundDraftLine, field: string): Promise<void> {
  selectedLineId.value = line.lineId;
  await nextTick();
  document
    .querySelector<HTMLElement>(`[data-line-id="${line.lineId}"][data-field="${field}"]`)
    ?.focus();
}

async function focusLineTemplate(line: InboundDraftLine, fieldName?: string): Promise<void> {
  selectedLineId.value = line.lineId;
  await nextTick();
  if (fieldName)
    document
      .querySelector<HTMLElement>(`[data-template-field="${CSS.escape(fieldName)}"]`)
      ?.focus();
  else
    (
      document.querySelector<HTMLElement>("[data-template-retry]") ??
      document.querySelector<HTMLElement>("[data-template-picker]")
    )?.focus();
}

async function focusBackendError(error: unknown): Promise<void> {
  if (!(error instanceof ApiError) || !isRecord(error.details)) return;
  const line = backendErrorLine(error);
  if (!line) return;
  validationAttempted.value = true;
  if (error.code === "item_not_found") await focusLineControl(line, "remove");
  else if (error.code === "location_not_found") await focusLineControl(line, "locationId");
  else if (error.code === "template_not_found") await focusLineTemplate(line);
  else if (error.code === "invalid_inbound_field" || error.code === "inbound_file_unavailable") {
    const fieldName =
      typeof error.details.field_name === "string" ? error.details.field_name : undefined;
    await focusLineTemplate(line, fieldName);
  } else {
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
  return invalid
    ? `请检查“${invalid.item.name}”的数量、单价、库位和模板属性。`
    : "请检查入库单信息。";
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
</script>

<style lang="scss" src="./InboundDraftPage.scss"></style>
