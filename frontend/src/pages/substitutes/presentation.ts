// 本文件拥有替代关系页面的数量与错误文案格式化；它不请求 API 或决定页面状态。
import { ApiError } from "../../api/errors";
import { i18n, translateMessageOrNull } from "../../i18n";

export function formatRelationCount(groupCount: number, relationCount: number): string {
  return i18n.global.t("substitutes.countSummary", { groupCount, relationCount });
}

export function substituteErrorMessage(error: unknown): string {
  if (error instanceof ApiError) return error.message;
  return (
    translateMessageOrNull("substitutes.networkUnavailable") ??
    "Cannot connect to the WineStock service"
  );
}
