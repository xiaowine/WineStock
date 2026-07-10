<!--
  本文件拥有注册路由的响应式页面选择，属于 frontend 鉴权页面层。
  桌面端提供首个用户注册流程，移动端仍只保留说明；它不管理平台安全存储。
-->
<template>
  <DesktopRegisterPage v-if="isDesktop" />

  <main v-else class="standalone-page">
    <section class="standalone-panel" aria-labelledby="register-title">
      <div class="brand-lockup">
        <span class="brand-mark">W</span>
        <span class="brand-name">WineStock</span>
      </div>
      <div>
        <p class="eyebrow">Authentication</p>
        <h1 id="register-title">创建首个用户</h1>
      </div>
      <div class="route-placeholder route-placeholder--compact">
        <h2>移动端注册暂未启用</h2>
        <p>请先在桌面端创建首个用户，再使用该账号登录。</p>
      </div>
      <RouterLink class="secondary-button" :to="{ name: 'login' }">返回登录</RouterLink>
    </section>
  </main>
</template>

<script setup lang="ts">
import { useResponsiveShell } from '../composables/useResponsiveShell'
import DesktopRegisterPage from './register/DesktopRegisterPage.vue'

const { isDesktop } = useResponsiveShell()
</script>
