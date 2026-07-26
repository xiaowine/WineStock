// 本文件拥有入库草稿的前端模型、行校验和请求转换规则；它不发起 HTTP 请求或管理页面路由。
import type { InboundCreateRequest, InboundSubmissionMode } from "../../api/inbound";
import type { ItemOptionResponse } from "../../api/items";

/** 独立入库明细；lineId 是同物品多批次的页面身份。 */
export interface InboundDraftLine {
  lineId: string;
  item: ItemOptionResponse;
  quantity: number;
  unitPrice: number;
  locationId: number | null;
  batchNo: string;
  expiresAt: string;
}

/** 从真实物品创建一条互相独立的入库明细。 */
export function createDraftLine(item: ItemOptionResponse): InboundDraftLine {
  return {
    lineId: createLineId(),
    item,
    quantity: 1,
    unitPrice: 0,
    locationId: null,
    batchNo: "",
    expiresAt: "",
  };
}

export function validQuantity(value: number): boolean {
  return Number.isFinite(value) && value > 0;
}
export function validUnitPrice(value: number): boolean {
  return Number.isFinite(value) && value >= 0;
}
export function positiveNumber(value: number): number {
  return Number.isFinite(value) && value > 0 ? value : 0;
}
export function lineSubtotal(line: InboundDraftLine): number {
  return positiveNumber(line.quantity) * (validUnitPrice(line.unitPrice) ? line.unitPrice : 0);
}

export function lineReady(line: InboundDraftLine): boolean {
  return validQuantity(line.quantity) && validUnitPrice(line.unitPrice) && line.locationId !== null;
}

/** 把页面草稿转换为稳定入库创建契约。 */
export function buildInboundRequest(
  source: string,
  notes: string,
  lines: InboundDraftLine[],
  submissionMode: InboundSubmissionMode,
): InboundCreateRequest {
  return {
    submission_mode: submissionMode,
    source: source.trim(),
    notes: notes.trim() || undefined,
    items: lines.map((line) => ({
      item_id: line.item.id,
      quantity: line.quantity,
      unit_price: line.unitPrice,
      location_id: line.locationId as number,
      batch_no: line.batchNo.trim() || undefined,
      expires_at: line.expiresAt || undefined,
    })),
  };
}

export function createLineId(): string {
  return crypto.randomUUID?.() ?? `line-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}
