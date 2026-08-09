<!--
  本文件拥有移动端用户管理操作入口，属于 frontend 用户管理组件层。
  它只编排可见操作，不调用业务 API，也不替代后端权限校验。
-->
<template>
  <ModalDialog :open="Boolean(user)" :title="$t('users.manageActions')" @close="emit('close')">
    <template #context>
      <div v-if="user" class="dialog-account-context">
        <span>{{ $t("users.targetUser") }}</span>
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
          <strong>{{ $t("users.updateUsername") }}</strong>
          <small>{{ $t("users.actionUpdateUsernameDetail") }}</small>
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
          <strong>{{ $t("users.permissionSettings") }}</strong>
          <small>{{ $t("users.actionPermissionsDetail") }}</small>
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
          <strong>{{ $t("users.setTemporaryPassword") }}</strong>
          <small>{{ $t("users.actionSetTemporaryPasswordDetail") }}</small>
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
          <strong>{{
            user.status === "active" ? $t("users.disableAccount") : $t("users.enableAccount")
          }}</strong>
          <small>
            {{
              user.status === "active"
                ? $t("users.disableAccountDetail")
                : $t("users.enableAccountDetail")
            }}
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
          <strong>{{ $t("users.deleteUser") }}</strong>
          <small>{{ $t("users.deleteUserDetail") }}</small>
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
