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
          <h1>{{ $route.meta.title }}</h1>
        </div>
      </div>
      <div
        v-if="flow.lines.value.length > 0"
        class="content-summary inbound-draft-summary"
        :aria-label="texts.summaryAriaLabel"
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
          清空草稿
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
                ? "正在提交…"
                : flow.canDirect.value
                  ? texts.submitButtonDirect
                  : texts.submitButtonPending
            }}
          </button>
        </template>
      </div>
    </header>

    <div class="inbound-workspace">
      <section class="inbound-step inbound-draft-step" aria-labelledby="stock-draft-step-title">
        <header class="inbound-step__header">
          <div>
            <h2 id="stock-draft-step-title">{{ texts.workspaceTitle }}</h2>
            <p>添加后立即配置该物品；再次添加会先返回未完成明细。</p>
          </div>
          <div class="inbound-step__actions">
            <!-- 领域附加入口（如入库的订单导入）排在通用入口之前。 -->
            <slot name="actions" />
            <button
              class="secondary-button inbound-add-item-button inbound-scan-button"
              type="button"
              title="扫描立创料袋二维码添加物品"
              aria-label="扫码添加物品"
              @click="openScan"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M8 4H4v4M16 4h4v4M20 16v4h-4M8 20H4v-4" />
                <path d="M9 9h6v6H9z" />
              </svg>
              <span>扫码添加</span>
            </button>
            <button
              class="primary-button inbound-add-item-button inbound-select-item-button"
              type="button"
              title="选择物品并配置明细"
              @click="openPicker"
            >
              选择物品
            </button>
          </div>
        </header>

        <div class="inbound-order__body" :inert="selectedLine !== null ? true : undefined">
          <section class="inbound-order-meta" :aria-label="texts.metaAriaLabel">
            <label class="inbound-order-meta__source">
              <span>{{ texts.sourceLabel }} *</span>
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
                    ? `请填写${texts.sourceLabel}`
                    : undefined
                "
                type="text"
                :name="texts.sourceName"
                maxlength="128"
                :placeholder="texts.sourcePlaceholder"
              />
              <span
                v-if="flow.validationAttempted.value && !flow.source.value.trim()"
                id="stock-draft-source-error"
                class="visually-hidden"
                role="alert"
                >请填写{{ texts.sourceLabel }}</span
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
              <span>备注</span>
              <input
                v-model="flow.notes.value"
                type="text"
                :name="texts.notesName"
                maxlength="1024"
                :placeholder="texts.notesPlaceholder"
              />
            </label>
          </section>

          <section
            v-if="flow.lines.value.length === 0"
            class="inbound-panel-state inbound-lines-empty"
          >
            <strong>{{ texts.emptyTitle }}</strong>
            <span>{{ texts.emptyHint }}</span>
          </section>

          <section v-else class="inbound-lines" :aria-label="texts.linesAriaLabel">
            <table>
              <thead>
                <tr>
                  <th scope="col">物品</th>
                  <th v-for="column in texts.columns" :key="column" scope="col">{{ column }}</th>
                  <th scope="col"><span class="visually-hidden">操作</span></th>
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
                  <td data-label="物品">
                    <div class="inbound-line__identity">
                      <AuthenticatedImage
                        :file-id="line.item.image_file_id"
                        :alt="line.item.name + ' 主图'"
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
                  <td data-label="操作">
                    <div class="inbound-line__actions">
                      <button
                        class="icon-button inbound-line__edit"
                        type="button"
                        :data-line-action="line.lineId"
                        :aria-label="flow.lineEditLabel(line)"
                        :title="'编辑 ' + line.item.name"
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
                        :aria-label="'移除 ' + line.item.name"
                        :title="'移除 ' + line.item.name"
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
      :title="texts.editorTitle"
      :description="texts.editorDescription"
      :workspace="!texts.editorWide"
      :wide="texts.editorWide"
      @close="stashAndCloseEditor"
      @after-close="handleEditorAfterClose"
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
            <strong
              v-copyable="{ text: selectedLine.item.name, label: '物品名称' }"
              :title="selectedLine.item.name"
              >{{ selectedLine.item.name }}</strong
            >
            <span>
              <span v-copyable="{ text: selectedLine.item.sku, label: '物品编号' }">{{
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
          暂存并关闭
        </button>
        <button
          class="primary-button inbound-line-editor-action"
          type="button"
          @click="completeEditorAndContinue"
        >
          完成并继续添加
        </button>
      </template>
    </ModalDialog>

    <ItemSelectionDialog
      :open="itemPickerOpen"
      :title="texts.pickerTitle"
      description="选择一项后进入明细配置。"
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
      <p v-else>{{ confirmMode === "clear" ? "此操作无法撤销。" : texts.leaveBody }}</p>
      <template #actions>
        <button
          class="secondary-button"
          type="button"
          :disabled="flow.submitting.value"
          @click="cancelConfirmation"
        >
          {{ confirmMode === "submit" ? "返回检查" : "取消" }}
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
      title="扫码添加物品"
      description="对准立创料袋上的二维码，识别后进入该物品的明细配置。"
      :status-text="scanStatusText"
      @close="closeScan"
      @detect="handleScanDetect"
    />

    <!-- 领域附加 Dialog（如入库的新建物品）挂在工作台根节点内，保证页面保持单根以配合路由切换动效。 -->
    <slot name="extras" />
  </section>
</template>

<script setup lang="ts" generic="L extends StockDraftLineBase">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
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
  summary(): unknown;
  actions?(): unknown;
  "line-cells"(props: { line: L }): unknown;
  "line-editor"(props: { line: L }): unknown;
  "submit-summary"(): unknown;
  extras(): unknown;
}>();

type ConfirmMode = "clear" | "leave" | "submit" | null;

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
    ? "收起备注"
    : props.flow.notes.value.trim()
      ? "备注已填写"
      : "添加备注",
);
const confirmTitle = computed(() => {
  if (confirmMode.value === "clear") return props.texts.clearTitle;
  if (confirmMode.value === "leave") return "离开当前页面？";
  return props.flow.canDirect.value
    ? props.texts.submitTitleDirect
    : props.texts.submitTitlePending;
});
const confirmDescription = computed(() => {
  if (confirmMode.value === "clear") return props.texts.clearDescription;
  if (confirmMode.value === "leave") return "当前草稿已自动保存在本机，离开后仍可恢复。";
  return props.flow.canDirect.value
    ? props.texts.submitDescriptionDirect
    : props.texts.submitDescriptionPending;
});
const confirmActionLabel = computed(() => {
  if (props.flow.submitting.value) return "正在提交…";
  if (confirmMode.value === "clear") return "确认清空";
  if (confirmMode.value === "leave") return "确认离开";
  return props.flow.canDirect.value
    ? props.texts.submitConfirmDirect
    : props.texts.submitConfirmPending;
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
    return error.status === 403 ? "当前账号没有读取物品的权限" : error.message;
  if (error instanceof ApiConfigurationError) return error.message;
  if (error instanceof ApiNetworkError) return "无法连接到 WineStock 服务";
  if (error instanceof ApiResponseError) return "服务响应格式无效，请检查前后端版本";
  return "加载物品失败，请重试";
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
  notice.warning("请先完成当前明细", {
    detail: `已重新打开“${incompleteLine.value.item.name}”的配置界面。`,
  });
  return true;
}

/** 扫码识别：只接受立创料袋码；命中即进入串行明细确认，未命中交由领域接管或就地提示。 */
async function handleScanDetect(text: string): Promise<void> {
  if (scanLookupBusy) return;
  const bagCode = parseLcscBagCode(text);
  if (!bagCode) {
    scanStatusText.value = "识别到的内容不是立创料袋码，已忽略。";
    return;
  }
  const sku = bagCode.productCode;
  const existing = props.flow.lines.value.find(
    (line) => line.item.sku.trim().toUpperCase() === sku,
  );
  if (existing) {
    notice.info(`“${existing.item.name}”已在草稿中`, { detail: "已打开该行明细。" });
    scanOpen.value = false;
    selectLine(existing.lineId);
    return;
  }

  scanLookupBusy = true;
  scanStatusText.value = `正在查找 ${sku}…`;
  try {
    const response = await listItemOptions(sku, 1, 20);
    const item =
      response.items.find((candidate) => candidate.sku.trim().toUpperCase() === sku) ?? null;
    if (!item) {
      if (props.flow.onScanItemMissing?.(bagCode)) {
        scanStatusText.value = "";
        scanOpen.value = false;
      } else {
        scanStatusText.value = `库中没有编号 ${sku} 的物品。`;
      }
      return;
    }
    const line = props.flow.addItem(item, { silent: true });
    props.flow.onScanItemAdded?.(line, bagCode);
    trackTelemetryEvent("bag_scan_matched");
    scanStatusText.value = `已添加 ${item.name}，可继续扫下一袋。`;
    scanOpen.value = false;
    selectLine(line.lineId);
  } catch (error) {
    scanStatusText.value = `${pickerErrorMessage(error)}，请再扫一次。`;
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
