<!--
  本文件拥有前端根路由出口和全局 Notice 挂载点，属于 frontend。
  它不拥有具体页面布局、业务提示文案、平台 WebView 生命周期或 Axum 资源服务。
-->
<template>
  <RouterView />
  <NoticeViewport />
</template>

<script setup lang="ts">
import NoticeViewport from './components/NoticeViewport.vue'
</script>
