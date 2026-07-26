// 本文件拥有物品替代关系的 HTTP 契约和请求函数；替代关系的整体替换语义由服务端负责校验。
// DTO 通过 contract.ts 别名映射到生成 schema，导出名保持稳定。
import { apiClient } from "./client";
import type { ApiResponse, ApiSchema } from "./contract";

/** 指定物品的一条替代关系响应。 */
export type ItemSubstituteResponse = ApiResponse<ApiSchema<"ItemSubstituteResponse">>;

/** 全局替代关系列表中的单条有向关系；全局接口不提供单位和库存状态。 */
export type SubstituteRelationResponse = ApiResponse<ApiSchema<"SubstituteRelationResponse">>;

/** 替代关系整体替换请求中的单条关系。 */
export type SubstituteReplacementItem = ApiSchema<"SubstituteReplacementItem">;

/** 替代关系整体替换请求；空数组表示清空。 */
export type SubstituteReplaceRequest = ApiSchema<"SubstituteReplaceRequest">;

/** 查询全部已有替代关系；当前接口不支持分页或服务端筛选。 */
export function listSubstituteRelations(signal?: AbortSignal) {
  return apiClient.request<SubstituteRelationResponse[]>("/api/substitutes", { signal });
}

/** 查询指定物品的替代关系。 */
export function listItemSubstitutes(itemId: number, signal?: AbortSignal) {
  return apiClient.request<ItemSubstituteResponse[]>(`/api/substitutes/${itemId}`, { signal });
}

/** 整体替换指定物品的替代关系。 */
export function replaceItemSubstitutes(itemId: number, request: SubstituteReplaceRequest) {
  return apiClient.request<ItemSubstituteResponse[]>(`/api/substitutes/${itemId}`, {
    method: "PUT",
    json: request,
  });
}
