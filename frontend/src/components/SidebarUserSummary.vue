<!--
  本文件拥有侧栏底部的当前用户摘要，属于 frontend 通用展示组件。
  它只显示头像和名称，不拥有交互行为、鉴权状态或用户数据加载。
-->
<template>
  <div class="sidebar-user-summary" aria-label="当前用户">
    <span class="sidebar-user-summary__avatar" aria-hidden="true">{{ initials }}</span>
    <span class="sidebar-user-summary__identity">
      <strong>{{ displayName }}</strong>
      <small>当前用户</small>
    </span>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  /** 当前用户头像占位文字，后续可替换为真实头像。 */
  initials: string
  /** 当前用户展示名称。 */
  displayName: string
}>()
</script>
