// 本文件拥有 frontend 对 WineStock 健康检查接口的调用；它不决定重试频率或页面呈现。
import { apiClient } from "./client";
import { ApiResponseError } from "./errors";
import { resolveApiBaseUrl } from "./runtime-config";

/** Axum 健康检查响应。 */
export interface HealthResponse {
  /** 服务状态；当前可用时固定为 `OK`。 */
  status: "OK";
}

/** 无鉴权检查当前配置的 WineStock 服务是否能够正常响应。 */
export async function checkHealth(signal?: AbortSignal): Promise<void> {
  const response = await apiClient.request<unknown>("/api/health", {
    authenticated: false,
    signal,
  });

  if (!isHealthResponse(response)) {
    throw new ApiResponseError(
      `${resolveApiBaseUrl()}/api/health`,
      new Error("健康检查响应不符合预期"),
    );
  }
}

function isHealthResponse(value: unknown): value is HealthResponse {
  return typeof value === "object" && value !== null && "status" in value && value.status === "OK";
}
