// 本文件拥有复制交互的纯规则（指令绑定值归一化、Notice detail 取舍）；
// 它不接触 DOM、剪贴板 API 或全局 Notice。
export interface CopyRequest {
  /** 要写入剪贴板的完整文本。 */
  readonly text: string;
  /** 反馈文案中的内容名称，如「连接地址」→"连接地址已复制"。 */
  readonly label?: string;
}

/** 超过该长度的内容不进 Notice detail（如整段 JSON），避免通知区被撑爆。 */
export const COPY_DETAIL_MAX_LENGTH = 120;

/**
 * 归一化 v-copyable 的绑定值：字符串、{ text, label } 或缺省（回退元素文本）。
 * 归一化后文本为空视为无效，返回 null（调用方应跳过复制）。
 */
export function normalizeCopyableValue(value: unknown, fallbackText: string): CopyRequest | null {
  if (typeof value === "string") {
    const text = value.trim();
    return text ? { text } : null;
  }
  if (typeof value === "object" && value !== null && "text" in value) {
    const record = value as { text: unknown; label?: unknown };
    if (typeof record.text !== "string") return null;
    const text = record.text.trim();
    if (!text) return null;
    return typeof record.label === "string" && record.label
      ? { text, label: record.label }
      : { text };
  }
  if (value === undefined || value === null) {
    const text = fallbackText.trim();
    return text ? { text } : null;
  }
  return null;
}

/** 成功 Notice 的 detail：短内容原样展示，长内容（JSON 等）省略。 */
export function copyNoticeDetail(text: string): string | undefined {
  return text.length <= COPY_DETAIL_MAX_LENGTH ? text : undefined;
}
