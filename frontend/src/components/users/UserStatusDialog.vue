<!--
  本文件拥有用户启停确认内容，属于 frontend 用户管理组件层。
  它不调用状态更新 API，也不决定防锁死规则。
-->
<template>
  <ModalDialog
    :open="Boolean(user)"
    :title="nextStatus === 'disabled' ? $t('users.disableUser') : $t('users.enableUser')"
    :busy="submitting"
    @close="emit('close')"
  >
    <template #context>
      <div v-if="user" class="dialog-account-context">
        <span>{{ $t("users.targetUser") }}</span>
        <strong :title="user.username">{{ user.username }}</strong>
      </div>
    </template>

    <div class="dialog-content">
      <p class="confirmation-copy">
        {{
          nextStatus === "disabled"
            ? $t("users.disableUserDescription")
            : $t("users.enableUserDescription")
        }}
      </p>
    </div>

    <template #actions>
      <button class="secondary-button" type="button" :disabled="submitting" @click="emit('close')">
        {{ $t("common.cancel") }}
      </button>
      <button class="primary-button" type="button" :disabled="submitting" @click="emit('submit')">
        {{
          submitting
            ? $t("users.saving")
            : nextStatus === "disabled"
              ? $t("users.confirmDisable")
              : $t("users.confirmEnable")
        }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import type { UserAdminResponse, UserStatus } from "../../api/users";
import ModalDialog from "../ModalDialog.vue";

defineProps<{
  user: UserAdminResponse | null;
  nextStatus: UserStatus;
  submitting: boolean;
  errorMessage: string;
}>();

const emit = defineEmits<{
  close: [];
  submit: [];
}>();
</script>
