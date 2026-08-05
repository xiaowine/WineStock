// 本模块把 Shell Bridge 更新错误码映射为前端安全文案，不解析底层异常字符串。

function updateErrorCode(error: unknown): unknown {
  return error instanceof Error ? (error as Error & { code?: unknown }).code : undefined;
}

export function updateCheckErrorMessage(error: unknown): string {
  if (updateErrorCode(error) === "update_manifest_invalid") {
    return "更新服务返回了无效清单，请稍后在偏好设置中重试。";
  }
  return "请检查网络，或稍后在偏好设置中重试。";
}

export function updateInstallErrorMessage(error: unknown): string {
  switch (updateErrorCode(error)) {
    case "update_install_permission_required":
      return "请在系统设置中允许 WineStock 安装未知来源应用后重试。";
    case "update_integrity_failed":
      return "更新文件校验失败，已停止安装。";
    case "update_manifest_invalid":
      return "更新服务返回了无效清单，请稍后重试。";
    case "update_not_available":
      return "该版本已不可用，请重新检查更新。";
    case "update_download_failed":
      return "更新文件下载失败，请检查网络后重试。";
    default:
      return "暂时无法连接更新服务，请稍后重试。";
  }
}
