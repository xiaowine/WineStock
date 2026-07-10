<!--
  本文件拥有桌面端应用壳，属于 frontend 布局层。
  它组织顶部状态、顶部账户摘要、路由导航和子路由出口，不拥有 token 规则或平台 shell 生命周期。
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

      <div class="desktop-account">
        <button
          class="desktop-account-trigger"
          type="button"
          :aria-expanded="accountMenuOpen"
          aria-controls="desktop-account-popover"
          :aria-label="`查看当前用户 ${userDisplayName} 的账户信息`"
          @click="toggleAccountMenu"
        >
          <AccountUserSummary :initials="userInitials" :display-name="userDisplayName" />
        </button>
        <button
          v-if="accountMenuOpen"
          class="account-popover-backdrop"
          type="button"
          aria-label="关闭账户信息"
          @click="closeAccountMenu"
        />
        <Transition name="account-popover">
          <AccountPopover
            v-if="accountMenuOpen"
            id="desktop-account-popover"
            :initials="userInitials"
            :display-name="userDisplayName"
            :show-user-summary="false"
            :logout-error="logoutError"
            :is-logging-out="isLoggingOut"
            @logout="handleLogout"
          />
        </Transition>
      </div>
    </header>

    <div class="desktop-workspace">
      <main class="main-viewport main-viewport--desktop" aria-label="主内容">
        <aside class="desktop-navigation-pane" aria-label="导航面板">
          <AppNavigationList :items="appNavigation" />
        </aside>

        <section class="desktop-content-pane">
          <RouterView />
        </section>
      </main>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { authSession, authStatus } from '../auth/session'
import AccountPopover from '../components/AccountPopover.vue'
import AccountUserSummary from '../components/AccountUserSummary.vue'
import AppNavigationList from '../components/AppNavigationList.vue'
import { useAccountPopover } from '../composables/useAccountPopover'
import { useShellLogout } from '../composables/useShellLogout'
import { appNavigation } from '../router/navigation'

const {
  accountMenuOpen,
  closeAccountMenu,
  toggleAccountMenu: toggleAccountPopover,
} = useAccountPopover()
const { handleLogout, isLoggingOut, logoutError } = useShellLogout()
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

function toggleAccountMenu(): void {
  logoutError.value = ''
  toggleAccountPopover()
}
</script>
