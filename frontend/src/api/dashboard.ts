// 本文件拥有 frontend 库存总览 HTTP 契约和请求函数；它不计算库存指标或决定页面布局。
// DTO 通过 contract.ts 别名映射到生成 schema，导出名保持稳定。
import { apiClient } from "./client";
import type { ApiResponse, ApiSchema } from "./contract";

/** 呆滞物品摘要。 */
export type SlowMovingItem = ApiResponse<ApiSchema<"SlowMovingItem">>;

/** 库存总览摘要响应。 */
export type DashboardOverviewResponse = ApiResponse<ApiSchema<"DashboardOverviewResponse">>;

/** 单日出入库趋势。 */
export type DailyTrend = ApiResponse<ApiSchema<"DailyTrend">>;

/** 出入库趋势响应；生成 schema 名为 TrendsResponse。 */
export type DashboardTrendsResponse = ApiResponse<ApiSchema<"TrendsResponse">>;

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
