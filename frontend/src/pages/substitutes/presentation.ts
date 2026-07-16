// 本文件拥有替代关系页面的数量与错误文案格式化；它不请求 API 或决定页面状态。
import { ApiError } from "../../api/errors";

export function formatRelationCount(groupCount: number, relationCount: number): string {
  return `${groupCount} 个主物品 / ${relationCount} 条关系`;
}

export function substituteErrorMessage(error: unknown): string {
  return error instanceof ApiError ? error.message : "无法连接到 WineStock 服务";
}
