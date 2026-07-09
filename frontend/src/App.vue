<!--
  本文件拥有前端应用入口壳层，属于 frontend。
  它只选择桌面/移动布局，不拥有平台 WebView 生命周期或 Axum 资源服务。
-->
<template>
  <DesktopShell v-if="isDesktop" />
  <MobileShell v-else />
</template>

<script setup lang="ts">
import { useResponsiveShell } from './composables/useResponsiveShell'
import DesktopShell from './layouts/DesktopShell.vue'
import MobileShell from './layouts/MobileShell.vue'

const { isDesktop } = useResponsiveShell()
</script>
