<!--
  本文件拥有应用壳共用的紧凑账户弹层，属于 frontend 通用交互组件。
  它按端侧需要展示用户摘要与账户操作，但不读取会话、运行快照，也不管理弹层开关。
-->
<template>
  <section class="account-popover" aria-label="账户与本机">
    <AccountUserSummary
      v-if="showUserSummary"
      class="account-popover__summary"
      :initials="initials"
      :display-name="displayName"
    />
    <p v-if="logoutError" class="account-popover__error" role="alert">
      {{ logoutError }}
    </p>
    <button
      class="secondary-button account-popover__settings"
      type="button"
      :disabled="isLoggingOut"
      @click="emit('runtimeSettings')"
    >
      本机运行设置
    </button>
    <button
      v-if="showLanAccess"
      class="secondary-button account-popover__lan-access"
      type="button"
      :disabled="isLoggingOut"
      @click="emit('lanAccess')"
    >
      本机局域网地址
    </button>
    <button
      v-if="showLogout"
      class="secondary-button account-popover__logout"
      type="button"
      :disabled="isLoggingOut"
      @click="emit('logout')"
    >
      {{ isLoggingOut ? "正在退出…" : "退出登录" }}
    </button>
  </section>
</template>

<script setup lang="ts">
import AccountUserSummary from "./AccountUserSummary.vue";

withDefaults(
  defineProps<{
    /** 从真实用户名生成的头像缩写。 */
    initials: string;
    /** 当前用户展示名称。 */
    displayName: string;
    /** 是否在弹层中补充头像和用户名；触发区已显示完整信息时应关闭。 */
    showUserSummary?: boolean;
    /** 当前 Shell 是否存在可向其它设备展示的真实局域网地址。 */
    showLanAccess?: boolean;
    /** 本机静默免登录模式下隐藏退出登录（登出后会立即静默重建，无意义）。 */
    showLogout?: boolean;
    /** 退出操作的可展示错误；空字符串表示没有错误。 */
    logoutError: string;
    /** 是否正在执行统一退出流程。 */
    isLoggingOut: boolean;
  }>(),
  {
    showUserSummary: true,
    showLanAccess: false,
    showLogout: true,
  },
);

const emit = defineEmits<{
  /** 打开当前设备独立于业务服务的运行设置。 */
  runtimeSettings: [];
  /** 打开当前设备的局域网连接地址。 */
  lanAccess: [];
  /** 请求所属 Shell 执行统一退出流程。 */
  logout: [];
}>();
</script>

<style lang="scss" src="./AccountPopover.scss"></style>
