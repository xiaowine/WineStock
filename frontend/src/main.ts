// 本文件拥有 frontend Vue 应用装配入口，连接服务监控、鉴权会话、路由守卫、全局浮层滚动条并挂载根组件；它不拥有平台 shell 生命周期。
import { createApp, nextTick, watch } from "vue";
import "./styles/index.scss";
import App from "./App.vue";
import { apiClient } from "./api/client";
import { startAuthSessionAutoRefresh } from "./auth/auto-refresh";
import {
  authStatus,
  ensureAuthSessionInitialized,
  getValidAccessToken,
  startAuthSessionSynchronization,
} from "./auth/session";
import { installOverlayScrollbars } from "./bootstrap/overlayScrollbars";
import { copyableDirective } from "./directives/copyable";
import { installNativeBackNavigation } from "./navigation/nativeBack";
import { router } from "./router";
import { installAuthGuards } from "./router/guards";
import { installNavigationPendingTracking } from "./router/navigationPending";
import {
  checkServiceAvailability,
  reportServiceUnavailable,
  startServiceAvailabilityMonitor,
  successfulServiceCheckSequence,
} from "./service/availability";
import { activeApiBaseUrl, initializeShellRuntime, reportFrontendReady } from "./shell/runtime";
import { startTelemetryIfConsented } from "./telemetry/clarity";
import { disposeThemeRuntime, initializeTheme } from "./theme/runtime";

let stopNativeBackNavigation: (() => void) | null = null;

// 主题必须先于 Shell 初始化和其它异步启动工作生效，避免首屏等待期间露出错误背景。
initializeTheme();

async function bootstrapFrontend(): Promise<void> {
  try {
    await initializeShellRuntime();
  } catch {
    // Shell 初始化失败时仍挂载运行设置页，由前端展示可恢复错误。
  }

  apiClient.setAccessTokenProvider(getValidAccessToken);
  apiClient.setNetworkErrorHandler(reportServiceUnavailable);
  startAuthSessionSynchronization();
  startAuthSessionAutoRefresh();
  // 等待追踪先于鉴权守卫注册，守卫内的会话初始化等待也计入切换反馈。
  installNavigationPendingTracking(router);
  installAuthGuards(router);

  if (activeApiBaseUrl.value) {
    startServiceAvailabilityMonitor();
    void ensureAuthSessionInitialized();
  }

  watch(activeApiBaseUrl, (current, previous) => {
    if (!current || current === previous) {
      return;
    }
    startServiceAvailabilityMonitor();
    void checkServiceAvailability();
    void ensureAuthSessionInitialized();
  });

  let handledServiceRecoverySequence = 0;
  watch(
    [successfulServiceCheckSequence, authStatus],
    ([sequence, status]) => {
      if (
        status !== "unavailable" ||
        sequence === 0 ||
        sequence === handledServiceRecoverySequence
      ) {
        return;
      }

      handledServiceRecoverySequence = sequence;
      void ensureAuthSessionInitialized();
    },
    { flush: "sync" },
  );

  createApp(App).use(router).directive("copyable", copyableDirective).mount("#app");
  installOverlayScrollbars();
  // 按已持久化的同意偏好补启动匿名采集；未同意时该调用不发起任何请求。
  startTelemetryIfConsented();
  await nextTick();
  try {
    stopNativeBackNavigation = await installNativeBackNavigation(router);
  } catch (error) {
    // capability 声明与订阅不一致时保持页面未 ready，让 Android 直接使用 native fallback。
    console.warn("无法安装平台原生返回订阅", error);
    return;
  }
  window.requestAnimationFrame(() => {
    void reportFrontendReady().catch((error: unknown) => {
      console.warn("无法向平台 Shell 报告前端就绪状态", error);
    });
  });
}

void bootstrapFrontend();

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    stopNativeBackNavigation?.();
    stopNativeBackNavigation = null;
    disposeThemeRuntime();
  });
}
