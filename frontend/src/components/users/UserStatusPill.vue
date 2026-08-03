<!-- 本文件拥有用户账号状态标签的统一语义和视觉类名，不拥有状态更新行为。 -->
<template>
  <span class="status-pill" :class="userStatusClass(status)">
    {{ userStatusLabel(status) }}
  </span>
</template>

<script setup lang="ts">
import type { UserStatus } from "../../api/users";
import { userStatusClass, userStatusLabel } from "./userDirectory";

defineProps<{
  status: UserStatus;
}>();
</script>
