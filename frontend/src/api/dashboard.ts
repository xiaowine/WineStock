// 本文件拥有 frontend 库存总览 HTTP 契约和请求函数；它不计算库存指标或决定页面布局。
import { apiClient } from "./client";

/** 呆滞物品摘要。 */
export interface SlowMovingItem {
  /** 物品 ID。 */
  item_id: number;
  /** 物品名称。 */
  item_name: string;
  /** 当前库存数量。 */
  quantity: number;
  /** 当前库存价值。 */
  value: number;
  /** 最近一次出入库距今天数。 */
  days_since_last_movement: number;
}

/** 库存总览摘要响应。 */
export interface DashboardOverviewResponse {
  /** 未删除物品种类数。 */
  total_items: number;
  /** 当前库存总数量。 */
  total_quantity: number;
  /** 当前库存总价值。 */
  total_value: number;
  /** 最近三天入库数量。 */
  inbound_3d: number;
  /** 最近三天出库数量。 */
  outbound_3d: number;
  /** 当前呆滞物品列表。 */
  slow_moving_items: SlowMovingItem[];
}

/** 单日出入库趋势。 */
export interface DailyTrend {
  /** 日期，格式为 YYYY-MM-DD。 */
  date: string;
  /** 当日入库数量。 */
  inbound_quantity: number;
  /** 当日出库数量。 */
  outbound_quantity: number;
}

/** 出入库趋势响应。 */
export interface DashboardTrendsResponse {
  /** 按日期升序排列的每日趋势。 */
  daily: DailyTrend[];
}

/** 查询库存总览摘要。 */
export function getDashboardOverview(signal?: AbortSignal): Promise<DashboardOverviewResponse> {
  return apiClient.request<DashboardOverviewResponse>("/api/dashboard/overview", { signal });
}

/** 查询指定天数的出入库趋势。 */
export function getDashboardTrends(
  days: number,
  signal?: AbortSignal,
): Promise<DashboardTrendsResponse> {
  return apiClient.request<DashboardTrendsResponse>("/api/dashboard/trends", {
    query: { days },
    signal,
  });
}
