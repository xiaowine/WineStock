<!--
  本组件拥有出入库合并草稿页共用的工作台壳：页头/摘要、单据头、明细表骨架、
  行编辑与物品选择 Dialog 编排、清空/离开/提交三态确认与路由离开守卫。
  它不定义任何领域字段、校验或提交请求；这些经 StockDraftFlow 由领域装配注入。
-->
<template>
  <section class="route-page inbound-draft-page" :class="texts.rootClass">
    <header class="content-header inbound-draft-page__header">
      <div class="inbound-page-title">
        <div>
          <h1>{{ $title($route.meta.title) }}</h1>
        </div>
      </div>
      <div
        v-if="flow.lines.value.length > 0"
        class="content-summary inbound-draft-summary"
        :aria-label="translate(texts.summaryAriaLabel)"
      >
        <slot name="summary" />
      </div>
      <div class="inbound-page-actions">
        <button
          class="text-button inbound-clear-button"
          type="button"
          :disabled="!flow.hasDraft.value || flow.submitting.value"
          @click="requestClear"
        >
          {{ $t('stockDraft.clearDraftButton') }}
        </button>
        <template v-if="flow.lines.value.length > 0">
          <button
            class="primary-button"
            type="button"
            :disabled="flow.submitting.value"
            @click="review"
          >
            {{
              flow.submitting.value
                ? $t('stockDraft.submitting')
                : flow.canDirect.value
                  ? translate(texts.submitButtonDirect)
                  : translate(texts.submitButtonPending)
            }}
          </button>
        </template>
      </div>
    </header>

    <div class="inbound-workspace">
      <section class="inbound-step inbound-draft-step" aria-labelledby="stock-draft-step-title">
        <header class="inbound-step__header">
          <div>
            <h2 id="stock-draft-step-title">{{ translate(texts.workspaceTitle) }}</h2>
            <p>{{ $t('stockDraft.addThenConfigureHint') }}</p>
          </div>
          <div class="inbound-step__actions">
            <!-- 领域附加入口（如入库的订单导入）排在通用入口之前。 -->
            <slot name="actions" />
            <button
              class="secondary-button inbound-add-item-button inbound-scan-button"
              type="button"
              :title="$t('stockDraft.scanAddTitle')"
              :aria-label="$t('stockDraft.scanAdd')"
              @click="openScan"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M8 4H4v4M16 4h4v4M20 16v4h-4M8 20H4v-4" />
                <path d="M9 9h6v6H9z" />
              </svg>
              <span>{{ $t('stockDraft.scanAdd') }}</span>
            </button>
            <button
              class="primary-button inbound-add-item-button inbound-select-item-button"
              type="button"
              :title="$t('stockDraft.selectItemTitle')"
              @click="openPicker"
            >
              {{ $t('stockDraft.selectItem') }}
            </button>
          </div>
        </header>

        <div class="inbound-order__body" :inert="selectedLine !== null ? true : undefined">
          <section class="inbound-order-meta" :aria-label="translate(texts.metaAriaLabel)">
            <label class="inbound-order-meta__source">
              <span>{{ translate(texts.sourceLabel) }} *</span>
              <input
                ref="sourceInput"
                v-model="flow.source.value"
                :class="{
                  'inbound-control--error':
                    flow.validationAttempted.value && !flow.source.value.trim(),
                }"
                :aria-invalid="
                  flow.validationAttempted.value && !flow.source.value.trim() ? true : undefined
                "
                :aria-describedby="
                  flow.validationAttempted.value && !flow.source.value.trim()
                    ? 'stock-draft-source-error'
                    : undefined
                "
                :title="
                  flow.validationAttempted.value && !flow.source.value.trim()
                    ? $t('stockDraft.fillRequired', { field: translate(texts.sourceLabel) })
                    : undefined
                "
                type="text"
                :name="texts.sourceName"
                maxlength="128"
                :placeholder="translate(texts.sourcePlaceholder)"
              />
              <span
                v-if="flow.validationAttempted.value && !flow.source.value.trim()"
                id="stock-draft-source-error"
                class="visually-hidden"
                role="alert"
                >{{ $t('stockDraft.fillRequired', { field: translate(texts.sourceLabel) }) }}</span
              >
            </label>
            <button
              class="icon-button inbound-order-meta__notes-toggle"
              :class="{
                'inbound-order-meta__notes-toggle--filled': flow.notes.value.trim().length > 0,
              }"
              type="button"
              :title="notesToggleLabel"
              :aria-label="notesToggleLabel"
              :aria-expanded="flow.notesOpen.value"
              aria-controls="stock-draft-notes"
              @click="flow.notesOpen.value = !flow.notesOpen.value"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M5 4h14v12H9l-4 4V4Z" />
                <path d="M8 8h8M8 12h5" />
              </svg>
            </button>
            <label
              v-if="flow.notesOpen.value"
              id="stock-draft-notes"
              class="inbound-order-meta__notes"
            >
              <span>{{ $t('common.remark') }}</span>
              <input
                v-model="flow.notes.value"
                type="text"
                :name="texts.notesName"
                maxlength="1024"
                :placeholder="translate(texts.notesPlaceholder)"
              />
            </label>
          </section>

          <section
            v-if="flow.lines.value.length === 0"
            class="inbound-panel-state inbound-lines-empty"
          >
            <strong>{{ translate(texts.emptyTitle) }}</strong>
            <span>{{ translate(texts.emptyHint) }}</span>
          </section>

          <section
            v-else
            v-overlay-scrollbar
            class="inbound-lines"
            :aria-label="translate(texts.linesAriaLabel)"
          >
            <table>
              <thead>
                <tr>
                  <th scope="col">{{ $t('stockDraft.item') }}</th>
                  <th v-for="column in texts.columns" :key="column" scope="col">
                    {{ translate(column) }}
                  </th>
                  <th scope="col"><span class="visually-hidden">{{ $t('common.actions') }}</span></th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="line in flow.lines.value"
                  :key="line.lineId"
                  :class="{ 'inbound-line--selected': selectedLineId === line.lineId }"
                  tabindex="0"
                  @click="selectLine(line.lineId)"
                  @keydown.enter="selectLine(line.lineId)"
                  @keydown.space.prevent="selectLine(line.lineId)"
                >
                  <td :data-label="$t('stockDraft.item')">
                    <div class="inbound-line__identity">
                      <AuthenticatedImage
                        :file-id="line.item.image_file_id"
                        :alt="$t('stockDraft.itemImageAlt', { name: line.item.name })"
                        :size="34"
                        previewable
                        @click.stop
                        @keydown.stop
                      />
                      <div>
                        <strong :title="line.item.name">{{ line.item.name }}</strong>
                        <span>{{ line.item.sku }} · {{ line.item.unit }}</span>
                      </div>
                    </div>
                  </td>
                  <slot name="line-cells" :line="line" />
                  <td :data-label="$t('common.actions')">
                    <div class="inbound-line__actions">
                      <button
                        class="icon-button inbound-line__edit"
                        type="button"
                        :data-line-action="line.lineId"
                        :aria-label="flow.lineEditLabel(line)"
                        :title="$t('stockDraft.editItem', { name: line.item.name })"
                        @click.stop="selectLine(line.lineId)"
                      >
                        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
                          <path d="m5 17-1 3 3-1L19 7l-2-2L5 17Z" />
                          <path d="m15 7 2 2" />
                        </svg>
                      </button>
                      <button
                        class="icon-button inbound-line__remove"
                        type="button"
                        :data-line-id="line.lineId"
                        data-field="remove"
                        :aria-label="$t('stockDraft.removeItem', { name: line.item.name })"
                        :title="$t('stockDraft.removeItem', { name: line.item.name })"
                        @click.stop="flow.removeLine(line.lineId)"
                      >
                        <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
                          <path d="M4 7h16M9 7V4h6v3M7 7l1 13h8l1-13M10 11v5M14 11v5" />
                        </svg>
                      </button>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
          </section>
        </div>
      </section>
    </div>

    <ModalDialog
      :open="selectedLine !== null"
      :title="translate(texts.editorTitle)"
      :description="translate(texts.editorDescription)"
      :workspace="!texts.editorWide"
      :wide="texts.editorWide"
      @close="stashAndCloseEditor"
      @after-close="handleEditorAfterClose"
    >
      <template v-if="selectedLine" #context>
        <div class="inbound-line-editor-context">
          <AuthenticatedImage
            :file-id="selectedLine.item.image_file_id"
            :alt="$t('stockDraft.itemImageAlt', { name: selectedLine.item.name })"
            :size="34"
            previewable
          />
          <div>
            <strong
              v-copyable="{ text: selectedLine.item.name, label: $t('stockDraft.itemNameLabel') }"
              :title="selectedLine.item.name"
              >{{ selectedLine.item.name }}</strong
            >
            <span>
              <span v-copyable="{ text: selectedLine.item.sku, label: $t('stockDraft.itemCodeLabel') }">{{
                selectedLine.item.sku
              }}</span>
              · {{ selectedLine.item.unit }}
            </span>
          </div>
        </div>
      </template>
      <slot v-if="selectedLine" name="line-editor" :line="selectedLine" />
      <template #actions>
        <button
          class="secondary-button inbound-line-editor-action"
          type="button"
          @click="stashAndCloseEditor"
        >
          {{ $t('stockDraft.stashAndClose') }}
        </button>
        <button
          class="primary-button inbound-line-editor-action"
          type="button"
          @click="completeEditorAndContinue"
        >
          {{ $t('stockDraft.completeAndContinue') }}
        </button>
      </template>
    </ModalDialog>

    <ItemSelectionDialog
      :open="itemPickerOpen"
      :title="translate(texts.pickerTitle)"
      :description="$t('stockDraft.pickerHint')"
      :search-name="texts.pickerSearchName"
      :items="items"
      :search-input="searchInput"
      :loading-items="loadingItems"
      :item-error="itemError"
      :items-exhausted="itemsExhausted"
      :selected-item-ids="selectedItemIds"
      :can-create-item="canCreateItem === true"
      @close="closePicker"
      @after-close="handlePickerAfterClose"
      @update:search-input="searchInput = $event"
      @search="applySearch"
      @reset-items="resetItems"
      @load-next-items="loadNextItems"
      @scroll-items="handleItemScroll"
      @list-element="setItemList"
      @select-item="handleItemSelected"
      @create-item="requestCreateItem"
    />

    <ModalDialog
      :open="confirmMode !== null"
      :title="confirmTitle"
      :description="confirmDescription"
      :busy="flow.submitting.value"
      @close="cancelConfirmation"
    >
      <slot v-if="confirmMode === 'submit'" name="submit-summary" />
      <p v-else>
        {{
          confirmMode === "clear" ? $t('stockDraft.clearIrreversible') : translate(texts.leaveBody)
        }}
      </p>
      <template #actions>
        <button
          class="secondary-button"
          type="button"
          :disabled="flow.submitting.value"
          @click="cancelConfirmation"
        >
          {{ confirmMode === "submit" ? $t('stockDraft.backToCheck') : $t('common.cancel') }}
        </button>
        <button
          :class="confirmMode === 'clear' ? 'danger-button' : 'primary-button'"
          type="button"
          :disabled="flow.submitting.value"
          @click="confirmCurrentAction"
        >
          {{ confirmActionLabel }}
        </button>
      </template>
    </ModalDialog>

    <BarcodeScanDialog
      :open="scanOpen"
      :title="$t('stockDraft.scanAdd')"
      :description="$t('stockDraft.scanDialogDescription')"
      :status-text="scanStatusText"
      @close="closeScan"
      @detect="handleScanDetect"
    />

    <!-- 领域附加 Dialog（如入库的新建物品）挂在工作台根节点内，保证页面保持单根以配合路由切换动效。 -->
    <slot name="extras" />
  </section>
</template>

<script setup lang="ts" generic="L extends StockDraftLineBase">
import { computed, onBeforeUnmount, onMounted, ref, type VNode } from "vue";
import { useI18n } from "vue-i18n";
import { onBeforeRouteLeave } from "vue-router";
import { listItemOptions, type ItemOptionResponse } from "../../api/items";
import {
  ApiConfigurationError,
  ApiError,
  ApiNetworkError,
  ApiResponseError,
} from "../../api/errors";
import { parseLcscBagCode } from "../../lcsc/bagCode";
import AuthenticatedImage from "../attributes/AuthenticatedImage.vue";
import BarcodeScanDialog from "../barcode/BarcodeScanDialog.vue";
import ItemSelectionDialog from "../items/ItemSelectionDialog.vue";
import ModalDialog from "../ModalDialog.vue";
import { useStockItemCatalog } from "../../composables/useStockItemCatalog";
import { notice } from "../../notices/notice";
import { trackTelemetryEvent } from "../../telemetry/clarity";
import { translateMessageOrNull } from "../../i18n";
import type {
  StockDraftFlow,
  StockDraftLineBase,
  StockDraftTexts,
  StockDraftWorkspaceHandle,
} from "../../pages/stock-draft/flow";

const props = defineProps<{
  flow: StockDraftFlow<L>;
  texts: StockDraftTexts;
  handle: StockDraftWorkspaceHandle;
  canCreateItem?: boolean;
}>();

defineSlots<{
  summary(): VNode[];
  actions?(): VNode[];
  "line-cells"(props: { line: L }): VNode[];
  "line-editor"(props: { line: L }): VNode[];
  "submit-summary"(): VNode[];
  extras(): VNode[];
}>();

type ConfirmMode = "clear" | "leave" | "submit" | null;

/** 工作台文案（texts）以消息键注入；渲染期翻译，键缺失时原样回退。 */
function translate(text: string): string {
  return translateMessageOrNull(text) ?? text;
}

const { t } = useI18n();
const selectedLineId = ref<string | null>(null);
const itemPickerOpen = ref(false);
const confirmMode = ref<ConfirmMode>(null);
const sourceInput = ref<HTMLInputElement | null>(null);
const scanOpen = ref(false);
const scanStatusText = ref("");
/** 扫码串行会话进行中：行编辑或快速新建结束后自动回到扫码，直到用户主动关闭扫码。 */
const scanFlowActive = ref(false);
let scanLookupBusy = false;
let pendingPickerItem: ItemOptionResponse | null = null;
let openCreateItemAfterPicker = false;
let afterEditorTarget: "picker" | "scan" | null = null;
let pendingLeaveResolution: ((allowed: boolean) => void) | null = null;

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
} = useStockItemCatalog(pickerErrorMessage);

const selectedLine = computed(
  () => props.flow.lines.value.find((line) => line.lineId === selectedLineId.value) ?? null,
);
const selectedItemIds = computed<ReadonlySet<number>>(
  () => new Set(props.flow.lines.value.map((line) => line.item.id)),
);
const incompleteLine = computed(
  () => props.flow.lines.value.find((line) => props.flow.lineError(line) !== null) ?? null,
);
const notesToggleLabel = computed(() =>
  props.flow.notesOpen.value
    ? t("stockDraft.collapseNotes")
    : props.flow.notes.value.trim()
      ? t("stockDraft.notesFilled")
      : t("stockDraft.addNotes"),
);
const confirmTitle = computed(() => {
  if (confirmMode.value === "clear") return translate(props.texts.clearTitle);
  if (confirmMode.value === "leave") return t("stockDraft.leavePageTitle");
  return props.flow.canDirect.value
    ? translate(props.texts.submitTitleDirect)
    : translate(props.texts.submitTitlePending);
});
const confirmDescription = computed(() => {
  if (confirmMode.value === "clear") return translate(props.texts.clearDescription);
  if (confirmMode.value === "leave") return t("stockDraft.draftSavedLocally");
  return props.flow.canDirect.value
    ? translate(props.texts.submitDescriptionDirect)
    : translate(props.texts.submitDescriptionPending);
});
const confirmActionLabel = computed(() => {
  if (props.flow.submitting.value) return t("stockDraft.submitting");
  if (confirmMode.value === "clear") return t("stockDraft.confirmClear");
  if (confirmMode.value === "leave") return t("stockDraft.confirmLeave");
  return props.flow.canDirect.value
    ? translate(props.texts.submitConfirmDirect)
    : translate(props.texts.submitConfirmPending);
});

onMounted(() => {
  // 装配层经 handle 反向调用编辑器/选择器，避免领域逻辑直接依赖组件实例。
  props.handle.openLineEditor = selectLine;
  props.handle.openItemPicker = openPicker;
  props.flow.sourceInput.value = sourceInput.value;
  window.addEventListener("keydown", handlePageKeydown);
  void resetItems();
});

onBeforeUnmount(() => {
  props.handle.openLineEditor = () => {};
  props.handle.openItemPicker = () => {};
  window.removeEventListener("keydown", handlePageKeydown);
  pendingLeaveResolution?.(false);
  pendingLeaveResolution = null;
});

onBeforeRouteLeave(() => {
  if (!props.flow.hasDraft.value) return true;
  confirmMode.value = "leave";
  return new Promise<boolean>((resolve) => {
    pendingLeaveResolution = resolve;
  });
});

function pickerErrorMessage(error: unknown): string {
  if (error instanceof ApiError)
    return translateMessageOrNull(`error.${error.code}`) ?? error.message;
  if (error instanceof ApiConfigurationError) return error.message;
  if (error instanceof ApiNetworkError)
    return translateMessageOrNull("error.network_unavailable") ?? "error.network_unavailable";
  if (error instanceof ApiResponseError)
    return (
      translateMessageOrNull("stockDraft.responseInvalidCheckVersions") ??
      "stockDraft.responseInvalidCheckVersions"
    );
  return translateMessageOrNull("stockDraft.loadItemsFailedRetry") ?? "stockDraft.loadItemsFailedRetry";
}

function openPicker(): void {
  // 用户显式选择手动添加即结束扫码会话，后续行编辑完成回到选择器。
  scanFlowActive.value = false;
  if (focusIncompleteLine()) return;
  itemPickerOpen.value = true;
  void resetItems();
}

function openScan(): void {
  // 先声明扫码意图：被未完成明细拦截时，完成该行后仍自动回到扫码。
  scanFlowActive.value = true;
  if (focusIncompleteLine()) return;
  scanStatusText.value = "";
  scanOpen.value = true;
}

function closeScan(): void {
  scanOpen.value = false;
  scanFlowActive.value = false;
  scanStatusText.value = "";
}

/** 存在未完成明细时打开该行并阻止新增入口。 */
function focusIncompleteLine(): boolean {
  if (!incompleteLine.value) return false;
  selectLine(incompleteLine.value.lineId);
  notice.warning(t("stockDraft.finishCurrentLine"), {
    detail: t("stockDraft.reopenedLineConfig", { name: incompleteLine.value.item.name }),
  });
  return true;
}

/** 扫码识别：只接受立创料袋码；命中即进入串行明细确认，未命中交由领域接管或就地提示。 */
async function handleScanDetect(text: string): Promise<void> {
  if (scanLookupBusy) return;
  const bagCode = parseLcscBagCode(text);
  if (!bagCode) {
    scanStatusText.value = t("stockDraft.notBagCodeIgnored");
    return;
  }
  const sku = bagCode.productCode;
  const existing = props.flow.lines.value.find(
    (line) => line.item.sku.trim().toUpperCase() === sku,
  );
  if (existing) {
    notice.info(t("stockDraft.itemAlreadyInDraft", { name: existing.item.name }), {
      detail: t("stockDraft.lineOpened"),
    });
    scanOpen.value = false;
    selectLine(existing.lineId);
    return;
  }

  scanLookupBusy = true;
  scanStatusText.value = t("stockDraft.searchingSku", { sku });
  try {
    const response = await listItemOptions(sku, 1, 20);
    const item =
      response.items.find((candidate) => candidate.sku.trim().toUpperCase() === sku) ?? null;
    if (!item) {
      if (props.flow.onScanItemMissing?.(bagCode)) {
        scanStatusText.value = "";
        scanOpen.value = false;
      } else {
        scanStatusText.value = t("stockDraft.skuNotFound", { sku });
      }
      return;
    }
    const line = props.flow.addItem(item, { silent: true });
    props.flow.onScanItemAdded?.(line, bagCode);
    trackTelemetryEvent("bag_scan_matched");
    scanStatusText.value = t("stockDraft.addedContinueScan", { name: item.name });
    scanOpen.value = false;
    selectLine(line.lineId);
  } catch (error) {
    scanStatusText.value = t("stockDraft.scanAgain", { message: pickerErrorMessage(error) });
  } finally {
    scanLookupBusy = false;
  }
}

function closePicker(): void {
  pendingPickerItem = null;
  openCreateItemAfterPicker = false;
  itemPickerOpen.value = false;
}

function handleItemSelected(item: ItemOptionResponse): void {
  pendingPickerItem = item;
  itemPickerOpen.value = false;
}

function requestCreateItem(): void {
  pendingPickerItem = null;
  openCreateItemAfterPicker = true;
  itemPickerOpen.value = false;
}

function handlePickerAfterClose(): void {
  if (openCreateItemAfterPicker) {
    openCreateItemAfterPicker = false;
    props.flow.onCreateItemRequest?.();
    return;
  }
  const item = pendingPickerItem;
  pendingPickerItem = null;
  if (!item) return;
  const line = props.flow.addItem(item);
  selectLine(line.lineId);
}

function selectLine(lineId: string): void {
  const line = props.flow.lines.value.find((candidate) => candidate.lineId === lineId);
  if (!line) return;
  props.flow.onEditorOpen(line);
  selectedLineId.value = lineId;
}

function stashAndCloseEditor(): void {
  // 扫码会话中暂存同样回到扫码，保持批量节奏；非扫码会话保持原样不弹层。
  afterEditorTarget = scanFlowActive.value ? "scan" : null;
  const line = selectedLine.value;
  if (line) props.flow.onEditorStash(line);
  selectedLineId.value = null;
}

function completeEditorAndContinue(): void {
  const line = selectedLine.value;
  if (!line) return;
  if (!props.flow.commitEditor(line)) return;
  afterEditorTarget = scanFlowActive.value ? "scan" : "picker";
  selectedLineId.value = null;
}

function handleEditorAfterClose(): void {
  const target = afterEditorTarget;
  afterEditorTarget = null;
  if (target === "scan") {
    scanOpen.value = true;
    return;
  }
  if (target === "picker") {
    itemPickerOpen.value = true;
    void resetItems();
  }
}

function requestClear(): void {
  if (props.flow.hasDraft.value) confirmMode.value = "clear";
}

function review(): void {
  if (!props.flow.reviewGate()) return;
  confirmMode.value = "submit";
}

function cancelConfirmation(): void {
  if (props.flow.submitting.value) return;
  if (confirmMode.value === "leave") pendingLeaveResolution?.(false);
  pendingLeaveResolution = null;
  confirmMode.value = null;
}

async function confirmCurrentAction(): Promise<void> {
  if (confirmMode.value === "leave") {
    const resolve = pendingLeaveResolution;
    pendingLeaveResolution = null;
    confirmMode.value = null;
    resolve?.(true);
    return;
  }
  if (confirmMode.value === "clear") {
    props.flow.clearDraft();
    confirmMode.value = null;
    return;
  }
  if (confirmMode.value !== "submit" || props.flow.submitting.value) return;
  const outcome = await props.flow.performSubmit();
  if (outcome === "close") confirmMode.value = null;
}

function setItemList(element: unknown): void {
  itemList.value = element instanceof HTMLElement ? element : null;
}

function handlePageKeydown(event: KeyboardEvent): void {
  if (event.key !== "Escape" || confirmMode.value !== null) return;
  if (selectedLineId.value) stashAndCloseEditor();
}
</script>
