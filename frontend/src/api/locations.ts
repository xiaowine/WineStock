// 本文件拥有库位分组、库位和整批次移库的 HTTP 契约；它不管理页面树状态或库存数量。
// DTO 通过 contract.ts 别名映射到生成 schema，导出名保持稳定；查询参数模型仍手写。
import { apiClient } from "./client";
import type { ApiResponse, ApiSchema } from "./contract";

/** 库位分组基础响应。 */
export type LocationGroupResponse = ApiResponse<ApiSchema<"LocationGroupResponse">>;

/** 可被入库明细和库存批次引用的库位。 */
export type LocationResponse = ApiResponse<ApiSchema<"LocationResponse">>;

/** 库位分组树节点，包含直接库位和直接子分组。 */
export type LocationGroupTreeNode = ApiResponse<ApiSchema<"LocationGroupTreeNode">>;

/** 创建库位分组请求。 */
export type LocationGroupCreateRequest = ApiSchema<"LocationGroupCreateRequest">;

/** 更新库位分组请求。 */
export type LocationGroupUpdateRequest = ApiSchema<"LocationGroupUpdateRequest">;

/** 创建库位请求。 */
export type LocationCreateRequest = ApiSchema<"LocationCreateRequest">;

/** 更新库位请求。 */
export type LocationUpdateRequest = ApiSchema<"LocationUpdateRequest">;

/** 库位列表查询条件。 */
export interface LocationListQuery {
  /** 可选所属分组 ID。 */
  group_id?: number;
  /** 可选名称或备注搜索词。 */
  search?: string;
}

/** 整批次移库请求。 */
export type LocationTransferCreateRequest = ApiSchema<"LocationTransferCreateRequest">;

/** 整批次移库结果。 */
export type LocationTransferResponse = ApiResponse<ApiSchema<"LocationTransferResponse">>;

/** 查询未删除库位分组树。 */
export function listLocationGroupTree(signal?: AbortSignal) {
  return apiClient.request<LocationGroupTreeNode[]>("/api/location-groups/tree", { signal });
}

/** 创建库位分组。 */
export function createLocationGroup(request: LocationGroupCreateRequest) {
  return apiClient.request<LocationGroupResponse>("/api/location-groups", {
    method: "POST",
    json: request,
  });
}

/** 更新库位分组基础资料或上级分组。 */
export function updateLocationGroup(groupId: number, request: LocationGroupUpdateRequest) {
  return apiClient.request<LocationGroupResponse>(`/api/location-groups/${groupId}`, {
    method: "PUT",
    json: request,
  });
}

/** 软删除空库位分组。 */
export function deleteLocationGroup(groupId: number) {
  return apiClient.request<void>(`/api/location-groups/${groupId}`, { method: "DELETE" });
}

/** 查询未删除库位，可按分组、名称或备注搜索。 */
export function listLocations(query: LocationListQuery = {}, signal?: AbortSignal) {
  return apiClient.request<LocationResponse[]>("/api/locations", { query: { ...query }, signal });
}

/** 创建库位。 */
export function createLocation(request: LocationCreateRequest) {
  return apiClient.request<LocationResponse>("/api/locations", {
    method: "POST",
    json: request,
  });
}

/** 更新库位基础资料。 */
export function updateLocation(locationId: number, request: LocationUpdateRequest) {
  return apiClient.request<LocationResponse>(`/api/locations/${locationId}`, {
    method: "PUT",
    json: request,
  });
}

/** 软删除当前没有库存批次引用的库位。 */
export function deleteLocation(locationId: number) {
  return apiClient.request<void>(`/api/locations/${locationId}`, { method: "DELETE" });
}

/** 将仍有余额的库存批次整体移动到另一个库位。 */
export function createLocationTransfer(request: LocationTransferCreateRequest) {
  return apiClient.request<LocationTransferResponse>("/api/location-transfers", {
    method: "POST",
    json: request,
  });
}
