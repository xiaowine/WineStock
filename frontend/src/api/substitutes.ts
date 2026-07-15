// 本文件拥有物品替代关系的 HTTP 契约和请求函数；替代关系的整体替换语义由服务端负责校验。
import { apiClient } from './client'
import type { ItemStockState } from './items'

/** 指定物品的一条替代关系响应。 */
export interface ItemSubstituteResponse {
  /** 主物品 ID。 */
  item_id: number
  /** 替代物品 ID。 */
  substitute_item_id: number
  /** 替代物品名称。 */
  substitute_item_name: string
  /** 替代物品 SKU。 */
  substitute_item_sku: string
  /** 替代物品分类名称。 */
  substitute_item_category_name: string | null
  /** 替代物品主图文件 ID。 */
  substitute_item_image_file_id: number
  /** 替代物品主图受控读取地址。 */
  substitute_item_image_url: string
  /** 替代物品计量单位。 */
  substitute_item_unit: string
  /** 替代物品再订货点。 */
  substitute_item_reorder_point: number | null
  /** 替代物品当前库存量。 */
  quantity: number
  /** 替代物品当前库存状态。 */
  substitute_item_stock_state: ItemStockState
  /** 替代优先级，数值越小越优先。 */
  priority: number
  /** 兼容性备注。 */
  notes: string | null
  /** 关系创建时间。 */
  created_at: string
  /** 创建关系的用户 ID。 */
  created_by_user_id: number | null
}

/** 全局替代关系列表中的单条有向关系。 */
export interface SubstituteRelationResponse {
  /** 主物品 ID。 */
  item_id: number
  /** 主物品名称。 */
  item_name: string
  /** 主物品 SKU。 */
  item_sku: string
  /** 替代物品 ID。 */
  substitute_item_id: number
  /** 替代物品名称。 */
  substitute_item_name: string
  /** 替代物品 SKU。 */
  substitute_item_sku: string
  /** 替代物品当前库存量；全局接口不提供单位和库存状态。 */
  quantity: number
  /** 替代优先级，数值越小越优先。 */
  priority: number
  /** 兼容性备注。 */
  notes: string | null
  /** 关系创建时间。 */
  created_at: string
  /** 创建关系的用户 ID。 */
  created_by_user_id: number | null
}

/** 替代关系整体替换请求中的单条关系。 */
export interface SubstituteReplacementItem {
  /** 替代物品 ID。 */
  substitute_item_id: number
  /** 替代优先级，数值越小越优先。 */
  priority: number
  /** 可选兼容性备注。 */
  notes?: string | null
}

/** 替代关系整体替换请求。 */
export interface SubstituteReplaceRequest {
  /** 提交后的完整替代物品列表；空数组表示清空。 */
  substitutes: SubstituteReplacementItem[]
}

/** 查询全部已有替代关系；当前接口不支持分页或服务端筛选。 */
export function listSubstituteRelations(signal?: AbortSignal) {
  return apiClient.request<SubstituteRelationResponse[]>('/api/substitutes', { signal })
}

/** 查询指定物品的替代关系。 */
export function listItemSubstitutes(itemId: number, signal?: AbortSignal) {
  return apiClient.request<ItemSubstituteResponse[]>(`/api/substitutes/${itemId}`, { signal })
}

/** 整体替换指定物品的替代关系。 */
export function replaceItemSubstitutes(itemId: number, request: SubstituteReplaceRequest) {
  return apiClient.request<ItemSubstituteResponse[]>(`/api/substitutes/${itemId}`, {
    method: 'PUT',
    json: request,
  })
}
