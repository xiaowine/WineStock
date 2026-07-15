// 本文件拥有出库草稿的本地模型与请求转换；它不发起 HTTP 请求或持久化。
import type { OutboundCreateRequest } from "../../api/outbound";
import type { ItemBatchStockResponse, ItemOptionResponse } from "../../api/items";

/** 出库明细的库存分配方式。 */
export type OutboundAllocationMode = "fifo" | "specific_batch";

/** 单个出库物品的可恢复草稿。 */
export interface OutboundDraftLine {
  lineId: string;
  item: ItemOptionResponse;
  quantity: string;
  allocationMode: OutboundAllocationMode;
  batchId: number | null;
  locationId: number | null;
}

/** 出库草稿中一条明细的成本预估结果；仅代表当前批次快照，不是实际出库后的成本事实。 */
export interface OutboundCostEstimate {
  state: "idle" | "loading" | "complete" | "insufficient" | "failed";
  amount: number | null;
  coveredQuantity: number;
  requestedQuantity: number;
  allocationCount: number;
}

/** 用物品选项建立默认 FIFO 明细。 */
export function createOutboundDraftLine(
  item: ItemOptionResponse,
): OutboundDraftLine {
  return {
    lineId: crypto.randomUUID?.() ?? `outbound-${Date.now()}-${Math.random()}`,
    item,
    quantity: "",
    allocationMode: "fifo",
    batchId: null,
    locationId: null,
  };
}

/** 将界面草稿转换为服务端创建请求。 */
export function buildOutboundRequest(
  destination: string,
  notes: string,
  lines: OutboundDraftLine[],
): OutboundCreateRequest {
  return {
    destination: destination.trim(),
    notes: notes.trim() || undefined,
    items: lines.map((line) => ({
      item_id: line.item.id,
      quantity: Number(line.quantity),
      batch_id:
        line.allocationMode === "specific_batch"
          ? (line.batchId ?? undefined)
          : undefined,
      location_id: line.locationId ?? undefined,
    })),
  };
}

/** 明细阻塞条件仅覆盖创建阶段能确定的必要字段。 */
export function lineError(line: OutboundDraftLine): string | null {
  if (!Number.isFinite(Number(line.quantity)) || Number(line.quantity) <= 0)
    return "请输入大于 0 的数量";
  if (line.allocationMode === "specific_batch" && line.batchId === null)
    return "请选择扣减批次";
  return null;
}

/** 按当前批次快照估算指定批次成本；找不到批次时由调用方提示用户重新选择。 */
export function estimateSpecificBatchCost(
  line: OutboundDraftLine,
  batch: ItemBatchStockResponse | undefined,
): OutboundCostEstimate {
  const quantity = positiveQuantity(line.quantity);
  if (quantity === null) return emptyCostEstimate("idle");
  if (!batch) return { ...emptyCostEstimate("failed"), requestedQuantity: quantity };
  const coveredQuantity = Math.min(quantity, Math.max(0, batch.remaining_quantity));
  return {
    state: coveredQuantity < quantity ? "insufficient" : "complete",
    amount: coveredQuantity * batch.unit_cost,
    coveredQuantity,
    requestedQuantity: quantity,
    allocationCount: 1,
  };
}

/** 按入库时间和批次 ID 的稳定顺序分摊 FIFO 预估成本，不修改调用方传入的批次数组。 */
export function estimateFifoCost(
  line: OutboundDraftLine,
  batches: ItemBatchStockResponse[],
): OutboundCostEstimate {
  const quantity = positiveQuantity(line.quantity);
  if (quantity === null) return emptyCostEstimate("idle");
  let remaining = quantity;
  let amount = 0;
  let allocationCount = 0;
  const candidates = batches
    .filter((batch) => line.locationId === null || batch.location_id === line.locationId)
    .filter((batch) => Number.isFinite(batch.remaining_quantity) && batch.remaining_quantity > 0)
    .slice()
    .sort((left, right) => left.received_at.localeCompare(right.received_at) || left.id - right.id);
  for (const batch of candidates) {
    if (remaining <= 0) break;
    const allocated = Math.min(remaining, batch.remaining_quantity);
    amount += allocated * batch.unit_cost;
    remaining -= allocated;
    allocationCount += 1;
  }
  const coveredQuantity = quantity - remaining;
  return {
    state: remaining > 0 ? "insufficient" : "complete",
    amount,
    coveredQuantity,
    requestedQuantity: quantity,
    allocationCount,
  };
}

/** 供异步批次快照尚未可用和输入未完成时构造一致的空预估。 */
export function emptyCostEstimate(
  state: OutboundCostEstimate["state"],
): OutboundCostEstimate {
  return { state, amount: null, coveredQuantity: 0, requestedQuantity: 0, allocationCount: 0 };
}

function positiveQuantity(value: string): number | null {
  const quantity = Number(value);
  return Number.isFinite(quantity) && quantity > 0 ? quantity : null;
}
