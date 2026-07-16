// 本文件拥有库位分组、库位和整批次移库的 HTTP 契约；它不管理页面树状态或库存数量。
import { apiClient } from "./client";

/** 库位分组基础响应。 */
export interface LocationGroupResponse {
  /** 分组 ID。 */
  id: number;
  /** 上级分组 ID；为空表示根分组。 */
  parent_id: number | null;
  /** 分组名称。 */
  name: string;
  /** 同级排序值。 */
  sort_order: number;
  /** 创建时间。 */
  created_at: string;
  /** 最近更新时间。 */
  updated_at: string;
}

/** 可被入库明细和库存批次引用的库位。 */
export interface LocationResponse {
  /** 库位 ID。 */
  id: number;
  /** 所属分组 ID。 */
  group_id: number;
  /** 所属分组名称。 */
  group_name: string;
  /** 全局唯一库位名称。 */
  name: string;
  /** 可选库位备注。 */
  notes: string | null;
  /** 同组排序值。 */
  sort_order: number;
  /** 创建时间。 */
  created_at: string;
  /** 最近更新时间。 */
  updated_at: string;
}

/** 库位分组树节点，包含直接库位和直接子分组。 */
export interface LocationGroupTreeNode extends LocationGroupResponse {
  /** 当前分组直接拥有的库位。 */
  locations: LocationResponse[];
  /** 当前分组直接拥有的子分组。 */
  children: LocationGroupTreeNode[];
}

/** 创建库位分组请求。 */
export interface LocationGroupCreateRequest {
  /** 上级分组 ID；为空表示根分组。 */
  parent_id?: number | null;
  /** 分组名称。 */
  name: string;
  /** 同级排序值。 */
  sort_order?: number | null;
}

/** 更新库位分组请求。 */
export interface LocationGroupUpdateRequest {
  /** 上级分组 ID；为空表示根分组。 */
  parent_id: number | null;
  /** 分组名称。 */
  name: string;
  /** 同级排序值。 */
  sort_order?: number | null;
}

/** 创建库位请求。 */
export interface LocationCreateRequest {
  /** 所属分组 ID。 */
  group_id: number;
  /** 全局唯一库位名称。 */
  name: string;
  /** 可选库位备注。 */
  notes?: string | null;
  /** 同组排序值。 */
  sort_order?: number | null;
}

/** 更新库位请求。 */
export type LocationUpdateRequest = LocationCreateRequest;

/** 库位列表查询条件。 */
export interface LocationListQuery {
  /** 可选所属分组 ID。 */
  group_id?: number;
  /** 可选名称或备注搜索词。 */
  search?: string;
}

/** 整批次移库请求。 */
export interface LocationTransferCreateRequest {
  /** 被移动库存批次 ID。 */
  batch_id: number;
  /** 调用方确认的当前原库位 ID。 */
  from_location_id: number;
  /** 目标库位 ID。 */
  to_location_id: number;
  /** 可选移库备注。 */
  notes?: string;
}

/** 整批次移库结果。 */
export interface LocationTransferResponse {
  /** 移库记录 ID。 */
  id: number;
  /** 被移动批次 ID。 */
  batch_id: number;
  /** 被移动物品 ID。 */
  item_id: number;
  /** 原库位 ID。 */
  from_location_id: number;
  /** 目标库位 ID。 */
  to_location_id: number;
  /** 被移动的完整当前批次余额。 */
  quantity: number;
  /** 可选移库备注。 */
  notes: string | null;
  /** 操作人用户 ID。 */
  created_by_user_id: number | null;
  /** 移库时间。 */
  created_at: string;
}

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
