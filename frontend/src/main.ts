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
import { router } from "./router";
import { installAuthGuards } from "./router/guards";
import {
  checkServiceAvailability,
  reportServiceUnavailable,
  startServiceAvailabilityMonitor,
  successfulServiceCheckSequence,
} from "./service/availability";
import { activeApiBaseUrl, initializeShellRuntime, reportFrontendReady } from "./shell/runtime";

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

  createApp(App).use(router).mount("#app");
  installOverlayScrollbars();
  await nextTick();
  window.requestAnimationFrame(() => {
    void reportFrontendReady().catch((error: unknown) => {
      console.warn("无法向平台 Shell 报告前端就绪状态", error);
    });
  });
}

void bootstrapFrontend();
