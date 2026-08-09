// 本文件拥有合并草稿页的入库域装配：行模型接线、库位加载、校验聚焦与提交编排。
// 它复用旧入库页的 model 与持久化（含同一 localStorage 键），不修改旧页面任何文件。
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { createInbound, listLocations, type LocationResponse } from "../../api/inbound";
import { getItemInventory, type ItemOptionResponse } from "../../api/items";
import { ApiError } from "../../api/errors";
import { useInboundDraftPersistence } from "../../composables/useInboundDraftPersistence";
import type { LcscBagCode } from "../../lcsc/bagCode";
import type { LcscOrderImportPayload } from "../../components/stock-draft/LcscOrderImportDialog.vue";
import type { ErpBackupImportPayload } from "../../components/stock-draft/ErpBackupImportDialog.vue";
import { notice } from "../../notices/notice";
import { trackTelemetryEvent, trackTelemetryIssue } from "../../telemetry/clarity";
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

/** 入库域的工作台文案与列配置；值均为消息键，由工作台壳翻译后渲染。 */
export const inboundDraftTexts: StockDraftTexts = {
  rootClass: "stock-draft-page--inbound",
  summaryAriaLabel: "stockDraft.inboundSummaryAriaLabel",
  workspaceTitle: "stockDraft.inboundWorkspaceTitle",
  metaAriaLabel: "stockDraft.inboundMetaAriaLabel",
  sourceLabel: "stockDraft.sourceLabel",
  sourceName: "inbound_source",
  sourcePlaceholder: "stockDraft.inboundSourcePlaceholder",
  notesName: "inbound_notes",
  notesPlaceholder: "stockDraft.inboundNotesPlaceholder",
  linesAriaLabel: "stockDraft.inboundLinesAriaLabel",
  emptyTitle: "stockDraft.inboundEmptyTitle",
  emptyHint: "stockDraft.inboundEmptyHint",
  columns: [
    "common.amount",
    "stockDraft.unitPriceSubtotal",
    "stockDraft.location",
    "stockDraft.batch",
  ],
  editorTitle: "stockDraft.inboundEditorTitle",
  editorDescription: "stockDraft.inboundEditorDescription",
  editorWide: false,
  pickerTitle: "stockDraft.inboundPickerTitle",
  pickerSearchName: "inbound_item_search",
  clearTitle: "stockDraft.clearDraftTitle",
  clearDescription: "stockDraft.clearDraftDescription",
  leaveBody: "stockDraft.leaveInboundBody",
  submitTitleDirect: "stockDraft.submitInboundDirectTitle",
  submitTitlePending: "stockDraft.submitPendingTitle",
  submitDescriptionDirect: "stockDraft.submitInboundDirectDescription",
  submitDescriptionPending: "stockDraft.submitPendingDescription",
  submitButtonDirect: "stockDraft.submitInboundDirectButton",
  submitButtonPending: "stockDraft.submitPendingButton",
  submitConfirmDirect: "stockDraft.confirmInboundDirect",
  submitConfirmPending: "stockDraft.confirmPending",
};

/** 组装入库域草稿流；handle 由工作台挂载后回填。 */
export function useInboundDraft(handle: StockDraftWorkspaceHandle) {
  const { t } = useI18n();
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
  const backupImportOpen = ref(false);
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
    return lines.value.length ? t("stockDraft.perLineSeparate") : "0";
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
      notice.info(t("stockDraft.restoredInboundDraft"));
    }
    if (removedDuplicates > 0) notice.info(t("stockDraft.removedDuplicates", { n: removedDuplicates }));
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
      if (!isAbortError(error))
        locationError.value = itemErrorMessage(error, "stockDraft.loadLocationsFailed");
    } finally {
      if (locationAbortController === controller) locationAbortController = null;
    }
  }

  function lineError(line: InboundDraftLine): string | null {
    return lineReady(line) ? null : t("stockDraft.lineIncomplete");
  }

  function addItem(item: ItemOptionResponse, options?: { silent?: boolean }): InboundDraftLine {
    const existing = lines.value.find((line) => line.item.id === item.id);
    if (existing) return existing;
    const line = createDraftLine(item);
    prefillLineLocation(line);
    lines.value.push(line);
    if (!options?.silent) notice.info(t("stockDraft.itemAdded", { name: item.name }));
    return line;
  }

  /**
   * 入库库位分层预填：同编号唯一库存库位 → 全局默认库位 → 待选择；
   * 预填等同手选可随时改，查询失败静默回落，不阻塞行创建。
   * 见 docs/implementation-notes/inbound-location-prefill.md。
   */
  function prefillLineLocation(line: InboundDraftLine): void {
    const fallback = locations.value.find((location) => location.is_default)?.id ?? null;
    line.locationId = fallback;
    void applyItemHistoryLocation(line, fallback);
  }

  /** 第一层：仅严格同一物品自己的库存分布，唯一有库存库位时覆盖预填；不做相似推断。 */
  async function applyItemHistoryLocation(
    line: InboundDraftLine,
    prefilled: number | null,
  ): Promise<void> {
    try {
      const inventory = await getItemInventory(line.item.id);
      const stocked = inventory.locations.filter((location) => location.quantity > 0);
      if (stocked.length !== 1) return;
      // 行已被移除或用户已手选其它库位时不再覆盖。
      if (!lines.value.includes(line)) return;
      if (line.locationId !== prefilled && line.locationId !== null) return;
      line.locationId = stocked[0].location_id;
    } catch {
      // 静默回落到已有预填。
    }
  }

  /** 批量设置库位：只填仍为"待选择"的明细，已设库位的行不覆盖。 */
  const batchLocationOpen = ref(false);
  const pendingLocationCount = computed(
    () => lines.value.filter((line) => line.locationId === null).length,
  );
  function applyBatchLocation(locationId: number): void {
    let applied = 0;
    for (const line of lines.value) {
      if (line.locationId === null) {
        line.locationId = locationId;
        applied += 1;
      }
    }
    batchLocationOpen.value = false;
    const name = locations.value.find((location) => location.id === locationId)?.name ?? "";
    if (applied > 0) notice.success(t("stockDraft.locationsSet", { n: applied }), { detail: name });
  }

  function removeLine(lineId: string): void {
    const line = lines.value.find((candidate) => candidate.lineId === lineId);
    if (!line) return;
    lines.value = lines.value.filter((candidate) => candidate.lineId !== lineId);
    notice.info(t("stockDraft.itemRemoved", { name: line.item.name }));
  }

  function commitEditor(line: InboundDraftLine): boolean {
    validationAttempted.value = true;
    if (!lineReady(line)) {
      notice.warning(t("stockDraft.lineIncompleteWarning"), {
        detail: t("stockDraft.lineIncompleteDetail", { name: line.item.name }),
      });
      void focusLineError(line);
      return false;
    }
    return true;
  }

  function reviewGate(): boolean {
    validationAttempted.value = true;
    if (source.value.trim().length > 0 && lines.value.length > 0 && lines.value.every(lineReady))
      return true;
    notice.warning(t("stockDraft.inboundFormIncomplete"), { detail: blockingReason() });
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
        notice.success(t("stockDraft.inboundSubmitSuccess"), {
          detail: t("stockDraft.inboundSubmitSuccessDetail", { id: created.id }),
        });
      } else {
        notice.success(t("stockDraft.inboundSubmitted"), {
          detail: t("stockDraft.inboundSubmittedDetail", { id: created.id }),
        });
      }
      trackTelemetryEvent("inbound_submitted");
      clearDraft();
      return "close";
    } catch (error) {
      // 走到这里说明前端校验全过仍被拒（契约或服务端问题），正是要抓的排查场景。
      trackTelemetryIssue("inbound_submit_failed");
      const message = inboundSubmitErrorMessage(error);
      const errorLine = backendErrorLine(error);
      if (error instanceof ApiError && error.code === "item_not_found" && errorLine) {
        message.title = t("stockDraft.itemInvalidRemove", { name: errorLine.item.name });
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
    if (!source.value.trim()) return t("stockDraft.fillInboundSource");
    if (!lines.value.length) return t("stockDraft.addAtLeastOneLine");
    const invalid = lines.value.find((line) => !lineReady(line));
    return invalid
      ? t("stockDraft.checkLineFields", { name: invalid.item.name })
      : t("stockDraft.checkInboundForm");
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
    notice.success(t("stockDraft.itemCreatedAdded"), { detail: item.name });
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
      // 「立创」为来源前缀的存储数据，与 UI 语言无关，转义保持写入值稳定。
      source.value = `\u7acb\u521b ${payload.orderNo}`;
      askedOrderNos.add(payload.orderNo);
    }
    orderImportOpen.value = false;
    if (added > 0) {
      trackTelemetryEvent("lcsc_order_imported");
      notice.success(t("stockDraft.linesImported", { n: added }), {
        detail: t("stockDraft.linesImportedDetail"),
      });
    } else {
      notice.info(t("stockDraft.noNewLinesToImport"));
    }
  }

  /**
   * ERP 备份导入确认：期初库存行按 库位/数量 写入草稿（单价 0），来源与备注预填。
   * 库位与物品已由 Dialog 落地；同物品多库位在此展开为多条明细，库位已定的行不再走预填。
   */
  async function importBackup(payload: ErpBackupImportPayload): Promise<void> {
    // Dialog 可能刚创建了备份中的库位；先同步选项，避免新 locationId 被误判为已失效。
    await loadLocationOptions();
    let added = 0;
    for (const row of payload.rows) {
      // 同物品可跨多个库位，逐行独立建明细（addItem 的同物品去重不适用，直接建行）。
      const line = createDraftLine(row.item);
      line.quantity = row.quantity;
      line.unitPrice = 0;
      if (row.locationId !== null) line.locationId = row.locationId;
      else prefillLineLocation(line);
      lines.value.push(line);
      added += 1;
    }
    if (!source.value.trim()) source.value = `\u5907\u4efd\u5bfc\u5165 ${payload.fileName}`;
    if (!notes.value.trim()) {
      // 备注内容为随草稿持久化的业务数据，不随 UI 语言变化，转义保持写入值稳定。
      const parts = ["\u7b2c\u4e09\u65b9 ERP \u5907\u4efd\u671f\u521d\u5bfc\u5165"];
      if (payload.appVersion) parts.push(`\u5bfc\u51fa\u7248\u672c ${payload.appVersion}`);
      if (payload.skippedManualCount > 0)
        parts.push(`\u8df3\u8fc7\u624b\u5de5\u5668\u4ef6 ${payload.skippedManualCount} \u9879`);
      notes.value = parts.join("；");
      notesOpen.value = true;
    }
    backupImportOpen.value = false;
    if (added > 0) {
      trackTelemetryEvent("erp_backup_imported");
      notice.success(t("stockDraft.backupLinesImported", { n: added }), {
        detail: t("stockDraft.backupLinesImportedDetail"),
      });
    } else {
      notice.info(t("stockDraft.noImportableStockLines"));
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
    if (!source.value.trim()) source.value = `\u7acb\u521b ${orderNo}`;
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
    lineEditLabel: (line) => t("stockDraft.editInboundLineAria", { name: line.item.name }),
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
    backupImportOpen,
    importBackup,
    batchLocationOpen,
    pendingLocationCount,
    applyBatchLocation,
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
