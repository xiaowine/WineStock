<!--
  本文件拥有用户管理桌面列表的列模型和展示事件，属于 frontend 用户管理组件层。
  它不请求 API、不判断后端授权，只把页面提供的用户和可用操作呈现为稳定列。
-->
<template>
  <div v-overlay-scrollbar class="user-directory-table-wrap">
    <div
      class="user-directory-table"
      role="table"
      :aria-label="$t('users.userList')"
      :aria-rowcount="users.length + 1"
    >
      <div class="user-directory-table__header" role="row">
        <span role="columnheader">{{ $t("users.user") }}</span>
        <span role="columnheader">{{ $t("common.status") }}</span>
        <span role="columnheader">{{ $t("users.permissions") }}</span>
        <span role="columnheader">{{ $t("users.password") }}</span>
        <span role="columnheader">{{ $t("users.lastUpdated") }}</span>
        <span role="columnheader">{{ $t("common.actions") }}</span>
      </div>

      <article v-for="user in users" :key="user.id" class="user-directory-table__row" role="row">
        <div class="user-directory-table__identity" role="cell">
          <strong :title="user.username">{{ user.username }}</strong>
          <small
            >#{{ user.id
            }}<template v-if="isCurrentUser(user, currentUserId)"> · {{ $t("users.currentAccount") }}</template></small
          >
        </div>

        <div role="cell">
          <UserStatusPill :status="user.status" />
        </div>

        <div role="cell" class="user-directory-table__permission-count">
          {{ $t("users.permissionCount", { n: user.permissions.length }) }}
        </div>

        <div role="cell">
          <UserPasswordStatePill :password-change-required="user.password_change_required" />
        </div>

        <div role="cell" class="user-directory-table__updated">
          <time :datetime="user.updated_at" :title="formatUserDate(user.updated_at)">
            {{ formatUserDate(user.updated_at) }}
          </time>
        </div>

        <div role="cell" class="user-directory-table__actions">
          <button
            v-if="canUpdateUsername"
            class="text-button"
            type="button"
            @click="emit('username', user)"
          >
            {{ $t("users.username") }}
          </button>
          <button
            v-if="canEditPermissions"
            class="text-button"
            type="button"
            @click="emit('permissions', user)"
          >
            {{ $t("users.permissions") }}
          </button>
          <button
            v-if="canResetPassword && !isCurrentUser(user, currentUserId)"
            class="text-button"
            type="button"
            @click="emit('password', user)"
          >
            {{ $t("users.temporaryPassword") }}
          </button>
          <button
            v-if="canUpdateStatus && !isCurrentUser(user, currentUserId)"
            class="text-button"
            type="button"
            @click="emit('status', user)"
          >
            {{ user.status === "active" ? $t("users.disable") : $t("users.enable") }}
          </button>
          <button
            v-if="canDelete && !isCurrentUser(user, currentUserId)"
            class="text-button"
            type="button"
            @click="emit('delete', user)"
          >
            {{ $t("common.delete") }}
          </button>
          <span
            v-if="
              !canUpdateUsername &&
              !canEditPermissions &&
              !(canResetPassword && !isCurrentUser(user, currentUserId)) &&
              !(canUpdateStatus && !isCurrentUser(user, currentUserId)) &&
              !(canDelete && !isCurrentUser(user, currentUserId))
            "
            class="user-directory-table__empty"
          >
            {{ $t("users.noAvailableActions") }}
          </span>
        </div>
      </article>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { UserAdminResponse } from "../../api/users";
import UserPasswordStatePill from "./UserPasswordStatePill.vue";
import UserStatusPill from "./UserStatusPill.vue";
import { formatUserDate, isCurrentUser } from "./userDirectory";

const props = defineProps<{
  users: UserAdminResponse[];
  currentUserId?: string;
  canUpdateUsername: boolean;
  canEditPermissions: boolean;
  canResetPassword: boolean;
  canUpdateStatus: boolean;
  canDelete: boolean;
}>();

const emit = defineEmits<{
  username: [user: UserAdminResponse];
  permissions: [user: UserAdminResponse];
  password: [user: UserAdminResponse];
  status: [user: UserAdminResponse];
  delete: [user: UserAdminResponse];
}>();
</script>

<style lang="scss" src="./UserDirectoryTable.scss"></style>
