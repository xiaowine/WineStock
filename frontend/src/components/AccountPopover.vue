<!--
  本文件拥有应用壳共用的紧凑账户弹层，属于 frontend 通用交互组件。
  它按端侧需要展示用户摘要与退出入口，但不读取会话、不执行退出请求，也不管理弹层开关。
-->
<template>
  <section class="account-popover" aria-label="账户信息">
    <AccountUserSummary
      v-if="showUserSummary"
      :initials="initials"
      :display-name="displayName"
    />
    <p v-if="logoutError" class="account-popover__error" role="alert">
      {{ logoutError }}
    </p>
    <button
      class="secondary-button account-popover__logout"
      type="button"
      :disabled="isLoggingOut"
      @click="emit('logout')"
    >
      {{ isLoggingOut ? '正在退出…' : '退出登录' }}
    </button>
  </section>
</template>

<script setup lang="ts">
import AccountUserSummary from './AccountUserSummary.vue'

withDefaults(defineProps<{
  /** 从真实用户名生成的头像缩写。 */
  initials: string
  /** 当前用户展示名称。 */
  displayName: string
  /** 是否在弹层中补充头像和用户名；触发区已显示完整信息时应关闭。 */
  showUserSummary?: boolean
  /** 退出操作的可展示错误；空字符串表示没有错误。 */
  logoutError: string
  /** 是否正在执行统一退出流程。 */
  isLoggingOut: boolean
}>(), {
  showUserSummary: true,
})

const emit = defineEmits<{
  /** 请求所属 Shell 执行统一退出流程。 */
  logout: []
}>()
</script>

<style lang="scss" src="./AccountPopover.scss"></style>
