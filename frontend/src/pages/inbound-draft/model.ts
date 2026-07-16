// 本文件拥有入库草稿的前端模型、模板字段校验和请求转换规则；它不发起 HTTP 请求或管理页面路由。
import type {
  FileAttributeReference,
  InboundCreateRequest,
  InboundSubmissionMode,
} from "../../api/inbound";
import type { InboundTemplateResponse } from "../../api/inboundTemplates";
import type { TemplateFieldResponse } from "../../api/templateFields";
import type { ItemOptionResponse } from "../../api/items";
import {
  isImageDraftValue,
  releaseImageDraft,
  type ImageDraftValue,
} from "../../components/attributes/imageDraft";

export type FileDraftValue = ImageDraftValue;
export type InboundTemplateSource = "none" | "recommended" | "manual";
export type InboundTemplateState = "idle" | "resolving" | "ready" | "unresolved" | "error";

/** 单个模板字段的草稿值。 */
export type AttributeValue = string | number | boolean | FileDraftValue | undefined;

/** 独立入库明细；lineId 是同物品多批次的页面身份。 */
export interface InboundDraftLine {
  lineId: string;
  item: ItemOptionResponse;
  quantity: number;
  unitPrice: number;
  locationId: number | null;
  batchNo: string;
  expiresAt: string;
  extAttributes: Record<string, AttributeValue>;
  template: InboundTemplateResponse | null;
  templateId: number | null;
  recommendedTemplateId: number | null;
  templateSource: InboundTemplateSource;
  templateState: InboundTemplateState;
  templateError: string;
}

/** 从真实物品创建一条互相独立的入库明细。 */
export function createDraftLine(item: ItemOptionResponse): InboundDraftLine {
  const recommendedTemplateId = item.recommended_inbound_template_id;
  const recommendedAvailable = item.recommended_inbound_template_available;
  return {
    lineId: createLineId(),
    item,
    quantity: 1,
    unitPrice: 0,
    locationId: null,
    batchNo: "",
    expiresAt: "",
    extAttributes: {},
    template: null,
    templateId: recommendedTemplateId,
    recommendedTemplateId,
    templateSource: recommendedTemplateId === null ? "none" : "recommended",
    templateState:
      recommendedTemplateId === null ? "idle" : recommendedAvailable ? "resolving" : "unresolved",
    templateError:
      recommendedTemplateId !== null && !recommendedAvailable
        ? "推荐入库模板已删除，请重新选择"
        : "",
  };
}

export function fileValue(line: InboundDraftLine, fieldName: string): FileDraftValue | undefined {
  const value = line.extAttributes[fieldName];
  return typeof value === "object" && value?.kind === "file" ? value : undefined;
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

/** 按服务端当前模板规则校验单个字段，并返回可直接展示的中文原因。 */
export function templateFieldError(
  line: InboundDraftLine,
  field: TemplateFieldResponse,
): string | null {
  const value = line.extAttributes[field.field_name];
  const empty = value === undefined || value === null || value === "";
  if (empty) return field.required ? "此项为必填属性" : null;
  if (field.field_type === "number")
    return Number.isFinite(Number(value)) ? null : "请输入有效数字";
  if (field.field_type === "url")
    return validHttpUrl(String(value)) ? null : "请输入 HTTP 或 HTTPS 地址";
  if (field.field_type === "select")
    return field.options?.includes(String(value)) ? null : "请选择有效候选值";
  if (field.field_type === "boolean") return typeof value === "boolean" ? null : "请选择是或否";
  if (field.field_type === "date") return validIsoDate(String(value)) ? null : "请输入有效日期";
  if (field.field_type === "file") {
    const file = fileValue(line, field.field_name);
    if (!file) return "请选择图片";
    if (file.status === "uploading") return "请等待图片上传完成";
    if (file.status === "failed" && !file.localFile) return file.error || "请重新选择图片";
  }
  return typeof value === "string" && !value.trim() ? "请输入有效文本" : null;
}

export function lineReady(line: InboundDraftLine): boolean {
  return (
    validQuantity(line.quantity) &&
    validUnitPrice(line.unitPrice) &&
    line.locationId !== null &&
    !["resolving", "unresolved", "error"].includes(line.templateState) &&
    (line.template?.fields.every((field) => templateFieldError(line, field) === null) ?? true)
  );
}

export function incompleteTemplateFieldCount(line: InboundDraftLine): number {
  return (
    line.template?.fields.filter((field) => templateFieldError(line, field) !== null).length ?? 0
  );
}

export function hasTemplateDraftValues(line: InboundDraftLine): boolean {
  return Object.values(line.extAttributes).some((value) => {
    if (value === undefined || value === null || value === "") return false;
    return typeof value !== "string" || value.trim().length > 0;
  });
}

/** 把页面草稿转换为稳定入库创建契约，file 字段只发送 file_id。 */
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
      inbound_template_id: line.templateId ?? undefined,
      ext_attributes: normalizedAttributes(line),
    })),
  };
}

export function revokeLinePreviews(line: InboundDraftLine): void {
  Object.values(line.extAttributes).forEach((value) => {
    if (isImageDraftValue(value)) releaseImageDraft(value);
  });
}

export function createLineId(): string {
  return crypto.randomUUID?.() ?? `line-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function normalizedAttributes(
  line: InboundDraftLine,
): Record<string, string | number | boolean | FileAttributeReference> | undefined {
  if (!line.template) return undefined;
  const attributes: Record<string, string | number | boolean | FileAttributeReference> = {};
  for (const field of line.template.fields) {
    const value = line.extAttributes[field.field_name];
    if (value === undefined || value === "") continue;
    if (field.field_type === "file") {
      const file = fileValue(line, field.field_name);
      if (file?.fileId) attributes[field.field_name] = { file_id: file.fileId };
    } else
      attributes[field.field_name] =
        field.field_type === "number" ? Number(value) : (value as string | number | boolean);
  }
  return Object.keys(attributes).length ? attributes : undefined;
}

function validHttpUrl(value: string): boolean {
  try {
    const url = new URL(value);
    return (url.protocol === "http:" || url.protocol === "https:") && !/\s/.test(value);
  } catch {
    return false;
  }
}

function validIsoDate(value: string): boolean {
  if (!/^(\d{4})-(\d{2})-(\d{2})$/.test(value)) return false;
  const date = new Date(`${value}T00:00:00Z`);
  return !Number.isNaN(date.getTime()) && date.toISOString().slice(0, 10) === value;
}
