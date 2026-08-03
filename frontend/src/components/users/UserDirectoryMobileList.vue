<!--
  本文件拥有用户管理窄桌面和移动端纵向列表的展示和管理入口，属于 frontend 用户管理组件层。
  它复用桌面列表的用户数据，不请求 API，也不拥有操作权限规则。
-->
<template>
  <div class="user-directory-mobile-list">
    <article v-for="user in users" :key="user.id" class="user-directory-mobile-list__item">
      <header class="user-directory-mobile-list__header">
        <div class="user-directory-mobile-list__identity">
          <h2 :title="user.username">{{ user.username }}</h2>
          <p>
            #{{ user.id }}<template v-if="isCurrentUser(user, currentUserId)"> · 当前账号</template>
          </p>
        </div>
        <button
          v-if="hasAvailableAction(user)"
          class="icon-button user-directory-mobile-list__more"
          type="button"
          title="管理用户"
          :aria-label="`管理用户：${user.username}`"
          @click="emit('actions', user)"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <circle cx="5" cy="12" r="1.25" />
            <circle cx="12" cy="12" r="1.25" />
            <circle cx="19" cy="12" r="1.25" />
          </svg>
        </button>
      </header>

      <dl>
        <div>
          <dt>状态</dt>
          <dd>
            <UserStatusPill :status="user.status" />
          </dd>
        </div>
        <div>
          <dt>权限</dt>
          <dd>{{ user.permissions.length }} 项</dd>
        </div>
        <div>
          <dt>密码</dt>
          <dd>
            <UserPasswordStatePill :password-change-required="user.password_change_required" />
          </dd>
        </div>
        <div>
          <dt>最近更新</dt>
          <dd>
            <time :datetime="user.updated_at">{{ formatUserDate(user.updated_at) }}</time>
          </dd>
        </div>
      </dl>
    </article>
  </div>
</template>

<script setup lang="ts">
import type { UserAdminResponse } from "../../api/users";
import UserPasswordStatePill from "./UserPasswordStatePill.vue";
import UserStatusPill from "./UserStatusPill.vue";
import {
  formatUserDate,
  hasAvailableUserAction,
  isCurrentUser,
  type UserDirectoryCapabilities,
} from "./userDirectory";

const props = defineProps<{
  users: UserAdminResponse[];
  currentUserId?: string;
  canUpdateUsername: boolean;
  canEditPermissions: boolean;
  canResetPassword: boolean;
  canUpdateStatus: boolean;
  canDelete: boolean;
}>();

const capabilities: UserDirectoryCapabilities = props;

const emit = defineEmits<{
  actions: [user: UserAdminResponse];
}>();

function hasAvailableAction(user: UserAdminResponse): boolean {
  return hasAvailableUserAction(user, props.currentUserId, capabilities);
}
</script>

<style lang="scss" src="./UserDirectoryMobileList.scss"></style>
