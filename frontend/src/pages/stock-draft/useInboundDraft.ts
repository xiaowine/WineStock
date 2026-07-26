// 本文件拥有合并草稿页的入库域装配：行模型接线、库位加载、校验聚焦与提交编排。
// 它复用旧入库页的 model 与持久化（含同一 localStorage 键），不修改旧页面任何文件。
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { createInbound, listLocations, type LocationResponse } from "../../api/inbound";
import type { ItemOptionResponse } from "../../api/items";
import { ApiError } from "../../api/errors";
import { useInboundDraftPersistence } from "../../composables/useInboundDraftPersistence";
import type { LcscBagCode } from "../../lcsc/bagCode";
import type { LcscOrderImportPayload } from "../../components/stock-draft/LcscOrderImportDialog.vue";
import { notice } from "../../notices/notice";
import { authSession } from "../../auth/session";
import { hasPermission, stockPermissions } from "../../auth/permissions";
import {
  buildInboundRequest,
  createDraftLine,
  lineReady,
  lineSubtotal,
  positiveNumber,
  validQuantity,
  validUnitPrice,
  type InboundDraftLine,
} from "../inbound-draft/model";
import {
  formatQuantity,
  inboundSubmitErrorMessage,
  isAbortError,
  itemErrorMessage,
} from "../inbound-draft/presentation";
import type { StockDraftFlow, StockDraftTexts, StockDraftWorkspaceHandle } from "./flow";

const restoredNoticeSessionKey = "winestock.inbound.restored-notice";
/** 「本单不再提示」随入库草稿持久化；提交或清空草稿时移除。 */
const scanSourceSuppressedKey = "winestock.inbound.scan-source-suppressed";

/** 入库域的工作台文案与列配置。 */
export const inboundDraftTexts: StockDraftTexts = {
  rootClass: "stock-draft-page--inbound",
  summaryAriaLabel: "当前入库草稿摘要",
  workspaceTitle: "入库单信息与明细",
  metaAriaLabel: "入库单基础信息",
  sourceLabel: "来源",
  sourceName: "inbound_source",
  sourcePlaceholder: "供应商名称或采购单号",
  notesName: "inbound_notes",
  notesPlaceholder: "可选，记录采购或收货说明",
  linesAriaLabel: "入库明细",
  emptyTitle: "还没有入库明细",
  emptyHint: "点击“添加物品”选择一项，完成对应入库明细。",
  columns: ["数量", "单价 / 小计", "库位", "批次"],
  editorTitle: "入库物品明细",
  editorDescription: "数量、单价和收货信息属于同一条明细；完成后才能继续添加下一项。",
  editorWide: false,
  pickerTitle: "选择入库物品",
  pickerSearchName: "inbound_item_search",
  clearTitle: "清空入库草稿？",
  clearDescription: "所有未提交明细都会被删除。",
  leaveBody: "确认离开当前入库流程吗？",
  submitTitleDirect: "确认直接入库？",
  submitTitlePending: "确认提交审核？",
  submitDescriptionDirect: "提交后将立即增加库存并写入库存流水。",
  submitDescriptionPending: "提交后单据进入待审批状态，审批通过前不会增加库存。",
  submitButtonDirect: "直接入库",
  submitButtonPending: "提交审核",
  submitConfirmDirect: "确认并入库",
  submitConfirmPending: "确认提交",
};

/** 组装入库域草稿流；handle 由工作台挂载后回填。 */
export function useInboundDraft(handle: StockDraftWorkspaceHandle) {
  const source = ref("");
  const notes = ref("");
  const notesOpen = ref(false);
  const lines = ref<InboundDraftLine[]>([]);
  const validationAttempted = ref(false);
  const submitting = ref(false);
  const sourceInput = ref<HTMLInputElement | null>(null);
  const locations = ref<LocationResponse[]>([]);
  const locationError = ref("");
  const itemCreateOpen = ref(false);
  const orderImportOpen = ref(false);
  let locationAbortController: AbortController | null = null;

  // 扫码状态：来源提示条、同单去重、快速新建的待应用料袋信息。
  const scanOrderPrompt = ref<string | null>(null);
  const scanSourceSuppressed = ref(false);
  const scanLcscCode = ref("");
  const askedOrderNos = new Set<string>();
  let pendingScanBagCode: LcscBagCode | null = null;

  const hasDraft = computed(
    () => source.value.trim().length > 0 || notes.value.trim().length > 0 || lines.value.length > 0,
  );
  const canDirect = computed(() =>
    hasPermission(authSession.value?.user.permissions, stockPermissions.inboundApprove),
  );
  const canCreateItem = computed(() =>
    hasPermission(authSession.value?.user.permissions, stockPermissions.itemManage),
  );
  const draftQuantity = computed(() =>
    lines.value.reduce((total, line) => total + positiveNumber(line.quantity), 0),
  );
  const draftTotal = computed(() =>
    lines.value.reduce((total, line) => total + lineSubtotal(line), 0),
  );
  const quantitySummary = computed(() => {
    const units = new Set(lines.value.map((line) => line.item.unit).filter(Boolean));
    if (units.size === 1) return `${formatQuantity(draftQuantity.value)} ${Array.from(units)[0]}`;
    return lines.value.length ? "按明细分别计量" : "0";
  });
  const draftAmountReady = computed(() => lines.value.length > 0 && lines.value.every(lineReady));

  const { restoreDraft, resumeDraftSaving, removePersistedDraft } = useInboundDraftPersistence(
    source,
    notes,
    notesOpen,
    lines,
    hasDraft,
  );

  onMounted(() => {
    const restored = restoreDraft();
    const removedDuplicates = removeRestoredDuplicateItems();
    if (restored && sessionStorage.getItem(restoredNoticeSessionKey) !== "shown") {
      sessionStorage.setItem(restoredNoticeSessionKey, "shown");
      notice.info("已恢复上次未提交的入库草稿");
    }
    if (removedDuplicates > 0) notice.info(`已移除 ${removedDuplicates} 条重复物品明细`);
    scanSourceSuppressed.value = localStorage.getItem(scanSourceSuppressedKey) === "1";
    resumeDraftSaving();
    void loadLocationOptions();
  });

  onBeforeUnmount(() => locationAbortController?.abort());

  /** 兼容旧版草稿数据，恢复时按物品 ID 保留第一条明细。 */
  function removeRestoredDuplicateItems(): number {
    const seen = new Set<number>();
    const unique: InboundDraftLine[] = [];
    let duplicates = 0;
    for (const line of lines.value) {
      if (seen.has(line.item.id)) duplicates += 1;
      else {
        seen.add(line.item.id);
        unique.push(line);
      }
    }
    if (!duplicates) return 0;
    lines.value = unique;
    return duplicates;
  }

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

  function lineError(line: InboundDraftLine): string | null {
    return lineReady(line) ? null : "请补齐数量、单价和库位";
  }

  function addItem(item: ItemOptionResponse, options?: { silent?: boolean }): InboundDraftLine {
    const existing = lines.value.find((line) => line.item.id === item.id);
    if (existing) return existing;
    const line = createDraftLine(item);
    lines.value.push(line);
    if (!options?.silent) notice.info(`已加入 ${item.name}`);
    return line;
  }

  function removeLine(lineId: string): void {
    const line = lines.value.find((candidate) => candidate.lineId === lineId);
    if (!line) return;
    lines.value = lines.value.filter((candidate) => candidate.lineId !== lineId);
    notice.info(`已移除 ${line.item.name}`);
  }

  function commitEditor(line: InboundDraftLine): boolean {
    validationAttempted.value = true;
    if (!lineReady(line)) {
      notice.warning("当前明细尚未完成", { detail: `请先补齐“${line.item.name}”的必填信息。` });
      void focusLineError(line);
      return false;
    }
    return true;
  }

  function reviewGate(): boolean {
    validationAttempted.value = true;
    if (source.value.trim().length > 0 && lines.value.length > 0 && lines.value.every(lineReady))
      return true;
    notice.warning("入库单信息尚未填写完整", { detail: blockingReason() });
    void focusFirstError();
    return false;
  }

  async function performSubmit(): Promise<"close" | "keep"> {
    submitting.value = true;
    try {
      const created = await createInbound(
        buildInboundRequest(
          source.value,
          notes.value,
          lines.value,
          canDirect.value ? "direct" : "pending_approval",
        ),
      );
      if (created.submission_mode === "direct") {
        notice.success("入库成功", { detail: `单号 #${created.id} 已完成入库，库存已更新。` });
      } else {
        notice.success("入库单已提交", { detail: `单号 #${created.id} 已进入待审批状态。` });
      }
      clearDraft();
      return "close";
    } catch (error) {
      const message = inboundSubmitErrorMessage(error);
      const errorLine = backendErrorLine(error);
      if (error instanceof ApiError && error.code === "item_not_found" && errorLine) {
        message.title = `“${errorLine.item.name}”已失效，请从入库单移除`;
      }
      notice.error(message.title, { detail: message.detail });
      await nextTick();
      await focusBackendError(error);
      return "close";
    } finally {
      submitting.value = false;
    }
  }

  function clearDraft(): void {
    source.value = "";
    notes.value = "";
    notesOpen.value = false;
    lines.value = [];
    validationAttempted.value = false;
    scanOrderPrompt.value = null;
    scanSourceSuppressed.value = false;
    askedOrderNos.clear();
    pendingScanBagCode = null;
    localStorage.removeItem(scanSourceSuppressedKey);
    removePersistedDraft();
  }

  async function focusFirstError(): Promise<void> {
    if (lines.value.length === 0) {
      handle.openItemPicker();
      await nextTick();
      document.querySelector<HTMLElement>('[name="inbound_item_search"]')?.focus();
      return;
    }
    if (!source.value.trim()) {
      sourceInput.value?.focus();
      return;
    }
    for (const line of lines.value) {
      if (!lineReady(line)) return focusLineError(line);
    }
  }

  async function focusLineError(line: InboundDraftLine): Promise<void> {
    if (!validQuantity(line.quantity)) return focusLineControl(line, "quantity");
    if (!validUnitPrice(line.unitPrice)) return focusLineControl(line, "unitPrice");
    if (line.locationId === null) return focusLineControl(line, "locationId");
  }

  async function focusLineControl(line: InboundDraftLine, field: string): Promise<void> {
    handle.openLineEditor(line.lineId);
    await nextTick();
    document
      .querySelector<HTMLElement>(`[data-line-id="${line.lineId}"][data-field="${field}"]`)
      ?.focus();
  }

  async function focusBackendError(error: unknown): Promise<void> {
    const line = backendErrorLine(error);
    if (!line) return;
    validationAttempted.value = true;
    if (error instanceof ApiError && error.code === "item_not_found")
      await focusLineControl(line, "remove");
    else if (error instanceof ApiError && error.code === "location_not_found")
      await focusLineControl(line, "locationId");
    else handle.openLineEditor(line.lineId);
  }

  function backendErrorLine(error: unknown): InboundDraftLine | null {
    if (!(error instanceof ApiError) || typeof error.details !== "object" || error.details === null)
      return null;
    const details = error.details as Record<string, unknown>;
    const lineIndex = typeof details.line_index === "number" ? details.line_index : -1;
    return lines.value[lineIndex] ?? null;
  }

  function blockingReason(): string {
    if (!source.value.trim()) return "请填写入库来源。";
    if (!lines.value.length) return "请至少添加一条入库明细。";
    const invalid = lines.value.find((line) => !lineReady(line));
    return invalid ? `请检查“${invalid.item.name}”的数量、单价和库位。` : "请检查入库单信息。";
  }

  function handleItemCreated(item: ItemOptionResponse): void {
    itemCreateOpen.value = false;
    scanLcscCode.value = "";
    const line = addItem(item, { silent: true });
    if (pendingScanBagCode) {
      applyScanToLine(line, pendingScanBagCode);
      pendingScanBagCode = null;
    }
    handle.openLineEditor(line.lineId);
    notice.success("物品已创建并加入入库单", { detail: item.name });
  }

  /**
   * 订单导入确认：命中行以订购数量与单价批量入草稿，库位留待逐条补齐；
   * 已在草稿中的物品由导入 Dialog 预先排除，这里的去重仅是兜底。
   */
  function importOrderLines(payload: LcscOrderImportPayload): void {
    let added = 0;
    for (const row of payload.rows) {
      if (lines.value.some((line) => line.item.id === row.item.id)) continue;
      const line = addItem(row.item, { silent: true });
      line.quantity = row.quantity;
      line.unitPrice = row.unitPrice;
      added += 1;
    }
    if (payload.applySource && payload.orderNo && !source.value.trim()) {
      source.value = `立创 ${payload.orderNo}`;
      askedOrderNos.add(payload.orderNo);
    }
    orderImportOpen.value = false;
    if (added > 0) {
      notice.success(`已导入 ${added} 条入库明细`, { detail: "请逐条补齐库位后提交。" });
    } else {
      notice.info("没有新的明细需要导入");
    }
  }

  /**
   * 扫码命中（或扫码新建完成）后的入库预填：袋内数量可改，来源按规则询问。
   * 扫码路径的行总是刚创建的（重复扫描在工作台层已拦截），覆盖默认数量是安全的。
   */
  function applyScanToLine(line: InboundDraftLine, bagCode: LcscBagCode): void {
    if (bagCode.quantity !== null) line.quantity = bagCode.quantity;
    proposeScanSource(bagCode.orderNo);
  }

  /** 来源已填完全静默；「忽略」只挡同一订单号；「不再提示」挡整单。 */
  function proposeScanSource(orderNo: string | null): void {
    if (!orderNo || scanSourceSuppressed.value) return;
    if (source.value.trim().length > 0) return;
    if (askedOrderNos.has(orderNo)) return;
    scanOrderPrompt.value = orderNo;
  }

  function applyScanOrderNo(): void {
    const orderNo = scanOrderPrompt.value;
    if (!orderNo) return;
    if (!source.value.trim()) source.value = `立创 ${orderNo}`;
    askedOrderNos.add(orderNo);
    scanOrderPrompt.value = null;
  }

  function ignoreScanOrderNo(): void {
    if (scanOrderPrompt.value) askedOrderNos.add(scanOrderPrompt.value);
    scanOrderPrompt.value = null;
  }

  function suppressScanOrderPrompt(): void {
    scanSourceSuppressed.value = true;
    localStorage.setItem(scanSourceSuppressedKey, "1");
    scanOrderPrompt.value = null;
  }

  const flow: StockDraftFlow<InboundDraftLine> = {
    source,
    notes,
    notesOpen,
    lines,
    validationAttempted,
    submitting,
    hasDraft,
    canDirect,
    sourceInput,
    lineError,
    lineEditLabel: (line) => `${line.item.name}，编辑入库明细`,
    addItem,
    removeLine,
    onEditorOpen: () => {},
    onEditorStash: () => {},
    commitEditor,
    reviewGate,
    performSubmit,
    clearDraft,
    onCreateItemRequest: () => {
      itemCreateOpen.value = true;
    },
    onScanItemAdded: applyScanToLine,
    onScanItemMissing: (bagCode) => {
      // 库中没有该 C 号：转入快速新建，自动以 C 号拉立创资料预填；创建完成后回明细并应用 qty/来源。
      pendingScanBagCode = bagCode;
      scanLcscCode.value = bagCode.productCode;
      itemCreateOpen.value = true;
      return true;
    },
  };

  function handleItemCreateClosed(): void {
    itemCreateOpen.value = false;
    scanLcscCode.value = "";
    pendingScanBagCode = null;
  }

  return {
    flow,
    canCreateItem,
    locations,
    locationError,
    loadLocationOptions,
    itemCreateOpen,
    handleItemCreated,
    handleItemCreateClosed,
    orderImportOpen,
    importOrderLines,
    scanOrderPrompt,
    applyScanOrderNo,
    ignoreScanOrderNo,
    suppressScanOrderPrompt,
    scanLcscCode,
    draftTotal,
    quantitySummary,
    draftAmountReady,
  };
}
