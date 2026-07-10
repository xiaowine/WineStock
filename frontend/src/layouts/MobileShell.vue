<!--
  本文件拥有移动端应用壳，属于 frontend 布局层。
  它组织顶部上下文栏、紧凑账户弹层、路由出口和左侧导航抽屉，不拥有原生 Android 生命周期。
-->
<template>
  <div class="app-shell mobile-shell" data-shell="mobile">
    <header class="mobile-topbar">
      <button class="icon-button" type="button" aria-label="打开导航" @click="openNavigation">☰</button>
      <div class="mobile-topbar__title">
        <span>WineStock</span>
        <strong>{{ pageTitle }}</strong>
      </div>
      <div v-if="userDisplayName" class="mobile-topbar__actions">
        <button
          class="mobile-account-trigger"
          type="button"
          :aria-expanded="accountMenuOpen"
          aria-controls="mobile-account-popover"
          :aria-label="`查看当前用户 ${userDisplayName} 的账户信息`"
          @click="toggleAccountMenu"
        >
          <span class="account-user-summary__avatar" aria-hidden="true">{{ userInitials }}</span>
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
            id="mobile-account-popover"
            :initials="userInitials"
            :display-name="userDisplayName"
            :logout-error="logoutError"
            :is-logging-out="isLoggingOut"
            @logout="handleLogout"
          />
        </Transition>
      </div>
    </header>

    <main class="main-viewport main-viewport--mobile" aria-label="主内容">
      <RouteContentView />
    </main>

    <div v-if="navOpen" class="mobile-nav-layer" role="dialog" aria-modal="true" aria-label="导航面板">
      <button class="mobile-nav-backdrop" type="button" aria-label="关闭导航" @click="navOpen = false" />
      <aside class="mobile-nav-drawer">
        <div class="mobile-nav-drawer__header">
          <div>
            <h2>导航</h2>
          </div>
          <button class="icon-button" type="button" aria-label="关闭导航" @click="navOpen = false">×</button>
        </div>

        <AppNavigationList :items="visibleNavigation" @navigate="navOpen = false" />
      </aside>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { authSession } from '../auth/session'
import AccountPopover from '../components/AccountPopover.vue'
import AppNavigationList from '../components/AppNavigationList.vue'
import RouteContentView from '../components/RouteContentView.vue'
import { useAccountPopover } from '../composables/useAccountPopover'
import { useShellLogout } from '../composables/useShellLogout'
import { getVisibleAppNavigation } from '../router/navigation'

const navOpen = ref(false)
const route = useRoute()
const {
  accountMenuOpen,
  closeAccountMenu,
  toggleAccountMenu: toggleAccountPopover,
} = useAccountPopover()
const { handleLogout, isLoggingOut, logoutError } = useShellLogout()
const pageTitle = computed(() => route.meta.title)
const visibleNavigation = computed(() =>
  getVisibleAppNavigation(authSession.value?.user.permissions),
)
const userDisplayName = computed(() => authSession.value?.user.username ?? '')
const userInitials = computed(() =>
  Array.from(userDisplayName.value.trim()).slice(0, 2).join('').toUpperCase(),
)

function openNavigation(): void {
  accountMenuOpen.value = false
  navOpen.value = true
}

function toggleAccountMenu(): void {
  navOpen.value = false
  logoutError.value = ''
  toggleAccountPopover()
}

/** Escape 关闭移动导航抽屉；账户弹层由共享 composable 处理。 */
function handleEscape(event: KeyboardEvent): void {
  if (event.key === 'Escape') {
    navOpen.value = false
  }
}

watch(() => route.fullPath, () => {
  navOpen.value = false
})

onMounted(() => document.addEventListener('keydown', handleEscape))
onBeforeUnmount(() => document.removeEventListener('keydown', handleEscape))
</script>
