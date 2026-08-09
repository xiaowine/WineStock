// 应用更新文案：update.<稳定码>。键与 desktop/update.rs、android AppUpdateManager 的码
// 逐字对应（snake_case）。
export const update = {
  update_manifest_invalid: "更新服务返回了无效清单，请稍后重试",
  update_install_permission_required: "请在系统设置中允许 WineStock 安装未知来源应用后重试",
  update_integrity_failed: "更新文件校验失败，已停止安装",
  update_not_available: "该版本已不可用，请重新检查更新",
  update_download_failed: "更新文件下载失败，请检查网络后重试",
  update_check_unavailable: "暂时无法连接更新服务，请稍后重试",
  update_install_failed: "更新安装失败，请稍后重试",
  update_check_retry_later: "请检查网络，或稍后在偏好设置中重试",
};
