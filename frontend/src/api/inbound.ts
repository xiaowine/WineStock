// 本文件拥有入库编辑所需的库位和模板 DTO 查询，属于 frontend API 边界；它不维护页面草稿或审批状态。
import { apiClient } from './client'
export type { InboundTemplateResponse } from './inboundTemplates'
export type { TemplateFieldResponse, TemplateFieldType } from './templateFields'

/** 可选入库库位。 */
export interface LocationResponse {
  /** 库位 ID。 */
  id: number
  /** 所属库位分组 ID。 */
  group_id: number
  /** 所属库位分组名称。 */
  group_name: string
  /** 库位编码。 */
  code: string
  /** 库位名称。 */
  name: string
  /** 同组排序值。 */
  sort_order: number
  /** 创建时间。 */
  created_at: string
  /** 最近更新时间。 */
  updated_at: string
}

/** 创建入库单的单条物品明细。 */
export interface InboundItemRequest {
  /** 已存在的物品 ID。 */
  item_id: number
  /** 入库数量，必须大于 0。 */
  quantity: number
  /** 入库单价，不允许为负。 */
  unit_price: number
  /** 明细存放库位 ID。 */
  location_id: number
  /** 可选外部批次号。 */
  batch_no?: string
  /** 可选有效期文本。 */
  expires_at?: string
  /** 本明细使用的入库模板。 */
  inbound_template_id?: number
  /** 物品模板扩展属性。 */
  ext_attributes?: Record<string, string | number | boolean | FileAttributeReference>
}

/** 模板 file 字段保存的稳定服务端引用。 */
export interface FileAttributeReference {
  /** 已上传文件对象 ID。 */
  file_id: number
}

/** 创建 pending 入库单请求。 */
export interface InboundCreateRequest {
  /** 供应商名称或采购单号等入库来源。 */
  source: string
  /** 可选入库备注。 */
  notes?: string
  /** 入库物品明细，至少一条。 */
  items: InboundItemRequest[]
}

/** 创建成功后页面使用的入库单摘要。 */
export interface InboundResponse {
  /** 新入库单 ID。 */
  id: number
  /** 入库来源。 */
  source: string
  /** 新建单据固定为 pending。 */
  status: 'pending' | 'approved' | 'rejected'
  /** 可选备注。 */
  notes: string | null
}

/** 查询全部有效库位。 */
export function listLocations(signal?: AbortSignal) {
  return apiClient.request<LocationResponse[]>('/api/locations', { signal })
}

/** 创建 pending 入库单；库存批次和流水仍由后续审批生成。 */
export function createInbound(request: InboundCreateRequest) {
  return apiClient.request<InboundResponse>('/api/inbound', {
    method: 'POST',
    json: request,
  })
}
