<!--
  本文件拥有用户软删除确认内容，属于 frontend 用户管理组件层。
  它不调用删除 API，也不替代后端的操作者保护和防锁死校验。
-->
<template>
  <ModalDialog
    :open="Boolean(user)"
    :title="$t('users.deleteUser')"
    :busy="submitting"
    @close="emit('close')"
  >
    <template #context>
      <div v-if="user" class="dialog-account-context dialog-account-context--danger">
        <span>{{ $t("users.targetUser") }}</span>
        <strong :title="user.username">{{ user.username }}</strong>
      </div>
    </template>

    <div class="dialog-content">
      <p class="confirmation-copy">{{ $t("users.deleteConfirmation") }}</p>
      <p class="form-warning">{{ $t("users.deleteWarning") }}</p>
    </div>

    <template #actions>
      <button class="secondary-button" type="button" :disabled="submitting" @click="emit('close')">
        {{ $t("common.cancel") }}
      </button>
      <button class="danger-button" type="button" :disabled="submitting" @click="emit('submit')">
        {{ submitting ? $t("users.deleting") : $t("users.confirmDelete") }}
      </button>
    </template>
  </ModalDialog>
</template>

<script setup lang="ts">
import type { UserAdminResponse } from "../../api/users";
import ModalDialog from "../ModalDialog.vue";

defineProps<{
  user: UserAdminResponse | null;
  submitting: boolean;
  errorMessage: string;
}>();

const emit = defineEmits<{
  close: [];
  submit: [];
}>();
</script>
