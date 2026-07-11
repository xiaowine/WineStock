<!--
  本文件拥有已登录应用区域的响应式 Shell 选择，属于 frontend 布局层。
  它不判断 Desktop、Android 或 Web 平台能力，也不拥有鉴权状态。
-->
<template>
  <DesktopShell v-if="isDesktop" />
  <MobileShell v-else />
</template>

<script setup lang="ts">
import { useResponsiveShell } from '../composables/useResponsiveShell'
import DesktopShell from './DesktopShell.vue'
import MobileShell from './MobileShell.vue'

const { isDesktop } = useResponsiveShell()
</script>

<style lang="scss" src="./AppShell.scss"></style>
