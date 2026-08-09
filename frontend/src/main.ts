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
import { i18n, initializeI18n, installDocumentTitleSync, translateMessageOrNull, translateTitle } from "./i18n";
import { startTelemetryIfConsented } from "./telemetry/clarity";
import { disposeThemeRuntime, initializeTheme } from "./theme/runtime";
import { openAppUpdateDialog } from "./updates/appUpdate";
import { updateCheckErrorMessage } from "./updates/messages";
import { autoUpdateCheckEnabled } from "./updates/updatePreferences";

let stopNativeBackNavigation: (() => void) | null = null;

// 主题必须先于 Shell 初始化和其它异步启动工作生效，避免首屏等待期间露出错误背景。
initializeTheme();
// 语言同样先于任何文案渲染生效；语言包静态打包，切换不发起网络请求。
initializeI18n();

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
  installDocumentTitleSync(router);

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

  const app = createApp(App);
  app.config.globalProperties.$title = translateTitle;
  app.use(i18n);
  app.use(router);
  app.directive("copyable", copyableDirective);
  app.directive("overlay-scrollbar", overlayScrollbarDirective);
  app.mount("#app");
  installOverlayScrollbars();
  if (autoUpdateCheckEnabled.value) {
    void checkForUpdate()
      .then((result) => {
        if (!result?.latestVersion) return;
        openAppUpdateDialog(result, "startup");
      })
      .catch((error: unknown) => {
        notice.warning(translateMessageOrNull("misc.updateCheckFailed") ?? "Unable to check for updates", {
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
    console.warn("Failed to install platform native back navigation subscription", error);
    void reportShellBridgeFailure(error, "shell_bridge_event_subscription_failed");
    return;
  }
  window.requestAnimationFrame(() => {
    void reportFrontendReady().catch((error: unknown) => {
      console.warn("Failed to report frontend readiness to the platform shell", error);
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
