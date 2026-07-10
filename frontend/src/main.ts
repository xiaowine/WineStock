// 本文件拥有 frontend Vue 应用装配入口，注册路由并挂载根组件；它不拥有平台 shell 生命周期。
import { createApp } from 'vue'
import './style.css'
import App from './App.vue'
import { apiClient } from './api/client'
import {
  getValidAccessToken,
  restoreAuthSession,
  startAuthSessionSynchronization,
} from './auth/session'
import { router } from './router'

apiClient.setAccessTokenProvider(getValidAccessToken)
startAuthSessionSynchronization()

// 会话恢复失败不能阻止页面挂载；持久 token 会在后续 API 请求时再次尝试刷新。
void restoreAuthSession().catch((error: unknown) => {
  console.warn('恢复 WineStock 登录状态失败', error)
})

createApp(App).use(router).mount('#app')
