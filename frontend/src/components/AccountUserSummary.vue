<!--
  本文件拥有应用壳复用的当前用户摘要，属于 frontend 通用展示组件。
  它只显示头像和名称，不拥有账户菜单、鉴权状态或用户数据加载。
-->
<template>
  <div class="account-user-summary" aria-label="当前用户">
    <span class="account-user-summary__avatar" aria-hidden="true">{{ initials }}</span>
    <span class="account-user-summary__identity">
      <strong>{{ displayName }}</strong>
      <small>{{ subtitle }}</small>
    </span>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  /** 从真实用户名生成的头像缩写。 */
  initials: string;
  /** 当前用户展示名称。 */
  displayName: string;
  /** 当前运行模式展示名称。 */
  subtitle: string;
}>();
</script>

<style lang="scss" src="./AccountUserSummary.scss"></style>
