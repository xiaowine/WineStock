// 本文件拥有剪贴板写入与统一复制反馈；它不决定哪些内容可复制（见 v-copyable 指令与各调用方）。
import { translateMessageOrNull } from "../i18n";
import { notice } from "../notices/notice";
import { copyNoticeDetail, type CopyRequest } from "./model";

/**
 * 写入剪贴板：安全上下文用异步 Clipboard API；
 * 否则降级隐藏 textarea + execCommand，覆盖 LAN HTTP 纯浏览器场景。
 */
export async function copyText(text: string): Promise<boolean> {
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return true;
    }
  } catch {
    // Clipboard API 被权限策略拒绝时继续走降级路径。
  }
  try {
    const textarea = document.createElement("textarea");
    textarea.value = text;
    textarea.setAttribute("readonly", "");
    textarea.style.position = "fixed";
    textarea.style.opacity = "0";
    document.body.appendChild(textarea);
    textarea.select();
    const succeeded = document.execCommand("copy");
    textarea.remove();
    return succeeded;
  } catch {
    return false;
  }
}

/** 复制并按项目惯例发全局 Notice（成功含短内容 detail，失败给手动复制指引）。 */
export async function copyWithFeedback(request: CopyRequest): Promise<boolean> {
  const succeeded = await copyText(request.text);
  const copied = translateMessageOrNull("common.copied") ?? "Copied";
  if (succeeded) {
    const copiedWithLabel = translateMessageOrNull("components.copiedWithLabel");
    notice.success(
      request.label
        ? (copiedWithLabel
            ? interpolate(copiedWithLabel, { label: request.label })
            : `${request.label} ${copied}`)
        : copied,
      { detail: copyNoticeDetail(request.text) },
    );
  } else {
    const failed = translateMessageOrNull("components.copyFailed") ?? "Copy failed";
    const failedWithLabel = translateMessageOrNull("components.copyFailedWithLabel");
    const manualHint =
      translateMessageOrNull("components.copyManualHint") ??
      "Press and hold or select the content to copy it manually.";
    notice.error(
      request.label
        ? (failedWithLabel
            ? interpolate(failedWithLabel, { label: request.label })
            : `${failed}: ${request.label}`)
        : failed,
      { detail: manualHint },
    );
  }
  return succeeded;
}

/** 替换消息模板中的 `{name}` 占位符；占位符缺失时原样保留模板。 */
function interpolate(template: string, params: Record<string, string>): string {
  return template.replace(/\{(\w+)\}/g, (match, name: string) => params[name] ?? match);
}
