<!-- 本文件拥有用户密码状态标签的统一展示，不拥有密码重置行为。 -->
<template>
  <span
    class="status-pill"
    :class="passwordChangeRequired ? 'status-pill--warn' : 'status-pill--neutral'"
  >
    {{ passwordChangeRequired ? "待修改" : "正常" }}
  </span>
</template>

<script setup lang="ts">
defineProps<{
  passwordChangeRequired: boolean;
}>();
</script>
