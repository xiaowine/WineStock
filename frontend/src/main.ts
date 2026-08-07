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
import { overlayScrollbarDirective } from "./directives/overlayScrollbar";
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
import {
  checkForUpdate,
  activeApiBaseUrl,
  initializeShellRuntime,
  reportFrontendReady,
  reportShellBridgeFailure,
} from "./shell/runtime";
import { notice } from "./notices/notice";
import { startTelemetryIfConsented } from "./telemetry/clarity";
import { disposeThemeRuntime, initializeTheme } from "./theme/runtime";
import { openAppUpdateDialog } from "./updates/appUpdate";
import { updateCheckErrorMessage } from "./updates/messages";
import { autoUpdateCheckEnabled } from "./updates/updatePreferences";

let stopNativeBackNavigation: (() => void) | null = null;

// 主题必须先于 Shell 初始化和其它异步启动工作生效，避免首屏等待期间露出错误背景。
initializeTheme();

async function bootstrapFrontend(): Promise<void> {
  try {
    await initializeShellRuntime();
  } catch (error) {
    // 原生桥失败时不再挂载 WebView 内的 UI；由平台 Shell 显示统一恢复提示。
    void reportShellBridgeFailure(error);
    return;
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

  createApp(App)
    .use(router)
    .directive("copyable", copyableDirective)
    .directive("overlay-scrollbar", overlayScrollbarDirective)
    .mount("#app");
  installOverlayScrollbars();
  if (autoUpdateCheckEnabled.value) {
    void checkForUpdate()
      .then((result) => {
        if (!result?.latestVersion) return;
        openAppUpdateDialog(result, "startup");
      })
      .catch((error: unknown) => {
        notice.warning("暂时无法检查更新", {
          detail: updateCheckErrorMessage(error),
          durationMs: 6_000,
        });
      });
  }
  // 按已持久化的同意偏好补启动匿名采集；未同意时该调用不发起任何请求。
  startTelemetryIfConsented();
  await nextTick();
  try {
    stopNativeBackNavigation = await installNativeBackNavigation(router);
  } catch (error) {
    // capability 声明与订阅不一致属于桥契约失败，交给平台 Shell 阻断 WebView。
    console.warn("无法安装平台原生返回订阅", error);
    void reportShellBridgeFailure(error, "shell_bridge_event_subscription_failed");
    return;
  }
  window.requestAnimationFrame(() => {
    void reportFrontendReady().catch((error: unknown) => {
      console.warn("无法向平台 Shell 报告前端就绪状态", error);
      void reportShellBridgeFailure(error, "shell_bridge_ready_failed");
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
