<!--
  本文件拥有桌面端应用壳，属于 frontend 布局层。
  它组织顶部状态、路由导航、账户退出入口和子路由出口，不拥有 token 规则或平台 shell 生命周期。
-->
<template>
  <div class="app-shell desktop-shell" data-shell="desktop">
    <header class="desktop-nav">
      <div class="brand-lockup">
        <span class="brand-mark">W</span>
        <span class="brand-name">WineStock</span>
      </div>

      <div class="desktop-nav__status" aria-label="服务状态">
        <span class="status-chip status-chip--ok">路由已就绪</span>
        <span
          class="status-chip"
          :class="{
            'status-chip--ok': authStatus === 'authenticated',
            'status-chip--warning': authStatus === 'unavailable',
          }"
        >
          {{ authStatusLabel }}
        </span>
      </div>
    </header>

    <div class="desktop-workspace">
      <main class="main-viewport main-viewport--desktop" aria-label="主内容">
        <aside class="desktop-navigation-pane" aria-label="导航面板">
          <div class="navigation-pane__header">
            <p class="eyebrow">WineStock</p>
            <h2>主导航</h2>
          </div>

          <div class="menu-section">
            <h3>模块导航</h3>
            <RouterLink
              v-for="item in appNavigation"
              :key="item.routeName"
              :to="{ name: item.routeName }"
              class="menu-item"
              active-class="menu-item--active"
            >
              {{ item.label }}
            </RouterLink>
          </div>

          <div class="sidebar-account-panel">
            <SidebarUserSummary :initials="userInitials" :display-name="userDisplayName" />
            <p v-if="logoutError" class="sidebar-account-error" role="alert">
              {{ logoutError }}
            </p>
            <button
              class="secondary-button sidebar-logout-button"
              type="button"
              :disabled="isLoggingOut"
              @click="handleLogout"
            >
              {{ isLoggingOut ? '正在退出…' : '退出登录' }}
            </button>
          </div>
        </aside>

        <section class="desktop-content-pane">
          <RouterView />
        </section>
      </main>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { AuthPersistenceError } from '../auth/storage'
import {
  authSession,
  authStatus,
  isLoggingOut,
  logoutAuthSession,
  type LogoutResult,
} from '../auth/session'
import SidebarUserSummary from '../components/SidebarUserSummary.vue'
import { appNavigation } from '../router/navigation'

const router = useRouter()
const logoutError = ref('')
const userDisplayName = computed(
  () =>
    authSession.value?.user.username ??
    (authStatus.value === 'unavailable' ? '登录状态待恢复' : '未登录'),
)
const userInitials = computed(() => {
  const characters = Array.from(userDisplayName.value.trim())
  return characters.slice(0, 2).join('').toUpperCase() || '--'
})
const authStatusLabel = computed(() => {
  if (authStatus.value === 'authenticated') {
    return '会话已验证'
  }
  if (authStatus.value === 'unavailable') {
    return '服务连接异常'
  }
  return authStatus.value === 'restoring' || authStatus.value === 'idle'
    ? '正在恢复会话'
    : '当前未登录'
})

/** 吊销服务端会话并退出本机；仅在服务端吊销未确认时把固定提示带到登录页。 */
async function handleLogout(): Promise<void> {
  logoutError.value = ''

  let result: LogoutResult
  try {
    result = await logoutAuthSession()
  } catch (error) {
    logoutError.value =
      error instanceof AuthPersistenceError
        ? '无法清除本地登录状态，请检查浏览器存储权限后重试'
        : '退出失败，请稍后重试'
    return
  }

  await router.replace({
    name: 'login',
    query: result === 'local_only' ? { logout: 'local_only' } : undefined,
  })
}
</script>
