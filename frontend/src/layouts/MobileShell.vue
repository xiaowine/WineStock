<!--
  本文件拥有移动端应用壳，属于 frontend 布局层。
  它只组织顶部上下文栏、路由出口和左侧导航抽屉，不拥有原生 Android 生命周期。
-->
<template>
  <div class="app-shell mobile-shell" data-shell="mobile">
    <header class="mobile-topbar">
      <button class="icon-button" type="button" aria-label="打开导航" @click="navOpen = true">☰</button>
      <div class="mobile-topbar__title">
        <span>WineStock</span>
        <strong>{{ pageTitle }}</strong>
      </div>
      <div class="mobile-topbar__actions">
        <span class="icon-button icon-button--static" role="img" aria-label="当前用户 AD">AD</span>
      </div>
    </header>

    <main class="main-viewport main-viewport--mobile" aria-label="主内容">
      <RouterView />
    </main>

    <div v-if="navOpen" class="mobile-nav-layer" role="dialog" aria-modal="true" aria-label="导航面板">
      <button class="mobile-nav-backdrop" type="button" aria-label="关闭导航" @click="navOpen = false" />
      <aside class="mobile-nav-drawer">
        <div class="mobile-nav-drawer__header">
          <div>
            <p class="eyebrow">Navigation</p>
            <h2>工作区</h2>
          </div>
          <button class="icon-button" type="button" aria-label="关闭导航" @click="navOpen = false">×</button>
        </div>

        <div class="mobile-drawer-status">
          <span class="status-chip status-chip--ok">路由已就绪</span>
          <span class="status-chip">API 待接入</span>
        </div>

        <div class="menu-section">
          <h3>模块导航</h3>
          <RouterLink
            v-for="item in appNavigation"
            :key="item.routeName"
            :to="{ name: item.routeName }"
            class="menu-item"
            active-class="menu-item--active"
            @click="navOpen = false"
          >
            {{ item.label }}
          </RouterLink>
        </div>
      </aside>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRoute } from 'vue-router'
import { appNavigation } from '../router/navigation'

const navOpen = ref(false)
const route = useRoute()
const pageTitle = computed(() => route.meta.title)
</script>
