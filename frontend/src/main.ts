// 本文件拥有 frontend Vue 应用装配入口，连接鉴权会话、路由守卫并挂载根组件；它不拥有平台 shell 生命周期。
import { createApp } from 'vue'
import './style.css'
import App from './App.vue'
import { apiClient } from './api/client'
import {
  ensureAuthSessionInitialized,
  getValidAccessToken,
  startAuthSessionSynchronization,
} from './auth/session'
import { router } from './router'
import { installAuthGuards } from './router/guards'

apiClient.setAccessTokenProvider(getValidAccessToken)
startAuthSessionSynchronization()
installAuthGuards(router)

// 提前启动统一恢复 Promise；路由守卫仍会等待同一个任务后再决定是否放行。
void ensureAuthSessionInitialized()

createApp(App).use(router).mount('#app')
