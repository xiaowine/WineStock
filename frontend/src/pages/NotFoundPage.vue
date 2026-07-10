<!--
  本文件拥有未知前端路径的兜底页面，属于 frontend 路由页面层。
  它只处理客户端路由未匹配状态，不替代平台资源加载错误处理。
-->
<template>
  <main class="standalone-page">
    <section class="standalone-panel" aria-labelledby="not-found-title">
      <p class="eyebrow">404</p>
      <h1 id="not-found-title">页面不存在</h1>
      <p class="standalone-description">未找到路径：{{ route.fullPath }}</p>
      <RouterLink class="primary-button" :to="{ name: 'dashboard' }">返回总览</RouterLink>
    </section>
  </main>
</template>

<script setup lang="ts">
import { useRoute } from 'vue-router'

const route = useRoute()
</script>
