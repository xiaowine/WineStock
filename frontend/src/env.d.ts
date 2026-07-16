// 本文件声明 frontend 的 Vite 环境变量和平台运行时注入对象；它不读取或修改实际配置。
import type { FrontendRuntimeConfig } from "./api/runtime-config";
import type { ShellBridge } from "./shell/contract";

interface ImportMetaEnv {
  /** 开发或 Web 部署时使用的 WineStock HTTP 服务根地址。 */
  readonly VITE_API_BASE_URL?: string;
  /** 开发环境登录客户端类型。 */
  readonly VITE_CLIENT_KIND?: string;
  /** 开发环境设备名称。 */
  readonly VITE_DEVICE_NAME?: string;
  /** 开发环境客户端版本号。 */
  readonly VITE_APP_VERSION?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

declare global {
  interface Window {
    /** Desktop、Android 或 Web shell 在应用挂载前注入的运行时配置。 */
    __WINESTOCK_RUNTIME_CONFIG__?: FrontendRuntimeConfig;
    /** Desktop、Android 在页面脚本执行前注入的版本化 Shell Bridge。 */
    __WINESTOCK_SHELL_BRIDGE__?: ShellBridge;
  }
}

export {};
