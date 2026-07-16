// 本文件拥有入库编辑所需的模板 DTO 和创建请求，并复用统一库位 API；它不维护页面草稿或审批状态。
import { apiClient } from "./client";
export { listLocations } from "./locations";
export type { LocationResponse } from "./locations";
export type { InboundTemplateResponse } from "./inboundTemplates";
export type { TemplateFieldResponse, TemplateFieldType } from "./templateFields";

/** 创建入库单的单条物品明细。 */
export interface InboundItemRequest {
  /** 已存在的物品 ID。 */
  item_id: number;
  /** 入库数量，必须大于 0。 */
  quantity: number;
  /** 入库单价，不允许为负。 */
  unit_price: number;
  /** 明细存放库位 ID。 */
  location_id: number;
  /** 可选外部批次号。 */
  batch_no?: string;
  /** 可选有效期文本。 */
  expires_at?: string;
  /** 本明细使用的入库模板。 */
  inbound_template_id?: number;
  /** 物品模板扩展属性。 */
  ext_attributes?: Record<string, string | number | boolean | FileAttributeReference>;
}

/** 模板 file 字段保存的稳定服务端引用。 */
export interface FileAttributeReference {
  /** 已上传文件对象 ID。 */
  file_id: number;
}

/** 创建入库单时采用的服务端处理方式。 */
export type InboundSubmissionMode = "pending_approval" | "direct";

/** 创建待审批单据或直接完成入库的请求。 */
export interface InboundCreateRequest {
  /** 本次提交进入待审批状态，或在具备审核权限时直接完成入库。 */
  submission_mode: InboundSubmissionMode;
  /** 供应商名称或采购单号等入库来源。 */
  source: string;
  /** 可选入库备注。 */
  notes?: string;
  /** 入库物品明细，至少一条。 */
  items: InboundItemRequest[];
}

/** 创建成功后页面使用的入库单摘要。 */
export interface InboundResponse {
  /** 新入库单 ID。 */
  id: number;
  /** 入库来源。 */
  source: string;
  /** 创建后的真实单据状态。 */
  status: "pending" | "approved" | "rejected";
  /** 本次创建实际采用的处理方式。 */
  submission_mode: InboundSubmissionMode;
  /** 可选备注。 */
  notes: string | null;
}

/** 创建待审批单据或直接完成入库；响应明确返回实际采用的提交方式。 */
export function createInbound(request: InboundCreateRequest) {
  return apiClient.request<InboundResponse>("/api/inbound", {
    method: "POST",
    json: request,
  });
}
