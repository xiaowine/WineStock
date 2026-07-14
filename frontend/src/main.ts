// 本文件拥有 frontend Vue 应用装配入口，连接服务监控、鉴权会话、路由守卫、全局浮层滚动条并挂载根组件；它不拥有平台 shell 生命周期。
import { createApp, watch } from 'vue'
import './styles/index.scss'
import App from './App.vue'
import { apiClient } from './api/client'
import { startAuthSessionAutoRefresh } from './auth/auto-refresh'
import {
  authStatus,
  ensureAuthSessionInitialized,
  getValidAccessToken,
  startAuthSessionSynchronization,
} from './auth/session'
import { installOverlayScrollbars } from './bootstrap/overlayScrollbars'
import { router } from './router'
import { installAuthGuards } from './router/guards'
import {
  reportServiceUnavailable,
  startServiceAvailabilityMonitor,
  successfulServiceCheckSequence,
} from './service/availability'

apiClient.setAccessTokenProvider(getValidAccessToken)
apiClient.setNetworkErrorHandler(reportServiceUnavailable)
startAuthSessionSynchronization()
startAuthSessionAutoRefresh()
startServiceAvailabilityMonitor()
installAuthGuards(router)

let handledServiceRecoverySequence = 0
watch(
  [successfulServiceCheckSequence, authStatus],
  ([sequence, status]) => {
    if (
      status !== 'unavailable' ||
      sequence === 0 ||
      sequence === handledServiceRecoverySequence
    ) {
      return
    }

    handledServiceRecoverySequence = sequence
    void ensureAuthSessionInitialized()
  },
  { flush: 'sync' },
)

// 提前启动统一恢复 Promise；路由守卫仍会等待同一个任务后再决定是否放行。
void ensureAuthSessionInitialized()

createApp(App).use(router).mount('#app')
installOverlayScrollbars()
