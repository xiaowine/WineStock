<!--
  本文件拥有桌面端应用壳，属于 frontend 布局层。
  它只组织顶部状态栏、路由导航和子路由出口，不拥有业务 API 或平台 shell 生命周期。
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
        <span class="status-chip">API 待接入</span>
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

          <SidebarUserSummary :initials="userInitials" :display-name="userDisplayName" />
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
import { authSession } from '../auth/session'
import SidebarUserSummary from '../components/SidebarUserSummary.vue'
import { appNavigation } from '../router/navigation'

const userDisplayName = computed(() => authSession.value?.user.username ?? '未登录')
const userInitials = computed(() => {
  const characters = Array.from(userDisplayName.value.trim())
  return characters.slice(0, 2).join('').toUpperCase() || '--'
})
</script>
