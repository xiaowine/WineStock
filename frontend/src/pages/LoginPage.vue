<!--
  本文件拥有登录路由的响应式页面选择，属于 frontend 鉴权页面层。
  桌面端已接入登录表单，移动端仍保留占位内容；它不决定 token 的平台持久化方式。
-->
<template>
  <DesktopLoginPage v-if="isDesktop" />

  <main v-else class="standalone-page">
    <section class="standalone-panel" aria-labelledby="login-title">
      <div class="brand-lockup">
        <span class="brand-mark">W</span>
        <span class="brand-name">WineStock</span>
      </div>
      <div>
        <p class="eyebrow">Authentication</p>
        <h1 id="login-title">登录</h1>
      </div>
      <div class="route-placeholder route-placeholder--compact">
        <h2>登录页面路由已建立</h2>
        <p>登录表单、token 管理和错误映射将在 API client 阶段实现。</p>
      </div>
      <RouterLink class="secondary-button" :to="{ name: 'dashboard' }">进入应用壳预览</RouterLink>
    </section>
  </main>
</template>

<script setup lang="ts">
import { useResponsiveShell } from '../composables/useResponsiveShell'
import DesktopLoginPage from './login/DesktopLoginPage.vue'

const { isDesktop } = useResponsiveShell()
</script>
