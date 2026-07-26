// 本文件拥有合并草稿页的出库域装配：分配草稿、批次分页、成本估算与提交编排。
// 它复用旧出库页的 model 与持久化（含同一 localStorage 键），不修改旧页面任何文件。
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";
import { useRouter } from "vue-router";
import {
  listItemBatches,
  type ItemBatchStockResponse,
  type ItemOptionResponse,
} from "../../api/items";
import { listLocations, type LocationResponse } from "../../api/locations";
import { createOutbound } from "../../api/outbound";
import { approveOutboundOrder } from "../../api/stockApprovals";
import { ApiError } from "../../api/errors";
import { hasPermission, stockPermissions } from "../../auth/permissions";
import { authSession } from "../../auth/session";
import { useOutboundDraftPersistence } from "../../composables/useOutboundDraftPersistence";
import { useStablePendingIndicator } from "../../composables/useStablePendingIndicator";
import { notice } from "../../notices/notice";
import { trackTelemetryEvent, trackTelemetryIssue } from "../../telemetry/clarity";
import {
  buildOutboundRequest,
  createOutboundDraftLine,
  emptyCostEstimate,
  estimateFifoCost,
  estimateSpecificBatchCost,
  lineError as outboundLineError,
  type OutboundAllocationMode,
  type OutboundCostEstimate,
  type OutboundDraftLine,
} from "../outbound-draft/model";
import { formatMoney } from "../inbound-draft/presentation";
import type { StockDraftFlow, StockDraftTexts, StockDraftWorkspaceHandle } from "./flow";

export type { OutboundDraftLine } from "../outbound-draft/model";

/** 行编辑 Dialog 中的分配草稿；写回时机由“暂存/完成”动作决定。 */
export interface OutboundAllocationDraft {
  quantity: string;
  mode: OutboundAllocationMode;
  batchId: number | null;
  locationId: number | null;
}

interface CostBatchSnapshot {
  state: "loading" | "complete" | "failed";
  batches: ItemBatchStockResponse[];
}

/** 出库域的工作台文案与列配置。 */
export const outboundDraftTexts: StockDraftTexts = {
  rootClass: "stock-draft-page--outbound",
  summaryAriaLabel: "当前出库草稿摘要",
  workspaceTitle: "出库单信息与明细",
  metaAriaLabel: "出库单基础信息",
  sourceLabel: "出库去向",
  sourceName: "outbound_destination",
  sourcePlaceholder: "客户 / 部门 / 项目",
  notesName: "outbound_notes",
  notesPlaceholder: "可选，记录出库说明",
  linesAriaLabel: "出库明细复核",
  emptyTitle: "还没有出库明细",
  emptyHint: "点击“添加物品”选择一项，完成对应出库明细。",
  columns: ["数量", "分配 / 批次", "库位", "预计成本"],
  editorTitle: "配置出库明细",
  editorDescription: "数量和扣减方式属于同一条明细；完成后才能继续添加下一项。",
  editorWide: true,
  pickerTitle: "选择出库物品",
  pickerSearchName: "outbound_item_search",
  clearTitle: "清空出库草稿？",
  clearDescription: "所有未提交内容将从本机删除。",
  leaveBody: "确认离开当前出库流程吗？",
  submitTitleDirect: "确认直接出库？",
  submitTitlePending: "确认提交审核？",
  submitDescriptionDirect: "确认后会立即扣减库存并写入出库流水。",
  submitDescriptionPending: "提交后进入待审批，库存不会立即扣减。",
  submitButtonDirect: "直接出库",
  submitButtonPending: "提交审核",
  submitConfirmDirect: "确认直接出库",
  submitConfirmPending: "确认提交审核",
};

/** 组装出库域草稿流；handle 由工作台挂载后回填。 */
export function useOutboundDraft(handle: StockDraftWorkspaceHandle) {
  const router = useRouter();
  const source = ref("");
  const notes = ref("");
  const notesOpen = ref(false);
  const lines = ref<OutboundDraftLine[]>([]);
  const validationAttempted = ref(false);
  const submitting = ref(false);
  const sourceInput = ref<HTMLInputElement | null>(null);
  const locations = ref<LocationResponse[]>([]);
  const locationError = ref("");
  const editorLine = ref<OutboundDraftLine | null>(null);
  const allocationDraft = reactive<OutboundAllocationDraft>({
    quantity: "",
    mode: "fifo",
    batchId: null,
    locationId: null,
  });
  const batches = ref<ItemBatchStockResponse[]>([]);
  const batchLoading = ref(false);
  const batchError = ref("");
  const batchPage = ref(0);
  const batchPages = ref(0);
  const costBatchSnapshots = ref<Record<number, CostBatchSnapshot>>({});
  let batchController: AbortController | null = null;
  const costBatchControllers = new Map<number, AbortController>();

  // 与旧出库页保持一致的入口拦截口径；路由已要求出库创建权限。
  const canReadItems = computed(() =>
    hasPermission(authSession.value?.user.permissions, stockPermissions.outboundCreate),
  );
  const hasDraft = computed(
    () => lines.value.length > 0 || !!source.value.trim() || !!notes.value.trim(),
  );
  const canDirect = computed(() =>
    hasPermission(authSession.value?.user.permissions, stockPermissions.outboundApprove),
  );
  const quantitySummary = computed(
    () =>
      Array.from(
        lines.value.reduce((map, line) => {
          const quantity = Number(line.quantity);
          if (quantity > 0) map.set(line.item.unit, (map.get(line.item.unit) || 0) + quantity);
          return map;
        }, new Map<string, number>()),
      )
        .map(([unit, quantity]) => `${quantity} ${unit}`)
        .join("、") || "未填写数量",
  );
  const batchMore = computed(() => batchPage.value < batchPages.value);
  const batchPending = useStablePendingIndicator(batchLoading, {
    showDelayMs: 200,
    minimumVisibleMs: 350,
  });
  const costSummary = computed(() => {
    const estimates = lines.value
      .filter((line) => Number(line.quantity) > 0)
      .map((line) => lineCostEstimate(line));
    if (!estimates.length) return emptyCostEstimate("idle");
    if (estimates.every((estimate) => estimate.state === "complete"))
      return {
        state: "complete" as const,
        amount: estimates.reduce((total, estimate) => total + (estimate.amount ?? 0), 0),
      };
    if (estimates.some((estimate) => estimate.state === "loading" || estimate.state === "idle"))
      return { state: "loading" as const, amount: null };
    if (estimates.some((estimate) => estimate.state === "insufficient"))
      return { state: "insufficient" as const, amount: null };
    return { state: "failed" as const, amount: null };
  });
  const allocationCostEstimate = computed(() => {
    const line = editorLine.value;
    if (!line) return emptyCostEstimate("idle");
    return lineCostEstimate({
      ...line,
      quantity: allocationDraft.quantity,
      allocationMode: allocationDraft.mode,
      batchId: allocationDraft.batchId,
      locationId: allocationDraft.locationId,
    });
  });
  const allocationCostHint = computed(() => costEstimateDetail(allocationCostEstimate.value));
  const confirmCostLabel = computed(() =>
    costSummary.value.state === "complete"
      ? `¥${formatMoney(costSummary.value.amount ?? 0)}`
      : "实际出库时按扣减批次确认",
  );

  const { restoreDraft, resumeDraftSaving, removePersistedDraft } = useOutboundDraftPersistence(
    source,
    notes,
    notesOpen,
    lines,
    hasDraft,
  );

  onMounted(async () => {
    if (restoreDraft()) notice.info("已恢复上次未提交的出库草稿");
    resumeDraftSaving();
    requestCostEstimates();
    if (hasPermission(authSession.value?.user.permissions, stockPermissions.locationRead)) {
      try {
        locations.value = await listLocations({});
      } catch {
        locationError.value = "无法加载库位";
      }
    }
  });

  onBeforeUnmount(() => {
    batchController?.abort();
    costBatchControllers.forEach((controller) => controller.abort());
  });

  watch(
    () => allocationDraft.mode,
    (mode) => {
      if (mode === "specific_batch" && editorLine.value && !batches.value.length)
        void resetBatches();
    },
  );
  watch(lines, requestCostEstimates, { deep: true });

  function addItem(item: ItemOptionResponse): OutboundDraftLine {
    const existing = lines.value.find((line) => line.item.id === item.id);
    if (existing) return existing;
    const line = createOutboundDraftLine(item);
    lines.value.push(line);
    return line;
  }

  function removeLine(lineId: string): void {
    const line = lines.value.find((candidate) => candidate.lineId === lineId);
    if (editorLine.value?.lineId === lineId) {
      batchController?.abort();
      editorLine.value = null;
    }
    lines.value = lines.value.filter((candidate) => candidate.lineId !== lineId);
    if (line && !lines.value.some((candidate) => candidate.item.id === line.item.id)) {
      costBatchControllers.get(line.item.id)?.abort();
      costBatchControllers.delete(line.item.id);
      const remaining = { ...costBatchSnapshots.value };
      delete remaining[line.item.id];
      costBatchSnapshots.value = remaining;
    }
  }

  function onEditorOpen(line: OutboundDraftLine): void {
    void loadCostBatches(line.item.id);
    editorLine.value = line;
    allocationDraft.quantity = line.quantity;
    allocationDraft.mode = line.allocationMode;
    allocationDraft.batchId = line.batchId;
    allocationDraft.locationId = line.locationId;
    batches.value = [];
    batchPage.value = 0;
    batchPages.value = 0;
    batchError.value = "";
    if (line.allocationMode === "specific_batch") void resetBatches();
  }

  function writeAllocationDraft(line: OutboundDraftLine): void {
    line.quantity = allocationDraft.quantity;
    line.allocationMode = allocationDraft.mode;
    if (allocationDraft.mode === "specific_batch") {
      line.batchId = allocationDraft.batchId;
      line.locationId =
        findBatch({ ...line, batchId: allocationDraft.batchId })?.location_id ??
        allocationDraft.locationId;
    } else {
      line.batchId = null;
      line.locationId = allocationDraft.locationId;
    }
  }

  function onEditorStash(line: OutboundDraftLine): void {
    writeAllocationDraft(line);
    batchController?.abort();
    editorLine.value = null;
  }

  function commitEditor(line: OutboundDraftLine): boolean {
    const candidate = {
      ...line,
      quantity: allocationDraft.quantity,
      allocationMode: allocationDraft.mode,
      batchId: allocationDraft.batchId,
      locationId: allocationDraft.locationId,
    };
    validationAttempted.value = true;
    const blocking = outboundLineError(candidate);
    if (blocking) {
      notice.warning("当前出库明细尚未完成", { detail: blocking });
      void nextTick(() =>
        document.querySelector<HTMLElement>("[data-outbound-allocation-quantity]")?.focus(),
      );
      return false;
    }
    writeAllocationDraft(line);
    batchController?.abort();
    editorLine.value = null;
    return true;
  }

  function reviewGate(): boolean {
    validationAttempted.value = true;
    if (!source.value.trim()) {
      notice.warning("请填写出库去向");
      sourceInput.value?.focus();
      return false;
    }
    if (!lines.value.length) {
      notice.warning("请至少添加一条出库明细");
      handle.openItemPicker();
      return false;
    }
    const bad = lines.value.find((line) => outboundLineError(line));
    if (bad) {
      notice.warning("出库明细尚未填写完整", { detail: `请检查“${bad.item.name}”。` });
      handle.openLineEditor(bad.lineId);
      return false;
    }
    return true;
  }

  async function performSubmit(): Promise<"close" | "keep"> {
    submitting.value = true;
    try {
      const order = await createOutbound(
        buildOutboundRequest(source.value, notes.value, lines.value),
      );
      if (canDirect.value) await approveOutboundOrder(order.id);
      notice.success(canDirect.value ? "出库成功" : "出库单已提交", {
        detail: canDirect.value
          ? `单号 #${order.id} 已完成出库，库存已扣减。`
          : `单号 #${order.id} 已进入待审批状态，库存未扣减。`,
        onClick: hasPermission(authSession.value?.user.permissions, stockPermissions.outboundRead)
          ? () => router.push({ name: "outbound-orders" })
          : undefined,
      });
      trackTelemetryEvent("outbound_submitted");
      clearDraft();
      return "close";
    } catch (error) {
      // 前端校验全过仍被拒（含直接出库的审批步骤失败），属于要抓的排查场景。
      trackTelemetryIssue("outbound_submit_failed");
      notice.error(canDirect.value ? "直接出库失败" : "提交出库单失败", {
        detail: error instanceof ApiError ? error.message : "请检查网络后重试",
      });
      return "keep";
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
    removePersistedDraft();
  }

  function requestCostEstimates(): void {
    for (const line of lines.value)
      if (Number(line.quantity) > 0) void loadCostBatches(line.item.id);
  }

  function costSnapshot(itemId: number): CostBatchSnapshot | undefined {
    return costBatchSnapshots.value[itemId];
  }

  function lineCostEstimate(line: OutboundDraftLine): OutboundCostEstimate {
    if (!validQuantity(line.quantity)) return emptyCostEstimate("idle");
    const snapshot = costSnapshot(line.item.id);
    if (!snapshot || snapshot.state === "loading") return emptyCostEstimate("loading");
    if (snapshot.state === "failed") return emptyCostEstimate("failed");
    return line.allocationMode === "specific_batch"
      ? estimateSpecificBatchCost(line, findBatch(line))
      : estimateFifoCost(line, snapshot.batches);
  }

  function costEstimatePrimary(line: OutboundDraftLine): string {
    const estimate = lineCostEstimate(line);
    if (estimate.state === "complete") return `¥${formatMoney(estimate.amount ?? 0)}`;
    if (estimate.state === "insufficient") return "库存可能不足";
    if (estimate.state === "failed") return "暂无法估算成本";
    return estimate.state === "loading" ? "正在估算成本…" : "填写数量后估算成本";
  }

  function costEstimateSecondary(line: OutboundDraftLine): string | null {
    const estimate = lineCostEstimate(line);
    if (estimate.state === "complete" && estimate.allocationCount > 1)
      return `${estimate.allocationCount} 个批次`;
    if (estimate.state === "insufficient" || estimate.state === "failed") return "实际出库时确认";
    return null;
  }

  function costEstimateDetail(estimate: OutboundCostEstimate): string {
    if (estimate.state === "complete")
      return `预计出库成本 ¥${formatMoney(estimate.amount ?? 0)}${estimate.allocationCount > 1 ? `，按 ${estimate.allocationCount} 个批次分摊。` : "。"} 实际出库时会按实际库存重新校验。`;
    if (estimate.state === "insufficient")
      return "当前库存快照无法完整覆盖该数量，成本将在实际出库时按实际扣减批次确认。";
    if (estimate.state === "failed")
      return "暂无法读取批次成本；不影响提交，实际出库时会按实际库存处理。";
    if (estimate.state === "loading") return "正在按当前批次余额估算成本…";
    return "填写数量后，将按当前批次余额估算成本。";
  }

  async function loadCostBatches(itemId: number): Promise<void> {
    const existing = costSnapshot(itemId);
    if (existing?.state === "loading" || existing?.state === "complete") return;
    const controller = new AbortController();
    costBatchControllers.get(itemId)?.abort();
    costBatchControllers.set(itemId, controller);
    let page = 1;
    let totalPages = 1;
    let snapshotBatches: ItemBatchStockResponse[] = [];
    costBatchSnapshots.value = {
      ...costBatchSnapshots.value,
      [itemId]: { state: "loading", batches: snapshotBatches },
    };
    try {
      while (page <= totalPages) {
        const response = await listItemBatches(itemId, page, 50, controller.signal);
        if (costBatchControllers.get(itemId) !== controller) return;
        snapshotBatches = [
          ...snapshotBatches,
          ...response.items.filter(
            (batch) => !snapshotBatches.some((candidate) => candidate.id === batch.id),
          ),
        ];
        page = response.page + 1;
        totalPages = response.total_pages;
        costBatchSnapshots.value = {
          ...costBatchSnapshots.value,
          [itemId]: { state: "loading", batches: snapshotBatches },
        };
      }
      costBatchSnapshots.value = {
        ...costBatchSnapshots.value,
        [itemId]: { state: "complete", batches: snapshotBatches },
      };
    } catch (error) {
      if (error instanceof DOMException && error.name === "AbortError") return;
      if (costBatchControllers.get(itemId) === controller)
        costBatchSnapshots.value = {
          ...costBatchSnapshots.value,
          [itemId]: { state: "failed", batches: snapshotBatches },
        };
    } finally {
      if (costBatchControllers.get(itemId) === controller) costBatchControllers.delete(itemId);
    }
  }

  function findBatch(line: OutboundDraftLine): ItemBatchStockResponse | undefined {
    if (line.batchId === null) return undefined;
    return (
      batches.value.find((batch) => batch.id === line.batchId) ??
      costSnapshot(line.item.id)?.batches.find((batch) => batch.id === line.batchId)
    );
  }

  function allocationSummary(line: OutboundDraftLine): string {
    if (line.allocationMode === "fifo")
      return line.locationId
        ? `按 FIFO 分配 · 库位 #${line.locationId}`
        : "按 FIFO 分配 · 全部库位";
    const batch = findBatch(line);
    return batch
      ? `批次 ${batch.batch_no} · ${batch.location_name}`
      : line.batchId
        ? `指定批次 #${line.batchId}`
        : "尚未选择批次";
  }

  function allocationPrimary(line: OutboundDraftLine): string {
    return line.allocationMode === "fifo" ? "FIFO 分配" : "指定批次";
  }

  function allocationSecondary(line: OutboundDraftLine): string {
    if (line.allocationMode === "fifo") return "";
    if (batchUnavailable(line)) return "批次已失效";
    const batch = findBatch(line);
    return batch ? batch.batch_no : line.batchId ? `批次 #${line.batchId}` : "待选择批次";
  }

  function allocationLocationLabel(line: OutboundDraftLine): string {
    const batch = findBatch(line);
    if (batch) return batch.location_name;
    if (line.locationId === null) return line.allocationMode === "fifo" ? "全部库位" : "随批次确定";
    return (
      locations.value.find((candidate) => candidate.id === line.locationId)?.name ??
      `库位 #${line.locationId}`
    );
  }

  function batchUnavailable(line: OutboundDraftLine): boolean {
    return (
      line.allocationMode === "specific_batch" &&
      line.batchId !== null &&
      costSnapshot(line.item.id)?.state === "complete" &&
      findBatch(line) === undefined
    );
  }

  function quantityLabel(line: OutboundDraftLine): string {
    return validQuantity(line.quantity) ? `${line.quantity} ${line.item.unit}` : "待填写";
  }

  async function resetBatches(): Promise<void> {
    if (editorLine.value && costSnapshot(editorLine.value.item.id)?.state === "failed")
      void retryCostBatches(editorLine.value.item.id);
    batches.value = [];
    batchPage.value = 0;
    batchPages.value = 0;
    await loadBatches();
  }

  async function retryCostBatches(itemId: number): Promise<void> {
    const remaining = { ...costBatchSnapshots.value };
    delete remaining[itemId];
    costBatchSnapshots.value = remaining;
    await loadCostBatches(itemId);
  }

  async function loadBatches(): Promise<void> {
    if (!editorLine.value || batchLoading.value || (!batchMore.value && batchPage.value > 0))
      return;
    batchController?.abort();
    const controller = new AbortController();
    batchController = controller;
    batchLoading.value = true;
    batchError.value = "";
    try {
      const response = await listItemBatches(
        editorLine.value.item.id,
        batchPage.value + 1,
        20,
        controller.signal,
      );
      if (batchController !== controller) return;
      batches.value = [
        ...batches.value,
        ...response.items.filter(
          (batch) => !batches.value.some((candidate) => candidate.id === batch.id),
        ),
      ];
      batchPage.value = response.page;
      batchPages.value = response.total_pages;
    } catch (error) {
      if (!(error instanceof DOMException && error.name === "AbortError"))
        batchError.value =
          error instanceof ApiError && error.status === 403 ? "无权读取库存批次" : "加载批次失败";
    } finally {
      if (batchController === controller) {
        batchController = null;
        batchLoading.value = false;
      }
    }
  }

  function validQuantity(value: string): boolean {
    const number = Number(value);
    return Number.isFinite(number) && number > 0;
  }

  const flow: StockDraftFlow<OutboundDraftLine> = {
    source,
    notes,
    notesOpen,
    lines,
    validationAttempted,
    submitting,
    hasDraft,
    canDirect,
    sourceInput,
    lineError: outboundLineError,
    lineEditLabel: (line) => `${line.item.name}：${allocationSummary(line)}，打开出库明细编辑器`,
    addItem,
    removeLine,
    onEditorOpen,
    onEditorStash,
    commitEditor,
    reviewGate,
    performSubmit,
    clearDraft,
  };

  return {
    flow,
    canReadItems,
    locations,
    locationError,
    allocationDraft,
    batches,
    batchError,
    batchPending,
    batchMore,
    resetBatches,
    loadBatches,
    costSummary,
    confirmCostLabel,
    allocationCostHint,
    quantitySummary,
    quantityLabel,
    validQuantity,
    allocationPrimary,
    allocationSecondary,
    allocationLocationLabel,
    batchUnavailable,
    lineCostEstimate,
    costEstimatePrimary,
    costEstimateSecondary,
  };
}
