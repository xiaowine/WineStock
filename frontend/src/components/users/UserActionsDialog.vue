<!--
  本文件拥有移动端用户管理操作入口，属于 frontend 用户管理组件层。
  它只编排可见操作，不调用业务 API，也不替代后端权限校验。
-->
<template>
  <ModalDialog :open="Boolean(user)" title="管理操作" @close="emit('close')">
    <template #context>
      <div v-if="user" class="dialog-account-context">
        <span>目标用户</span>
        <strong :title="user.username">{{ user.username }}</strong>
      </div>
    </template>

    <div v-if="user" class="user-actions-menu">
      <button
        v-if="canUpdateUsername"
        class="user-action-option"
        type="button"
        @click="emit('username')"
      >
        <span>
          <strong>修改用户名</strong>
          <small>修改该账号的登录用户名</small>
        </span>
        <span aria-hidden="true">›</span>
      </button>
      <button
        v-if="canEditPermissions"
        class="user-action-option"
        type="button"
        @click="emit('permissions')"
      >
        <span>
          <strong>权限设置</strong>
          <small>调整页面和操作权限</small>
        </span>
        <span aria-hidden="true">›</span>
      </button>
      <button
        v-if="canResetPassword"
        class="user-action-option"
        type="button"
        @click="emit('password')"
      >
        <span>
          <strong>设置临时密码</strong>
          <small>要求用户下次登录后修改密码</small>
        </span>
        <span aria-hidden="true">›</span>
      </button>
      <button
        v-if="canUpdateStatus"
        class="user-action-option"
        type="button"
        @click="emit('status')"
      >
        <span>
          <strong>{{ user.status === "active" ? "停用账号" : "启用账号" }}</strong>
          <small>
            {{ user.status === "active" ? "结束现有会话并禁止登录" : "允许该用户重新登录" }}
          </small>
        </span>
        <span aria-hidden="true">›</span>
      </button>
      <button
        v-if="canDelete"
        class="user-action-option user-action-option--danger"
        type="button"
        @click="emit('delete')"
      >
        <span>
          <strong>删除用户</strong>
          <small>从用户列表永久移除该账号</small>
        </span>
        <span aria-hidden="true">›</span>
      </button>
    </div>
  </ModalDialog>
</template>

<script setup lang="ts">
import type { UserAdminResponse } from "../../api/users";
import ModalDialog from "../ModalDialog.vue";

defineProps<{
  user: UserAdminResponse | null;
  canUpdateUsername: boolean;
  canEditPermissions: boolean;
  canResetPassword: boolean;
  canUpdateStatus: boolean;
  canDelete: boolean;
}>();

const emit = defineEmits<{
  close: [];
  username: [];
  permissions: [];
  password: [];
  status: [];
  delete: [];
}>();
</script>
