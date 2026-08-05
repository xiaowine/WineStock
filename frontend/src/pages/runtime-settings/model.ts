// 本文件拥有运行设置页面的模式判断、地址预览和配置差异规则；它不调用 Shell Bridge 或修改响应式状态。
import { normalizeApiBaseUrl } from "../../api/runtime-config";
import type { EditableRuntimeConfig, RuntimeMode } from "../../shell/contract";

/** UI 中远端客户端模式统一归入“连接远端”。 */
export function isRemoteRuntimeMode(mode: RuntimeMode): boolean {
  return mode === "client-only" || mode === "connect-to-remote";
}

/** 当前配置是否需要 Shell 管理本地 Axum。 */
export function isLocalRuntimeMode(mode: RuntimeMode): boolean {
  return mode === "self-hosted" || mode === "server-mode";
}

/** 根据有效草稿生成应用后的 API 地址预览；无效输入不展示不可访问的伪地址。 */
export function previewApiBaseUrl(config: EditableRuntimeConfig): string {
  if (isRemoteRuntimeMode(config.mode)) {
    try {
      return normalizeApiBaseUrl(config.remoteBaseUrl);
    } catch {
      return "";
    }
  }
  if (!Number.isInteger(config.port) || config.port < 1 || config.port > 65535) {
    return "";
  }
  return `http://127.0.0.1:${config.port}`;
}

/** 比较设置页草稿和 Shell 当前配置。 */
export function sameRuntimeConfig(
  left: EditableRuntimeConfig,
  right: EditableRuntimeConfig,
): boolean {
  return (
    left.mode === right.mode &&
    left.bindHost === right.bindHost &&
    left.port === right.port &&
    left.remoteBaseUrl === right.remoteBaseUrl
  );
}

/** 选择运行模式时补齐该模式的安全默认值，但保留用户已经输入的其它草稿。 */
export function applyRuntimeModeDefaults(
  config: EditableRuntimeConfig,
  mode: RuntimeMode,
): EditableRuntimeConfig {
  if (mode === "self-hosted") {
    return { ...config, mode, bindHost: "127.0.0.1", port: 0 };
  }
  if (mode === "server-mode") {
    return {
      ...config,
      mode,
      bindHost: config.bindHost === "127.0.0.1" ? "0.0.0.0" : config.bindHost,
      port: config.port > 0 ? config.port : 17890,
    };
  }
  return { ...config, mode, port: config.port > 0 ? config.port : 17890 };
}
